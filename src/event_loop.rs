use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::TimerWheel;

// Reserved by the runtime — consumers must not register sources under this token.
const WAKER_TOKEN: mio::Token = mio::Token(usize::MAX);

#[derive(Clone)]
pub struct Waker {
    inner: Arc<mio::Waker>,
}

pub struct EventLoop {
    #[allow(dead_code)]
    poll: mio::Poll,
    #[allow(dead_code)]
    wheel: TimerWheel,
    waker: Waker,
    running: AtomicBool,
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
            running: AtomicBool::new(false),
        })
    }

    pub fn waker(&self) -> Waker {
        self.waker.clone()
    }

    pub fn stop(&mut self) {
        // Same-thread store/load — Relaxed is sufficient, no cross-thread sync needed.
        self.running.store(false, Ordering::Relaxed);
    }
}
