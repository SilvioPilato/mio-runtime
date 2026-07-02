---
id: d-dual-stop-mechanism-eventloop-stop-for-the-owner
type: decision
title: 'Dual stop mechanism: EventLoop::stop for the owner, StopHandle for callbacks and threads'
status: accepted
date: 2026-05-03
cites:
- f-single-threaded-loop-run-holds-an-exclusive-borr
- f-callbacks-receive-only-registry-so-they-have-no
relates:
- d-single-threaded-callback-event-loop-not-async-aw
- d-pass-registry-as-a-parameter-to-every-handler-ca
tags:
- api-design
- event-loop
- lifecycle
---
Two complementary stop mechanisms: `EventLoop::stop(&mut self)` for the owner (callable from outside `run()`, e.g. test teardown or a thread that owns the loop), and `StopHandle` — a `Clone + Send` handle wrapping the same `Arc<AtomicBool>` that `run()` checks at each iteration boundary — obtained via `EventLoop::stop_handle()` for use inside callbacks or from external threads. The loop exits cleanly after the current iteration completes.

`Relaxed` ordering is sufficient for the flag: it is read only at iteration boundaries on a single thread, and the callback's return establishes the happens-before relationship before the check.

Precedent in other single-threaded loops: calloop's `LoopSignal` (the direct model for StopHandle), libuv's `uv_stop(loop)` callable from any callback. Owner-only stop works in tokio/Netty only because those are multi-threaded runtimes.

Alternatives considered: owner-only stop (the original design) is unworkable single-threaded — the owner is blocked inside `run()` and cannot act until something else stops the loop, a bootstrapping problem; `stop()` on `Registry` lets any callback stop the loop and makes shutdown harder to audit; a `ControlFlow` return value on every callback (winit pattern) couples the stop mechanism to the callback protocol.

Documented in docs/adr/ADR-001-event-loop-api.md §4; implemented in PR #5.