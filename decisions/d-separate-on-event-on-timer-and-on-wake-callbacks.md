---
id: d-separate-on-event-on-timer-and-on-wake-callbacks
type: decision
title: Separate on_event, on_timer, and on_wake callbacks on EventHandler
status: accepted
date: 2026-04-30
cites:
- f-i-o-events-timer-expirations-and-wake-ups-are-di
- f-callback-signatures-should-insulate-consumers-fr
relates:
- d-single-threaded-callback-event-loop-not-async-aw
tags:
- api-design
- event-handler
- callbacks
---
The `EventHandler` trait has three focused methods instead of one merged callback: `on_event(registry, token, ReadyState)` for I/O readiness, `on_timer(registry, timer_id)` for timer expirations, and `on_wake(registry)` for cross-thread notifications via `Waker`.

Merging them would require either a reserved `Token` namespace for timers/wakes (two concepts sharing one type) or a tagged enum parameter that adds branching overhead. Separation keeps each callback in one semantic domain: the consumer's `match token` never handles timer pseudo-tokens, and external notifications don't require polling shared state on every iteration.

`on_event` reports readiness via a local `ReadyState { readable, writable }` type rather than re-exporting `mio::Interest`, insulating consumers from mio internals and keeping the callback signature self-documenting.

Alternatives considered: separate `on_readable`/`on_writable` methods (removes the consumer's ability to handle both states in one logical unit, and still doesn't solve the timer problem); a single `on_event` for everything including timers (reserved Token values, conflated namespaces); an internal reserved `TimerId` for wake-ups (same conflation); no wake callback at all (consumer must poll an AtomicBool on every callback, adding overhead to the common case and obscuring intent).

Documented in docs/adr/ADR-001-event-loop-api.md §2–3.