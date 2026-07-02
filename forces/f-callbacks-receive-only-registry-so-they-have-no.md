---
id: f-callbacks-receive-only-registry-so-they-have-no
type: force
title: Callbacks receive only &Registry, so they have no path to EventLoop
status_log:
- status: holds
  since: 2026-07-02
---
Handler callbacks are invoked with &Registry, not with the EventLoop itself. Any callback that needs to signal shutdown (or reach loop lifecycle controls) has no reference through which to do it — lifecycle control from inside a callback requires a separate handle obtained before run() starts.