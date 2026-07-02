---
id: d-pass-registry-as-a-parameter-to-every-handler-ca
type: decision
title: Pass &Registry as a parameter to every handler callback
status: accepted
date: 2026-04-30
cites:
- f-handlers-must-modify-loop-registrations-during-e
- f-single-threaded-loop-run-holds-an-exclusive-borr
relates:
- d-single-threaded-callback-event-loop-not-async-aw
tags:
- api-design
- event-loop
- registry
---
Every `EventHandler` callback (`on_event`, `on_timer`, `on_wake`) receives `&Registry` as a parameter. `Registry` wraps `mio::Registry` plus a reference to the timer wheel, and is not `Clone` and not `Send` — it is lent to the handler for the duration of the callback, never owned by the consumer.

This gives the handler immediate, explicit, zero-overhead access to `register`/`reregister`/`deregister`/`insert_timer`/`cancel_timer` during dispatch; modifications take effect within the same iteration.

Alternatives considered: a command queue (handler returns `Vec<LoopCommand>`, applied after dispatch) is deferred and predictable but adds indirection and forces the consumer to reason about when modifications take effect; a shared handle (`Arc<Mutex<EventLoopHandle>>`) introduces exactly the shared-state synchronization the single-threaded model is designed to avoid.

Documented in docs/adr/ADR-001-event-loop-api.md §1.