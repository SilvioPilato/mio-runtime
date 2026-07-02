---
id: f-rustikv-and-raft-rs-need-a-shared-non-blocking-i
type: force
title: rustikv and raft-rs need a shared non-blocking I/O foundation
status_log:
- status: holds
  since: 2026-07-02
---
Two consumers drive the design: rustikv wants to replace its thread-per-connection model with a non-blocking event loop for connection scalability without touching storage-engine logic; raft-rs needs persistent TCP connections to Raft peers plus election/heartbeat timers.