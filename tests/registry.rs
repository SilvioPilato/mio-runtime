use std::time::Duration;

use mio::{Interest, net::TcpListener};
use mio_runtime::{Registry, TimerWheel, Token};

const SLOTS: usize = 512;

fn setup() -> (mio::Poll, TimerWheel) {
    (
        mio::Poll::new().unwrap(),
        TimerWheel::new(Duration::from_millis(SLOTS as u64)),
    )
}

// --- I/O forwarding ---

#[test]
fn register_tcp_listener_succeeds() {
    let (poll, mut wheel) = setup();
    let registry = Registry::new(poll.registry(), &mut wheel);
    let mut listener = TcpListener::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    assert!(
        registry
            .register(&mut listener, Token(0), Interest::READABLE)
            .is_ok()
    );
}

#[test]
fn reregister_changes_interest() {
    let (poll, mut wheel) = setup();
    let registry = Registry::new(poll.registry(), &mut wheel);
    let mut listener = TcpListener::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    registry
        .register(&mut listener, Token(0), Interest::READABLE)
        .unwrap();
    assert!(
        registry
            .reregister(
                &mut listener,
                Token(0),
                Interest::READABLE | Interest::WRITABLE
            )
            .is_ok()
    );
}

#[test]
fn deregister_after_register_succeeds() {
    let (poll, mut wheel) = setup();
    let registry = Registry::new(poll.registry(), &mut wheel);
    let mut listener = TcpListener::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    registry
        .register(&mut listener, Token(0), Interest::READABLE)
        .unwrap();
    assert!(registry.deregister(&mut listener).is_ok());
}

// --- timer round-trip ---

#[test]
fn insert_timer_returns_distinct_ids() {
    let (poll, mut wheel) = setup();
    let registry = Registry::new(poll.registry(), &mut wheel);
    let a = registry.insert_timer(Duration::from_millis(10));
    let b = registry.insert_timer(Duration::from_millis(20));
    assert_ne!(a, b);
}

#[test]
fn cancel_timer_does_not_panic() {
    let (poll, mut wheel) = setup();
    let registry = Registry::new(poll.registry(), &mut wheel);
    let id = registry.insert_timer(Duration::from_millis(10));
    registry.cancel_timer(id);
}

#[test]
fn cancelled_timer_is_not_returned_by_advance() {
    let (poll, mut wheel) = setup();
    let registry = Registry::new(poll.registry(), &mut wheel);
    let id = registry.insert_timer(Duration::from_millis(1));
    registry.cancel_timer(id);
    // Drop the registry borrow before mutating the wheel directly.
    drop(registry);
    let fired = wheel.advance(std::time::Instant::now() + Duration::from_millis(10));
    assert!(fired.is_empty());
}
