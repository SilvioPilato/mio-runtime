//! Public scalar types for the runtime API surface.

/// Identifier for a registered I/O source.
///
/// Chosen by the consumer at registration time and opaque to the runtime —
/// the runtime hands the value back through `on_event` unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token(pub usize);

/// Identifier returned by `Registry::insert_timer`.
///
/// Used both for cancellation and to identify expirations in `on_timer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(pub u64);

/// Readiness flags for an I/O event, exposed by `on_event`.
///
/// Insulates consumers from `mio::Interest` so callback signatures don't
/// require importing `mio`, and leaves room to evolve readiness semantics
/// independently of `mio`'s `Interest` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyState {
    readable: bool,
    writable: bool,
}

impl ReadyState {
    pub fn new(readable: bool, writable: bool) -> Self {
        Self { readable, writable }
    }

    pub fn readable(&self) -> bool {
        self.readable
    }

    pub fn writable(&self) -> bool {
        self.writable
    }
}
