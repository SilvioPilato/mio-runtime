---
id: f-single-threaded-loop-run-holds-an-exclusive-borr
type: force
title: 'Single-threaded loop: run() holds an exclusive borrow, no shared-state sync'
status_log:
- status: holds
  since: 2026-07-02
---
The event loop runs on one thread. `run(&mut self)` holds an exclusive borrow of the EventLoop for its entire duration, and the design deliberately avoids Arc<Mutex<...>>-style shared state. Anything a handler needs during dispatch must be passed into the callback; anything an external actor needs must be a dedicated Send handle.