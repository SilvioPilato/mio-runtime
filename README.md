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
| `TimeWheel` | Single-tier hashed timer wheel: 512 slots × 1 ms tick. `insert(delay) -> TimerId`, `cancel(id)`, `advance(now) -> Vec<TimerId>`, `next_deadline() -> Option<Duration>`. Exposed publicly to enable integration tests; not part of the stable consumer API (ADR-001). |

More public surface (`Registry`, `EventLoop`, `EventHandler`, `Waker`) is added
in subsequent tasks — see `TASKS.md`.
