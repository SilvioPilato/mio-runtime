use std::time::Duration;

use mio_runtime::{EventHandler, EventLoop, ReadyState, Registry, TimerId, Token, Waker};

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
