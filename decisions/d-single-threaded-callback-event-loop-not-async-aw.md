---
id: d-single-threaded-callback-event-loop-not-async-aw
type: decision
title: Single-threaded callback event loop, not async/await
status: accepted
date: 2026-04-30
cites:
- f-educational-project-every-component-hand-rolled
- f-rustikv-and-raft-rs-need-a-shared-non-blocking-i
tags:
- architecture
- event-loop
- api-design
---
The runtime is a single-threaded I/O event loop exposing a callback-based `EventHandler` trait. It deliberately does not implement `Future`, `Waker` (in the Rust async sense), thread pools, or work-stealing — a callback model, not a polling model.

CPU-bound work (compaction, Raft log application) must be offloaded to separate threads; the `Waker` mechanism is the bridge back into the loop.

Alternatives considered: async/await integration was ruled out of scope by design — it would hide the mechanics the project exists to expose, and the two consumers do not need it. Multi-threaded loops (tokio/Netty style) would introduce shared-state synchronization the design avoids; the API does not preclude running multiple `EventLoop` instances later, but the implementation does not pursue it.

Documented in docs/adr/ADR-001-event-loop-api.md.