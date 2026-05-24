//! End-to-end timer timing regression tests for the running `EventLoop`.
//!
//! Background (issue #6): the `TimerWheel` off-by-one fixed in PR #7 was paired
//! with a *suspected* "poll-timeout interaction" — a fear that a timer armed for
//! delay `D` could fire far later than `D` (or never) once real I/O readiness
//! events started waking `poll()` early. Structured debugging ruled that out at
//! the runtime layer: `run()` calls `advance()` unconditionally every iteration
//! and dispatches timers before I/O handlers, and `advance()` preserves the
//! sub-millisecond remainder, so early/continuous wakeups do not starve or drift
//! a timer. These tests lock that conclusion in.
//!
//! The upper bound is deliberately generous (well above OS timer granularity and
//! CI noise) — its job is to catch a *regression* of the form "fires at the wheel
//! capacity / never fires", not to assert millisecond precision.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use mio::{Interest, net::TcpListener};
use mio_runtime::{EventHandler, EventLoop, ReadyState, Registry, StopHandle, TimerId, Token};

/// Wheel capacity / max poll timeout. A regression that fell back to this value
/// (the `next_deadline().unwrap_or(capacity)` path) would blow the 250ms bound.
const CAPACITY: Duration = Duration::from_millis(512);
const TIMER_DELAY: Duration = Duration::from_millis(100);
/// Generous upper bound: ~2.4x the delay, below the 512ms capacity fallback.
const MAX_FIRE_DELAY: Duration = Duration::from_millis(250);
/// Hard stop so a "never fires" regression fails fast instead of hanging.
const WATCHDOG: Duration = Duration::from_millis(800);

/// Spawn a watchdog that force-stops the loop after `WATCHDOG`, so a timer that
/// never fires surfaces as a failed assertion rather than a hung test.
fn spawn_watchdog(stop: StopHandle, waker: mio_runtime::Waker) {
    std::thread::spawn(move || {
        std::thread::sleep(WATCHDOG);
        stop.stop();
        let _ = waker.wake();
    });
}

/// A 100ms timer fires within tolerance with no I/O sources registered.
#[test]
fn timer_fires_on_schedule_without_io() {
    let mut event_loop = EventLoop::new(CAPACITY).unwrap();
    event_loop.waker().wake().unwrap();
    spawn_watchdog(event_loop.stop_handle(), event_loop.waker());

    struct H {
        armed: bool,
        arm_time: Option<Instant>,
        fire_delay: Option<Duration>,
        stop: StopHandle,
    }
    impl EventHandler for H {
        fn on_event(&mut self, _: &Registry, _: Token, _: ReadyState) {}
        fn on_timer(&mut self, _: &Registry, _: TimerId) {
            self.fire_delay = self.arm_time.map(|t| t.elapsed());
            self.stop.stop();
        }
        fn on_wake(&mut self, registry: &Registry) {
            if !self.armed {
                self.arm_time = Some(Instant::now());
                registry.insert_timer(TIMER_DELAY);
                self.armed = true;
            }
        }
    }

    let mut h = H {
        armed: false,
        arm_time: None,
        fire_delay: None,
        stop: event_loop.stop_handle(),
    };
    event_loop.run(&mut h).unwrap();

    let delay = h
        .fire_delay
        .expect("timer never fired within the watchdog window");
    assert!(
        delay < MAX_FIRE_DELAY,
        "timer fired too late: {delay:?} (>= {MAX_FIRE_DELAY:?})"
    );
}

