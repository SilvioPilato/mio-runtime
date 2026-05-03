use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::{EventHandler, ReadyState, Registry, TimerWheel, Token};

// Reserved by the runtime — consumers must not register sources under this token.
const WAKER_TOKEN: mio::Token = mio::Token(usize::MAX);
const POLL_EVENTS_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct Waker {
    inner: Arc<mio::Waker>,
}

/// A `Clone + Send` handle that stops the event loop from a callback or an
/// external thread.
///
/// Obtained via [`EventLoop::stop_handle`]. Internally shares the same
/// `Arc<AtomicBool>` that [`EventLoop::run`] checks at each iteration
/// boundary, so calling [`stop`](StopHandle::stop) on any clone causes the
/// loop to exit cleanly after the current iteration completes — timers and
/// I/O events already dispatched in that iteration are not interrupted.
///
/// # Why this exists
///
/// [`EventLoop::run`] holds `&mut self` for its entire duration, making it
/// impossible for the owner to call [`EventLoop::stop`] on the same thread
/// while the loop is running. `StopHandle` is the escape hatch: obtain one
/// before calling `run`, move it into the handler or spawn it onto another
/// thread, and call `stop()` whenever shutdown is appropriate.
///
/// # Example
///
/// ```rust,no_run
/// # use std::time::Duration;
/// # use mio_runtime::{EventHandler, EventLoop, ReadyState, Registry, StopHandle, TimerId, Token};
/// struct MyHandler { stop: StopHandle }
///
/// impl EventHandler for MyHandler {
///     fn on_wake(&mut self, _: &Registry) {
///         self.stop.stop(); // loop exits after this iteration
///     }
///     fn on_event(&mut self, _: &Registry, _: Token, _: ReadyState) {}
///     fn on_timer(&mut self, _: &Registry, _: TimerId) {}
/// }
///
/// let mut event_loop = EventLoop::new(Duration::from_millis(512)).unwrap();
/// let stop = event_loop.stop_handle();
/// event_loop.run(&mut MyHandler { stop }).unwrap();
/// ```
#[derive(Clone)]
pub struct StopHandle {
    flag: Arc<AtomicBool>,
}

impl StopHandle {
    /// Signals the event loop to exit after the current iteration completes.
    pub fn stop(&self) {
        self.flag.store(false, Ordering::Relaxed);
    }
}

pub struct EventLoop {
    poll: mio::Poll,
    wheel: TimerWheel,
    waker: Waker,
    running: Arc<AtomicBool>,
}

impl Waker {
    pub(crate) fn new(inner: Arc<mio::Waker>) -> Self {
        Self { inner }
    }

    pub fn wake(&self) -> io::Result<()> {
        self.inner.wake()
    }
}

impl EventLoop {
    pub fn new(capacity: Duration) -> io::Result<Self> {
        let poll = mio::Poll::new()?;
        let mio_waker = mio::Waker::new(poll.registry(), WAKER_TOKEN)?;
        let waker = Waker::new(Arc::new(mio_waker));
        Ok(EventLoop {
            poll,
            wheel: TimerWheel::new(capacity),
            waker,
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn waker(&self) -> Waker {
        self.waker.clone()
    }

    /// Returns a [`StopHandle`] that shares this loop's stop flag.
    ///
    /// Call this before [`run`](Self::run) and move the handle into the handler
    /// or an external thread. See [`StopHandle`] for usage and rationale.
    pub fn stop_handle(&self) -> StopHandle {
        StopHandle {
            flag: Arc::clone(&self.running),
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn run(&mut self, handler: &mut dyn EventHandler) -> io::Result<()> {
        let mut events = mio::Events::with_capacity(POLL_EVENTS_CAPACITY);
        self.running.store(true, Ordering::Relaxed);

        loop {
            let next_deadline = match self.wheel.next_deadline() {
                Some(timeout) => timeout,
                None => self.wheel.capacity(),
            };
            match self.poll.poll(&mut events, Some(next_deadline)) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
            let timers = self.wheel.advance(Instant::now());
            let registry = Registry::new(self.poll.registry(), &mut self.wheel);

            for timer_id in timers {
                handler.on_timer(&registry, timer_id);
            }

            // Deduplicate the waker token: multiple wake() calls may produce
            // multiple events on some platforms (e.g. IOCP on Windows), but
            // the contract is at most one on_wake per iteration.
            let mut woke = false;
            for event in events.iter() {
                match event.token() {
                    WAKER_TOKEN => woke = true,
                    token => handler.on_event(
                        &registry,
                        Token(token.0),
                        ReadyState::new(event.is_readable(), event.is_writable()),
                    ),
                }
            }
            if woke {
                handler.on_wake(&registry);
            }

            if !self.running.load(Ordering::Relaxed) {
                return Ok(());
            }
        }
    }
}
