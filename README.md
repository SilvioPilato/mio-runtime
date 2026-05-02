# mio-runtime

A single-threaded, callback-based I/O event loop built on top of [`mio`](https://docs.rs/mio).
Designed as the async I/O foundation for `rustikv` and `raft-rs`.

The project is educational — every component is hand-rolled to understand the
underlying mechanics. `mio` is the only external dependency.

## Public types

| Type | Purpose |
|------|---------|
| `Token(pub usize)` | Identifier for a registered I/O source. Chosen by the consumer at registration time and handed back unchanged through `on_event`. |
| `TimerId(pub u64)` | Identifier returned by `Registry::insert_timer`, used for cancellation and to identify expirations in `on_timer`. |
| `ReadyState` | Readiness flags (`readable`, `writable`) exposed by `on_event`. Insulates consumers from `mio::Interest`. Constructed via `ReadyState::new(readable, writable)`; queried via `.readable()` / `.writable()`. |
| `TimerWheel` | Single-tier hashed timer wheel. `new(capacity: Duration)` creates a wheel with millisecond-resolution slots over the given range. `insert(delay) -> TimerId`, `cancel(id)`, `advance(now) -> Vec<TimerId>`, `next_deadline() -> Option<Duration>`. Hidden from consumer docs (`#[doc(hidden)]`); exposed for integration tests only — not part of the stable consumer API (ADR-001). |
| `EventHandler` | Trait consumers implement to receive callbacks from the loop: `on_event(&mut self, registry, token, interest)` for I/O readiness, `on_timer(&mut self, registry, timer_id)` for timer expirations, `on_wake(&mut self, registry)` for external wake-ups. `&Registry` is passed to every callback so sources can be registered, reregistered, or timers inserted/cancelled in-band. |
| `Waker` | Cloneable handle to the loop's internal `mio::Waker`. Obtained via `EventLoop::waker()`; multiple clones share the same underlying waker. Call `wake() -> io::Result<()>` from any thread to interrupt a blocking `Poll::poll`. |
| `EventLoop` | Owner of the event loop. `new(capacity: Duration) -> io::Result<Self>` creates a loop with a timer wheel of the given range. `waker() -> Waker` returns a cloneable wake handle. `stop(&mut self)` signals the loop to exit after the current iteration. `run()` is added in the next task. |
