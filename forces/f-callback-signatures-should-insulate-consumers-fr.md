---
id: f-callback-signatures-should-insulate-consumers-fr
type: force
title: Callback signatures should insulate consumers from mio internals
status_log:
- status: holds
  since: 2026-07-02
---
Consumers should be able to write handlers without importing mio types. Self-documenting local types (like ReadyState with explicit readable/writable fields) keep the public contract independent of the underlying polling library.