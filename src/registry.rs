use crate::{TimerId, Token, timerwheel::TimerWheel};
use mio::{Interest, event::Source};
use std::{cell::RefCell, io, time::Duration};

pub struct Registry<'a> {
    inner: &'a mio::Registry,
    wheel: RefCell<&'a mut TimerWheel>,
}

impl<'a> Registry<'a> {
    pub fn new(inner: &'a mio::Registry, wheel: &'a mut TimerWheel) -> Registry<'a> {
        Registry {
            inner,
            wheel: RefCell::new(wheel),
        }
    }

    pub fn register<S: Source>(
        &self,
        source: &mut S,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        self.inner.register(source, mio::Token(token.0), interests)
    }

    pub fn reregister<S: Source>(
        &self,
        source: &mut S,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        self.inner
            .reregister(source, mio::Token(token.0), interests)
    }

    pub fn deregister<S: Source>(&self, source: &mut S) -> io::Result<()> {
        self.inner.deregister(source)
    }

    pub fn insert_timer(&self, delay: Duration) -> TimerId {
        self.wheel.borrow_mut().insert(delay)
    }

    pub fn cancel_timer(&self, id: TimerId) {
        self.wheel.borrow_mut().cancel(id);
    }
}
