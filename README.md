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
| `StopHandle` | `Clone + Send` stop token obtained via `EventLoop::stop_handle()`. Backed by the same `Arc<AtomicBool>` that `run()` checks at each iteration boundary. Move a clone into a handler callback or an external thread and call `stop_handle.stop()` to terminate the loop after the current iteration completes. |
| `EventLoop` | Owner of the event loop. `new(capacity: Duration) -> io::Result<Self>` creates a loop with a timer wheel of the given range. `waker() -> Waker` returns a cloneable wake handle. `stop_handle() -> StopHandle` returns a shareable stop token for use in callbacks and external threads. `stop(&mut self)` stops the loop directly when called by the owner. `run(&mut self, handler) -> io::Result<()>` runs the dispatch loop until `stop()` or a `StopHandle::stop()` call signals shutdown. |

## Usage example

The snippet below shows the minimal pattern: implement `EventHandler`, obtain a
`StopHandle` before `run`, and signal shutdown from inside a callback.

```rust
use std::time::Duration;
use mio_runtime::{EventHandler, EventLoop, ReadyState, Registry, StopHandle, TimerId, Token};

struct MyHandler {
    stop: StopHandle,
    ticks: u32,
}

impl EventHandler for MyHandler {
    fn on_event(&mut self, _registry: &Registry, _token: Token, _ready: ReadyState) {}

    fn on_timer(&mut self, registry: &Registry, _id: TimerId) {
        self.ticks += 1;
        println!("tick {}", self.ticks);

        if self.ticks >= 3 {
            // Stop the loop; it will exit after this iteration.
            self.stop.stop();
        } else {
            // Re-arm the timer for the next tick.
            registry.insert_timer(Duration::from_millis(500)).unwrap();
        }
    }

    fn on_wake(&mut self, _registry: &Registry) {}
}

fn main() -> std::io::Result<()> {
    // Create an event loop with a 1-second timer wheel capacity.
    let mut event_loop = EventLoop::new(Duration::from_secs(1))?;

    // Obtain a stop handle before handing `event_loop` to `run`.
    let stop = event_loop.stop_handle();

    // Register an initial timer so the loop has something to fire.
    // Timers must be registered through the Registry exposed in callbacks;
    // for the first timer, use the waker to trigger on_wake and register there,
    // or schedule via a Registry obtained outside the loop if available.
    // Here we demonstrate the pattern inside on_timer after the first wake.
    let mut handler = MyHandler { stop, ticks: 0 };

    // Wake the loop immediately so on_wake can arm the first timer.
    let waker = event_loop.waker();
    waker.wake()?;

    event_loop.run(&mut handler)
}
```

### Key points

- **`StopHandle`** — obtain before `run` (which holds `&mut EventLoop`); share
  it by cloning. `stop()` is safe to call from any thread.
- **`Waker`** — use `EventLoop::waker()` to interrupt a blocking poll from
  another thread or from within a callback.
- **Timers** — call `registry.insert_timer(delay)` inside any callback to
  schedule a one-shot timer; `registry.cancel_timer(id)` to cancel one.
- **I/O sources** — call `registry.register(source, token, interest)` to
  receive `on_event` callbacks when the source becomes ready.
