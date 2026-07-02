---
id: d-timers-via-a-single-level-hashed-timer-wheel-wit
type: decision
title: Timers via a single-level hashed timer wheel with 1ms slots and lazy cancellation
status: accepted
date: 2026-05-02
cites:
- f-educational-project-every-component-hand-rolled
- f-timer-workload-is-many-short-lived-frequently-re
relates:
- d-single-threaded-callback-event-loop-not-async-aw
tags:
- architecture
- timers
- data-structures
---
Timers are implemented as a hand-rolled single-level hashed timer wheel (`src/timerwheel.rs`): one slot per millisecond of capacity (`EventLoop::new(capacity: Duration)` sizes the wheel; 512 slots is the typical configuration). `insert(delay)` is an O(1) push into slot `(cursor + delay_ms) % slots`. Cancellation is lazy: `cancel(id)` adds the `TimerId` to a `deleted` set, and cancelled timers are filtered out when their slot is drained or scanned — no O(n) removal from slot vectors.

`next_deadline()` scans forward from the cursor to give the poll timeout; `advance(now)` drains every slot from the cursor through the landing slot inclusive (a timer at delay D fires once elapsed >= D — the inclusive endpoint was the subject of the off-by-one fix in PR #7).

There is no hierarchical/cascading overflow level: a delay beyond the wheel's capacity is a programming error (asserted). Consumers pick a capacity that covers their longest timer.

Alternatives like a `BinaryHeap` of deadlines (O(log n) insert, ordered pop) or hierarchical wheels (unbounded delays) were not taken: the workload is many short timers, and a flat wheel is the simplest structure that makes insert and cancel effectively free.