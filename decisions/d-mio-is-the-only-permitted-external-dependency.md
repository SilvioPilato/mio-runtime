---
id: d-mio-is-the-only-permitted-external-dependency
type: decision
title: mio is the only permitted external dependency
status: accepted
date: 2026-05-01
cites:
- f-educational-project-every-component-hand-rolled
- f-cross-platform-readiness-polling-requires-os-spe
relates:
- d-single-threaded-callback-event-loop-not-async-aw
- d-runtime-is-not-tcp-aware-it-operates-on-mio-sour
tags:
- architecture
- dependencies
- scope
---
The crate depends on `mio` and nothing else. No other crate may be added without explicit user approval (enforced as a rule in AGENTS.md). Everything above the polling layer — timer wheel, registry, dispatch loop, handler trait, public types — is hand-rolled.

The line is drawn at readiness polling because that is the one component where hand-rolling teaches nothing useful and costs enormously: a portable wrapper over epoll (Linux), kqueue (BSD/macOS), and IOCP (Windows) is exactly what mio already is, and the project targets development on Windows while its consumers (rustikv, raft-rs) run wherever they run. Everything above that layer is where the educational value lives, so it stays in-tree.

Consequences: TLS, if ever needed, would be a justified new external dependency rather than hand-rolled crypto; utility crates (slab, log, etc.) are rejected by default.