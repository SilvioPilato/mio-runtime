use std::collections::HashSet;

use mio_runtime::{ReadyState, TimerId, Token};

#[test]
fn token_equality() {
    assert_eq!(Token(1), Token(1));
    assert_ne!(Token(1), Token(2));
}

#[test]
fn token_hashable() {
    let mut set = HashSet::new();
    set.insert(Token(7));
    set.insert(Token(7));
    set.insert(Token(8));
    assert_eq!(set.len(), 2);
    assert!(set.contains(&Token(7)));
    assert!(set.contains(&Token(8)));
}

#[test]
fn token_field_is_public() {
    let t = Token(42);
    assert_eq!(t.0, 42);
}

#[test]
fn timer_id_equality() {
    assert_eq!(TimerId(42), TimerId(42));
    assert_ne!(TimerId(42), TimerId(43));
}

#[test]
fn timer_id_hashable() {
    let mut set = HashSet::new();
    set.insert(TimerId(1));
    set.insert(TimerId(1));
    assert_eq!(set.len(), 1);
}

#[test]
fn ready_state_queries() {
    let r = ReadyState::new(true, false);
    assert!(r.readable());
    assert!(!r.writable());

    let w = ReadyState::new(false, true);
    assert!(!w.readable());
    assert!(w.writable());

    let both = ReadyState::new(true, true);
    assert!(both.readable());
    assert!(both.writable());

    let none = ReadyState::new(false, false);
    assert!(!none.readable());
    assert!(!none.writable());
}

#[test]
fn ready_state_equality() {
    assert_eq!(ReadyState::new(true, false), ReadyState::new(true, false));
    assert_ne!(ReadyState::new(true, false), ReadyState::new(false, true));
}
