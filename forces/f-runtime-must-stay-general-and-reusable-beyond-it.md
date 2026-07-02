---
id: f-runtime-must-stay-general-and-reusable-beyond-it
type: force
title: Runtime must stay general and reusable beyond its immediate consumers
status_log:
- status: holds
  since: 2026-07-02
---
Each consumer brings its own protocol and connection-management logic. A runtime that bakes in transport-level assumptions (TCP accept loops, stream types) would constrain consumers and duplicate their state machines. The looser the coupling, the wider the reuse.