---
id: f-handlers-must-modify-loop-registrations-during-e
type: force
title: Handlers must modify loop registrations during event dispatch
status_log:
- status: holds
  since: 2026-07-02
---
Consumers frequently need to change registrations while handling an event — e.g. accepting a new TCP connection and registering it, or switching a connection's interest from READABLE to WRITABLE after buffering a response. This access must be available during the callback, not deferred.