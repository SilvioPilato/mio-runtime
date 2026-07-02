---
id: f-i-o-events-timer-expirations-and-wake-ups-are-di
type: force
title: I/O events, timer expirations, and wake-ups are distinct namespaces
status_log:
- status: holds
  since: 2026-07-02
---
I/O events originate from file descriptors registered with mio, identified by a consumer-chosen Token. Timer expirations originate from the timer wheel, identified by a TimerId returned by insert_timer(). Wake-ups originate from external threads via Waker. Forcing these through one identifier type requires reserved values — a code smell where different concepts share one namespace.