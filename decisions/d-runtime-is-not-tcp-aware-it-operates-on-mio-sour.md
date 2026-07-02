---
id: d-runtime-is-not-tcp-aware-it-operates-on-mio-sour
type: decision
title: 'Runtime is not TCP-aware: it operates on mio::Source only'
status: accepted
date: 2026-04-30
cites:
- f-rustikv-and-raft-rs-need-a-shared-non-blocking-i
- f-runtime-must-stay-general-and-reusable-beyond-it
relates:
- d-single-threaded-callback-event-loop-not-async-aw
tags:
- architecture
- api-design
- scope
---
The runtime operates on `mio::Source` trait objects (file descriptors), leaving protocol parsing and connection management entirely to the consumer. `TcpStream`/`TcpListener` must never appear in the core runtime modules (`event_loop.rs`, `registry.rs`, `timer.rs`) — TCP belongs in consumer code or integration tests only.

This keeps the runtime general and reusable beyond the two immediate consumers: rustikv and raft-rs each bring their own protocol and connection state machines, so baking TCP into the loop would duplicate or constrain that logic. UDP support and TLS are likewise out of scope (TLS would be an external dependency, justified only when needed).

Documented in docs/adr/ADR-001-event-loop-api.md and enforced as a code-style rule in AGENTS.md.