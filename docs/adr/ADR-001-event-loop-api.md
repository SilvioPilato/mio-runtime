# ADR-001: Event Loop Public API Design

**Status**: Accepted  
**Date**: 2026-04-30  
**Project**: mio-runtime

---

## Context

`mio-runtime` is a single-threaded I/O event loop built on top of `mio`, designed
to serve as the async I/O foundation for two consumers:

- **rustikv**: replacing the current thread-per-connection model with a
  non-blocking event loop, enabling better connection scalability without
  changing the storage engine logic.
- **raft-rs**: managing persistent TCP connections to Raft peers and election/
  heartbeat timers.

The runtime is intentionally **not TCP-aware**: it operates on `mio::Source`
trait objects (file descriptors), leaving protocol parsing and connection
management to the consumer. This keeps the runtime general and reusable beyond
the two immediate consumers.

The core design question is: **what contract does the runtime expose to its
consumers?** Specifically, how does the consumer register interest in events,
receive notifications, and control the loop lifecycle?

---

## Decision

### 1. Registry access from handler callbacks

**Decision**: pass `&Registry` as a parameter to every callback method.

The consumer frequently needs to modify loop registrations during event
handling — for example, accepting a new TCP connection and registering it, or
switching a connection's interest from `READABLE` to `WRITABLE` after buffering
a response. This access must be available *during* the callback, not deferred.

**Alternatives considered**:

- **Command queue** (handler returns `Vec<LoopCommand>`, loop applies after
  dispatch): deferred, predictable execution order, but adds indirection and
  forces the consumer to reason about *when* modifications take effect.
- **Shared handle** (`Arc<Mutex<EventLoopHandle>>`): introduces shared-state
  synchronization that the single-threaded model is designed to avoid.

`&Registry` as a parameter is immediate, explicit, and zero-overhead. The
consumer sees modifications take effect within the same iteration.

---

### 2. Callback structure

**Decision**: `on_event` for I/O events, `on_timer` as a separate method.

```rust
pub trait EventHandler {
    fn on_event(&mut self, registry: &Registry, token: Token, interest: ReadyState);
    fn on_timer(&mut self, registry: &Registry, timer_id: TimerId);
    fn on_wake(&mut self, registry: &Registry);
}
```

I/O events and timer expirations are **conceptually distinct**:

- I/O events originate from file descriptors registered with `mio`. They are
  identified by a `Token` chosen by the consumer at registration time.
- Timer expirations originate from the timer wheel. They are identified by a
  `TimerId` returned by `registry.insert_timer()`.

Merging them into a single callback would require either a reserved `Token`
namespace for timers (a code smell — two different concepts sharing one type)
or a tagged enum parameter that adds branching overhead.

Separating `on_event` and `on_timer` keeps each callback focused on one
semantic domain. The consumer's `match token { ... }` never needs to handle
timer pseudo-tokens.

**Note on `ReadyState`**: rather than re-exporting `mio::Interest` directly,
the API uses a local `ReadyState` type:

```rust
pub struct ReadyState {
    pub readable: bool,
    pub writable: bool,
}
```

This insulates the consumer from `mio` internals and makes callback signatures
self-documenting without requiring the consumer to import `mio`.

**Alternatives considered**:

- **`on_readable` / `on_writable` as separate methods**: the runtime performs
  the readable/writable branching instead of the consumer. Marginally cleaner
  at the call site, but removes the consumer's ability to handle both states in
  a single logical unit (e.g. updating a last-activity timestamp before
  branching). Also does not solve the timer problem — timers would still need
  either a reserved token or a separate method.
- **Single `on_event` for everything including timers**: requires reserved
  `Token` values for timers, conflating two distinct namespaces.

---

### 3. Notification from external threads

**Decision**: `on_wake()` as a dedicated callback on the trait.

The runtime exposes a `Waker` handle (wrapping `mio::Waker`) that external
threads can use to interrupt `poll()`. When the loop wakes due to a `wake()`
call, it invokes `handler.on_wake(registry)`.

```rust
// external thread (e.g. compaction thread in rustikv):
waker.wake()?;

// handler:
fn on_wake(&mut self, registry: &Registry) {
    if self.compaction_done.load(Ordering::Acquire) {
        // react to compaction completion
    }
}
```

A dedicated callback makes the source of the wake-up explicit. The consumer
does not need to poll shared state on every `on_event` or `on_timer` iteration
to detect external notifications.

**Alternatives considered**:

- **Internal `TimerId` for wake-ups**: reuses `on_timer` with a reserved
  `TimerId`. Same code smell as the reserved `Token` problem above — conflates
  two concepts in one namespace.
