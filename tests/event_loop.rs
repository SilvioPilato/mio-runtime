use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use mio::Interest;
use mio_runtime::{
    EventHandler, EventLoop, ReadyState, Registry, StopHandle, TimerId, Token, Waker,
};

struct NoopHandler;

impl EventHandler for NoopHandler {
    fn on_event(&mut self, _registry: &Registry, _token: Token, _interest: ReadyState) {}
    fn on_timer(&mut self, _registry: &Registry, _timer_id: TimerId) {}
    fn on_wake(&mut self, _registry: &Registry) {}
}

#[test]
fn event_handler_impl_compiles() {
    let _h: &dyn EventHandler = &NoopHandler;
}

#[test]
fn event_loop_new_succeeds() {
    EventLoop::new(Duration::from_millis(512)).unwrap();
}

#[test]
fn waker_is_clone_and_can_wake() {
    let event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
    let waker: Waker = event_loop.waker();
    let waker2 = waker.clone();
    waker.wake().unwrap();
    waker2.wake().unwrap();
}

#[test]
fn stop_can_be_called_multiple_times() {
    let mut event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
    event_loop.stop();
    event_loop.stop();
}

// --- run() integration tests ---

/// A TcpListener registered for READABLE becomes readable when a connection
/// arrives. The handler verifies the token and ReadyState, then stops the loop.
#[test]
fn on_event_fires_with_correct_ready_state() {
    use mio::net::TcpListener;

    let mut event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
    let waker = event_loop.waker();
    let stop = event_loop.stop_handle();

    // Pre-wake so on_wake fires on the first iteration and we can register.
    waker.wake().unwrap();

    // Channel: handler sends the bound address; connector thread receives it.
    let (addr_tx, addr_rx) = mpsc::channel::<std::net::SocketAddr>();
    std::thread::spawn(move || {
        let addr = addr_rx.recv().unwrap();
        std::net::TcpStream::connect(addr).unwrap();
    });

    struct Handler {
        listener: TcpListener,
        registered: bool,
        received: Option<ReadyState>,
        stop: StopHandle,
        addr_tx: Option<mpsc::Sender<std::net::SocketAddr>>,
    }

    impl EventHandler for Handler {
        fn on_event(&mut self, _: &Registry, token: Token, ready_state: ReadyState) {
            assert_eq!(token, Token(0));
            self.received = Some(ready_state);
            self.stop.stop();
        }

        fn on_timer(&mut self, _: &Registry, _: TimerId) {}

        fn on_wake(&mut self, registry: &Registry) {
            if !self.registered {
                let addr = self.listener.local_addr().unwrap();
                registry
                    .register(&mut self.listener, Token(0), Interest::READABLE)
                    .unwrap();
                self.registered = true;
                if let Some(tx) = self.addr_tx.take() {
                    tx.send(addr).unwrap();
                }
            }
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let mut handler = Handler {
        listener,
        registered: false,
        received: None,
        stop,
        addr_tx: Some(addr_tx),
    };

    event_loop.run(&mut handler).unwrap();

    let ready = handler.received.expect("on_event was never called");
    assert!(ready.readable(), "expected readable=true");
}

/// A timer inserted via Registry::insert_timer fires exactly once.
#[test]
fn timer_fires_exactly_once() {
    let mut event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
    let waker = event_loop.waker();
    let stop = event_loop.stop_handle();

    waker.wake().unwrap();

    struct Handler {
        setup_done: bool,
        expected_id: Option<TimerId>,
        fire_count: usize,
        stop: StopHandle,
    }

    impl EventHandler for Handler {
        fn on_event(&mut self, _: &Registry, _: Token, _: ReadyState) {}

        fn on_timer(&mut self, _: &Registry, timer_id: TimerId) {
            assert_eq!(Some(timer_id), self.expected_id, "unexpected timer id");
            self.fire_count += 1;
            self.stop.stop();
        }

        fn on_wake(&mut self, registry: &Registry) {
            if !self.setup_done {
                self.expected_id = Some(registry.insert_timer(Duration::from_millis(10)));
                self.setup_done = true;
            }
        }
    }

    let mut handler = Handler {
        setup_done: false,
        expected_id: None,
        fire_count: 0,
        stop,
    };

    event_loop.run(&mut handler).unwrap();
    assert_eq!(handler.fire_count, 1);
}

/// Multiple concurrent wake() calls before a poll coalesce: on_wake is called
/// exactly once per iteration, not once per wake() invocation.
#[test]
fn multiple_wakes_coalesce_into_one_on_wake() {
    let mut event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
    let waker = event_loop.waker();
    let stop = event_loop.stop_handle();

    // Three wakes queued before the loop even starts.
    waker.wake().unwrap();
    waker.wake().unwrap();
    waker.wake().unwrap();

    struct Handler {
        wake_count: usize,
        stop: StopHandle,
    }

    impl EventHandler for Handler {
        fn on_event(&mut self, _: &Registry, _: Token, _: ReadyState) {}
        fn on_timer(&mut self, _: &Registry, _: TimerId) {}
        fn on_wake(&mut self, _: &Registry) {
            self.wake_count += 1;
            self.stop.stop();
        }
    }

    let mut handler = Handler {
        wake_count: 0,
        stop,
    };
    event_loop.run(&mut handler).unwrap();
    assert_eq!(
        handler.wake_count, 1,
        "three pre-queued wakes must collapse to one on_wake"
    );
}

/// A timer that is cancelled before its slot is advanced never appears in on_timer.
#[test]
fn cancelled_timer_does_not_fire() {
    let mut event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
    let waker = event_loop.waker();
    let stop = event_loop.stop_handle();

    waker.wake().unwrap();

    struct Handler {
        setup_done: bool,
        cancelled_id: Option<TimerId>,
        cancelled_fired: bool,
        stop: StopHandle,
    }

    impl EventHandler for Handler {
        fn on_event(&mut self, _: &Registry, _: Token, _: ReadyState) {}

        fn on_timer(&mut self, _: &Registry, timer_id: TimerId) {
            if Some(timer_id) == self.cancelled_id {
                self.cancelled_fired = true;
            }
            // Stop on the first timer that fires (should be the watchdog only).
            self.stop.stop();
        }

        fn on_wake(&mut self, registry: &Registry) {
            if !self.setup_done {
                let id = registry.insert_timer(Duration::from_millis(10));
                registry.cancel_timer(id);
                self.cancelled_id = Some(id);
                registry.insert_timer(Duration::from_millis(50)); // watchdog
                self.setup_done = true;
            }
        }
    }

    let mut handler = Handler {
        setup_done: false,
        cancelled_id: None,
        cancelled_fired: false,
        stop,
    };

    event_loop.run(&mut handler).unwrap();
    assert!(
        !handler.cancelled_fired,
        "cancelled timer must never reach on_timer"
    );
}

/// A callback that flips an AtomicBool and calls StopHandle::stop() causes
/// run() to return cleanly at the next iteration boundary.
#[test]
fn stop_from_callback_terminates_loop_cleanly() {
    let mut event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
    let waker = event_loop.waker();
    let stop = event_loop.stop_handle();

    waker.wake().unwrap();

    let callback_ran = Arc::new(AtomicBool::new(false));
    let callback_ran2 = Arc::clone(&callback_ran);

    struct Handler {
        stop: StopHandle,
        callback_ran: Arc<AtomicBool>,
    }

    impl EventHandler for Handler {
        fn on_event(&mut self, _: &Registry, _: Token, _: ReadyState) {}
        fn on_timer(&mut self, _: &Registry, _: TimerId) {}
        fn on_wake(&mut self, _: &Registry) {
            self.callback_ran.store(true, Ordering::Release);
            self.stop.stop();
        }
    }

    let mut handler = Handler {
        stop,
        callback_ran: callback_ran2,
    };
    event_loop.run(&mut handler).unwrap();
    assert!(
        callback_ran.load(Ordering::Acquire),
        "callback must have run before run() returned"
    );
}

/// StopHandle is Send — an external thread can stop the loop independently of
/// any callback, using only the handle and the waker to interrupt poll().
#[test]
fn stop_handle_from_external_thread_terminates_loop() {
    let mut event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
    let waker = event_loop.waker();
    let stop = event_loop.stop_handle();

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        stop.stop();
        waker.wake().unwrap();
    });

    event_loop.run(&mut NoopHandler).unwrap();
}