/// A 100ms timer fires within tolerance even while a real socket floods the loop
/// with continuous readiness events — the exact condition the downstream report
/// blamed. The I/O must not starve or defer the timer.
#[test]
fn timer_fires_on_schedule_under_continuous_io() {
    let mut event_loop = EventLoop::new(CAPACITY).unwrap();
    event_loop.waker().wake().unwrap();
    spawn_watchdog(event_loop.stop_handle(), event_loop.waker());

    let listener = TcpListener::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let addr = listener.local_addr().unwrap();

    // Peer thread: connect and write continuously to keep the stream readable.
    let keep_going = Arc::new(AtomicBool::new(true));
    let keep_going2 = Arc::clone(&keep_going);
    let peer = std::thread::spawn(move || {
        use std::io::Write;
        if let Ok(mut s) = std::net::TcpStream::connect(addr) {
            // Small writes paced at ~1ms keep the stream continuously readable
            // without saturating the send buffer, so `keep_going` is re-checked
            // every iteration and shutdown is prompt.
            while keep_going2.load(Ordering::Relaxed) {
                if s.write_all(b"xxxxxxxx").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    });

    struct H {
        listener: TcpListener,
        registered: bool,
        accepted: Option<mio::net::TcpStream>,
        armed: bool,
        arm_time: Option<Instant>,
        fire_delay: Option<Duration>,
        stop: StopHandle,
    }
    impl EventHandler for H {
        fn on_event(&mut self, registry: &Registry, token: Token, _: ReadyState) {
            match token {
                Token(0) => {
                    if let Ok((mut stream, _)) = self.listener.accept() {
                        registry
                            .register(&mut stream, Token(1), Interest::READABLE)
                            .unwrap();
                        self.accepted = Some(stream);
                    }
                }
                Token(1) => {
                    use std::io::Read;
                    if let Some(s) = self.accepted.as_mut() {
                        let mut buf = [0u8; 4096];
                        let _ = s.read(&mut buf);
                    }
                }
                _ => {}
            }
        }
        fn on_timer(&mut self, _: &Registry, _: TimerId) {
            self.fire_delay = self.arm_time.map(|t| t.elapsed());
            self.stop.stop();
        }
        fn on_wake(&mut self, registry: &Registry) {
            if !self.registered {
                registry
                    .register(&mut self.listener, Token(0), Interest::READABLE)
                    .unwrap();
                self.registered = true;
            }
            if !self.armed {
                self.arm_time = Some(Instant::now());
                registry.insert_timer(TIMER_DELAY);
                self.armed = true;
            }
        }
    }

    let mut h = H {
        listener,
        registered: false,
        accepted: None,
        armed: false,
        arm_time: None,
        fire_delay: None,
        stop: event_loop.stop_handle(),
    };
    event_loop.run(&mut h).unwrap();
    keep_going.store(false, Ordering::Relaxed);
    // Close the server side so any in-flight peer write returns immediately.
    drop(h.accepted.take());
    let _ = peer.join();

    let delay = h
        .fire_delay
        .expect("timer never fired under continuous I/O");
    assert!(
        delay < MAX_FIRE_DELAY,
        "timer fired too late under I/O load: {delay:?} (>= {MAX_FIRE_DELAY:?})"
    );
}

/// Re-arming the timer from inside `on_timer` keeps it firing on schedule rather
/// than drifting toward "never" — guards against cumulative cursor lag.
#[test]
fn rearmed_timer_keeps_firing_on_schedule() {
    const PERIOD: Duration = Duration::from_millis(50);
    const RUN_FOR: Duration = Duration::from_millis(600);
    // At 50ms cadence over ~600ms we expect ~11 fires; require a clear majority
    // so a regression that fired once then stalled fails.
    const MIN_FIRES: usize = 6;

    let mut event_loop = EventLoop::new(CAPACITY).unwrap();
    event_loop.waker().wake().unwrap();

    // Watchdog tuned to RUN_FOR for this periodic test.
    let stop = event_loop.stop_handle();
    let waker = event_loop.waker();
    std::thread::spawn(move || {
        std::thread::sleep(RUN_FOR);
        stop.stop();
        let _ = waker.wake();
    });

    struct H {
        armed: bool,
        fires: usize,
    }
    impl EventHandler for H {
        fn on_event(&mut self, _: &Registry, _: Token, _: ReadyState) {}
        fn on_timer(&mut self, registry: &Registry, _: TimerId) {
            self.fires += 1;
            registry.insert_timer(PERIOD); // re-arm
        }
        fn on_wake(&mut self, registry: &Registry) {
            if !self.armed {
                registry.insert_timer(PERIOD);
                self.armed = true;
            }
        }
    }

    let mut h = H {
        armed: false,
        fires: 0,
    };
    event_loop.run(&mut h).unwrap();

    assert!(
        h.fires >= MIN_FIRES,
        "re-armed timer stalled: only {} fires in {:?} (expected >= {})",
        h.fires,
        RUN_FOR,
        MIN_FIRES
    );
}