- **No callback, loop re-iterates silently**: the consumer must poll shared
  state (e.g. `AtomicBool`) on every callback invocation to detect external
  notifications. Adds overhead to the common case and obscures the intent.

---

### 4. Loop lifecycle — stop

**Decision**: `stop()` is exposed only on `EventLoop`, not on `Registry`.

```rust
impl EventLoop {
    pub fn stop(&mut self);
}
```

Centralizing stop control in the owner of the loop follows the principle of
least authority: only the component that constructed and runs the loop can
terminate it. Handler callbacks cannot stop the loop directly.

If a handler needs to signal shutdown, it does so through shared state
(e.g. an `AtomicBool`) that the owner observes and acts on by calling
`event_loop.stop()`.

This design is consistent with how mature runtimes handle shutdown:
- **tokio**: `Runtime::shutdown_timeout()` on the owner only
- **Netty**: `EventLoopGroup.shutdownGracefully()` on the owner only
- **mio**: no stop concept — the caller controls the loop

**Extension path**: adding `stop()` to `Registry` later is a non-breaking
additive change (one method, same internal `AtomicBool`). The reverse —
removing it — would be a breaking change. Starting minimal is therefore the
lower-risk choice.

**Alternatives considered**:

- **`stop()` on `Registry`**: any callback can stop the loop, including
  unexpectedly. Distributed control makes shutdown behavior harder to reason
  about under load.
- **`ControlFlow` enum parameter on callbacks** (calloop pattern): callbacks
  return or mutate a `ControlFlow` value to signal stop. More explicit than
  shared state, but adds a parameter to every callback signature and couples
  the stop mechanism to the callback protocol.

---

## Resulting API surface

```rust
// --- types ---

pub struct Token(pub usize);

pub struct TimerId(u64); // opaque

pub struct ReadyState {
    pub readable: bool,
    pub writable: bool,
}

#[derive(Clone)]
pub struct Waker { /* wraps mio::Waker */ }

impl Waker {
    pub fn wake(&self) -> io::Result<()>;
}

// --- handler contract ---

pub trait EventHandler {
    fn on_event(&mut self, registry: &Registry, token: Token, interest: ReadyState);
    fn on_timer(&mut self, registry: &Registry, timer_id: TimerId);
    fn on_wake(&mut self, registry: &Registry);
}

// --- registry (passed to callbacks, not owned by consumer) ---

pub struct Registry { /* not Clone, not Send */ }

impl Registry {
    pub fn register<S: Source>(&self, source: &mut S, token: Token, interest: Interest) -> io::Result<()>;
    pub fn reregister<S: Source>(&self, source: &mut S, token: Token, interest: Interest) -> io::Result<()>;
    pub fn deregister<S: Source>(&self, source: &mut S) -> io::Result<()>;
    pub fn insert_timer(&self, delay: Duration) -> TimerId;
    pub fn cancel_timer(&self, id: TimerId);
}

// --- event loop (owned by caller) ---

pub struct EventLoop { /* Poll + TimerWheel + running flag */ }

impl EventLoop {
    pub fn new() -> io::Result<Self>;
    pub fn waker(&self) -> Waker;
    pub fn run(&mut self, handler: &mut dyn EventHandler) -> io::Result<()>;
    pub fn stop(&mut self);
}
```

---

## Consequences

**Positive**:
- The trait is minimal (3 methods) and covers all identified use cases for
  both rustikv and raft-rs without over-engineering.
- `Registry` as a parameter eliminates shared state for the common case of
  modifying registrations during event handling.
- Separate `on_timer` and `on_wake` keep each callback semantically focused;
  no reserved token or timer ID namespaces are needed.
- `stop()` on `EventLoop` only keeps shutdown control predictable and auditable.

**Negative / accepted tradeoffs**:
- `ReadyState` in `on_event` means the consumer still branches on
  `readable`/`writable` internally. This is a conscious choice: the runtime
  does not dictate how the consumer structures its per-connection state machine.
- `on_wake` requires the consumer to coordinate with external threads via
  shared state — the callback is a signal, not a data channel. This is
  intentional: data transfer is out of scope for the runtime.
- Single-threaded design means CPU-bound work (e.g. compaction, Raft log
  application) must be offloaded to separate threads. The `Waker` mechanism
  is the bridge back into the loop from those threads.

---

## Out of scope

- Multi-threaded event loop (multiple `EventLoop` instances on multiple
  threads): deferred. The API is designed to not preclude this — `EventLoop`
  is an explicit struct with no global state — but the implementation does not
  pursue it.
- TLS: external dependency, justified when needed, not part of this crate.
- UDP support: not required by rustikv or raft-rs at this time.
- `async`/`await` integration (`Future`/`Waker` in the Rust sense): out of
  scope by design. The runtime uses a callback model, not a polling model.