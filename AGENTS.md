# Agent Instructions

This is a single-threaded I/O event loop built in Rust, designed as the async
I/O foundation for `rustikv` and `raft-rs`. It wraps `mio` (the only external
dependency) and exposes a callback-based `EventHandler` trait. The project is
educational — every component is hand-rolled to understand the underlying
mechanics. Simplicity and clarity matter more than production-readiness.

The public API design is documented in `docs/adr/ADR-001-event-loop-api.md`.
Read it before starting any task.

## Task Workflow

- Tasks live in `TASKS.md` at the repo root, split into **In Progress**,
  **Open Tasks**, and **Closed Tasks**.
- Before starting work, read `TASKS.md` and identify the relevant task number.
- When starting a task, move its `## #N` section from **Open Tasks** to
  **In Progress**.
- When a task is done, move its entire `## #N` section from **In Progress** to
  **Closed Tasks** — never delete it.
- New tasks get the next sequential `#N` number.

## Git & PRs

- **Never push directly to `main`** — always use a dedicated branch and open a PR.
- Use **git** and the **gh CLI** for all version control and PR operations.
- Branch off `main` with the pattern `<task-number>-<short-description>`
  (e.g. `3-timer-wheel`).
- PR title format: `#<task-number> — <short description>`.
- PR body must include a line: `Opened via <agent>`
  (e.g. `Opened via Claude`, `Opened via Copilot`).
- Keep PRs focused on a single task.
- After opening a PR, add the PR link to the corresponding task in `TASKS.md`.
- Before opening the PR, move the task from **Open Tasks** to **Closed Tasks**
  in `TASKS.md`.
- After completing a task, always ask the user if they want to checkout back
  to `main`.

## Agent Role

- **Do not implement features or write production code unless the user
  explicitly asks.**
- The user writes the implementation — the agent assists with testing,
  reviewing, and running checks.
- When a task involves code changes, propose an approach and wait for the user
  to confirm or ask for implementation.
- Proactively write and run tests, run `cargo clippy`, and review code for
  correctness.

## Code Style

- Rust, built with Cargo. Source in `src/`, tests in `tests/`.
- **All tests go in the `tests/` directory** — do not use inline
  `#[cfg(test)]` modules in `src/`.
- `mio` is the only permitted external dependency. Do not add crates without
  explicit user approval.
- Follow existing patterns and module structure. New modules go in `src/`.
- Keep implementations simple. Avoid over-engineering or premature abstraction.
- The runtime is not TCP-aware. Do not introduce `TcpStream` or `TcpListener`
  into the core runtime modules (`event_loop.rs`, `registry.rs`,
  `timer.rs`). TCP belongs in consumer code or integration tests only.
- Do not implement `Future`, `Waker` (in the Rust async sense), thread pools,
  or work-stealing. This is a callback model, not an async/await model.

## Pre-commit Checklist

Before every commit, run these commands **in order** and ensure each one passes:

1. `cargo fmt` — all code must be formatted.
2. `cargo clippy -- -D warnings` — **zero warnings allowed**. If clippy reports
   any warnings or errors, fix them before continuing.
3. `cargo test` — all unit and integration tests must pass.

Do **not** commit or open a PR until all three pass. If a step fails, fix the
issue and re-run from that step.

Before opening a PR, also check `README.md` — if the change adds or modifies
public API surface, types, or observable behaviour, update the relevant section
of the README.

## Project Structure

| Path | Purpose |
|------|---------|
| `src/lib.rs` | Crate root — public re-exports of `EventLoop`, `Registry`, `EventHandler`, `Token`, `TimerId`, `ReadyState`, `Waker` |
| `src/event_loop.rs` | `EventLoop` struct — holds `mio::Poll`, `TimerWheel`, `running` flag; exposes `new()`, `waker()`, `run()`, `stop()` |
| `src/registry.rs` | `Registry` struct — wraps `mio::Registry` and a reference to `TimerWheel`; exposes `register`, `reregister`, `deregister`, `insert_timer`, `cancel_timer` |
| `src/timer.rs` | `TimerWheel` — hashed wheel timer: 512 slots × 1ms tick, O(1) insert, lazy cancel, `advance(now)`, `next_deadline()` |
| `src/types.rs` | `Token`, `TimerId`, `ReadyState`, `Waker` — all public types with no logic |
| `src/handler.rs` | `EventHandler` trait definition |
| `tests/` | All tests (unit and integration) |
| `docs/adr/` | Architecture Decision Records — read before modifying public API |