---
id: f-cross-platform-readiness-polling-requires-os-spe
type: force
title: Cross-platform readiness polling requires OS-specific APIs
status_log:
- status: holds
  since: 2026-07-02
---
Non-blocking readiness notification is epoll on Linux, kqueue on BSD/macOS, and IOCP on Windows — three divergent OS interfaces with subtle semantics. Hand-rolling a portable abstraction over them is a large, low-insight effort; mio exists precisely to provide this one layer.