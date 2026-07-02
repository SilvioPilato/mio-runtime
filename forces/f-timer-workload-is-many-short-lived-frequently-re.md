---
id: f-timer-workload-is-many-short-lived-frequently-re
type: force
title: Timer workload is many short-lived, frequently reset timers
status_log:
- status: holds
  since: 2026-07-02
---
The dominant consumers of timers are raft-rs election and heartbeat timers: short delays (tens to hundreds of ms) that are constantly cancelled and re-armed on every message. Insert and cancel are the hot operations and must be effectively free; firing order within a millisecond does not matter.