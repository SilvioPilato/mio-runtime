You are a technical assistant with deep expertise in Rust and distributed systems.
Your task is to create a TASKS.md for a new Rust project called `mio-runtime`.

## Format

Follow EXACTLY this format, modeled after the rustikv TASKS.md attached below.
Rules:

- Sections: "# In Progress", "# Open Tasks", "# Closed Tasks"
- Each task has a heading: `## #N — <title>`
- The body is dense prose: a complete technical description explaining WHAT to
  do, WHY to do it, and where the scope boundary lies. Do not use bullet lists.
  Write as if explaining the task to a senior engineer with no prior context —
  they must understand the motivation, not just the steps.
- If a task depends on another, state it explicitly: "Depends on #N."
- Closed tasks end with a `PR: <url>` line.
- Order open tasks by natural implementation priority (dependencies first).
- Number tasks starting from #1.

## Project context

`mio-runtime` is a single-threaded I/O event loop written in Rust, with `mio`
as its only external dependency. It is not TCP-aware: it operates on generic
`mio::Source` trait objects. It serves as the async I/O foundation for two
consumers: `rustikv` (replacing the thread-per-connection model) and `raft-rs`
(managing peer connections and timers).

The public API design is defined in ADR-001 (attached). Key decisions already
made:

- `&Registry` passed as a parameter to every callback — immediate modification,
  no indirection
- `EventHandler` trait with three methods:
  `on_event(&mut self, registry: &Registry, token: Token, interest: ReadyState)`
  `on_timer(&mut self, registry: &Registry, timer_id: TimerId)`
  `on_wake(&mut self, registry: &Registry)`
- `stop()` only on `EventLoop` — control centralized in the owner
- Single-tier timer wheel (512 slots × 1ms tick = 512ms range), sufficient for
  Raft election timeouts and heartbeats
- `ReadyState { readable: bool, writable: bool }` instead of re-exporting
  `mio::Interest` directly
- Single-threaded now; architecture does not preclude multi-threading later
  (no global state, `EventLoop` is an explicit struct)

## Components to implement

Translate these components into atomic, well-ordered tasks:

1. **Public types** — `Token`, `TimerId`, `ReadyState`, `Waker`
2. **`Registry`** — wrapper around `mio::Registry` plus a reference to the
   timer wheel; exposes `register`, `reregister`, `deregister`,
   `insert_timer`, `cancel_timer`
3. **Timer wheel** — hashed wheel timer: 512 slots, 1ms tick, O(1) insert,
   lazy cancel (cancelled flag), `advance(now)` returns `Vec<TimerId>`,
   `next_deadline()` answers "how long should I wait in poll()"
4. **`EventLoop`** — struct holding `mio::Poll`, timer wheel,
   `running: AtomicBool`; exposes `new()`, `waker()`, `run()`, `stop()`
5. **`EventLoop::run`** — the main loop: computes timeout from
   `next_deadline()`, calls `poll.poll()`, calls `timer_wheel.advance()`,
   dispatches events to `on_event` / `on_timer` / `on_wake` callbacks
6. **rustikv integration** — replaces the thread-per-connection model with
   `EventLoop`; requires a `ParseState` state machine for BFFP parsing
   (`ReadingHeader` / `ReadingPayload`) and a `Connection` struct with
   `read_buf`, `write_buf`, `write_offset`

## Constraints

- Do not implement TCP-awareness in the runtime — the runtime does not call
  `accept()` and does not explicitly know about `TcpStream`
- Do not implement `Future` / `Waker` in the Rust async sense — this is a
  callback model, not a polling model
- Do not implement multi-threading — deferred by design
- Do not implement TLS or UDP
- Every task must be implementable in isolation and verifiable with a test
  before moving to the next
- The rustikv integration task is last — it depends on everything else

## Attachments

<adr>
# Agent Instructions

This is a didactic key-value store built while reading *Designing Data-Intensive Applications*. It supports two storage engines: a Bitcask-style hash-indexed engine (`kv`) and an LSM-tree engine with sorted string tables (`lsm`), selectable at startup via `--engine`. The project is educational — simplicity and clarity matter more than production-readiness.

## Task Workflow

- Tasks live in `TASKS.md` at the repo root, split into **In Progress**, **Open Tasks**, and **Closed Tasks**.
- Before starting work, read `TASKS.md` and identify the relevant task number.
- When starting a task, move its `## #N` section from **Open Tasks** to **In Progress**.
- When a task is done, move its entire `## #N` section from **In Progress** to **Closed Tasks** — never delete it.
- New tasks get the next sequential `#N` number.

## Git & PRs

- **Never push directly to `main`** — always use a dedicated branch and open a PR.
- Use **git** and the **gh CLI** for all version control and PR operations.
- Branch off `main` with the pattern `<task-number>-<short-description>` (e.g. `15-crc-checksums`).
- PR title format: `#<task-number> — <short description>`.
- PR body must include a line: `Opened via <agent>` (e.g. `Opened via Copilot`, `Opened via Claude`, `Opened via OpenCode`).
- Keep PRs focused on a single task.
- After opening a PR, add the PR link to the corresponding task in `TASKS.md`.
- Before opening the PR, move the task from **Open Tasks** to **Closed Tasks** in `TASKS.md`.
- After completing a task, always ask the user if they want to checkout back to `main`.

## Agent Role

- **Do not implement features or write production code unless the user explicitly asks.**
- The user writes the implementation — the agent assists with testing, reviewing, and running checks.
- When a task involves code changes, propose an approach and wait for the user to confirm or ask for implementation.
- Proactively write and run tests, run `cargo clippy`, and review code for correctness.

## Code Style

- Rust, built with Cargo. Source in `src/`, tests in `tests/`.
- **All tests go in the `tests/` directory** — do not use inline `#[cfg(test)]` modules in `src/`.
- Prefer hand-rolled implementations over external crates when the goal is to learn the concept.
- Follow existing patterns and module structure. New modules go in `src/`.
- Keep implementations simple. Avoid over-engineering or premature abstraction.

## Pre-commit Checklist

Before every commit, run these commands **in order** and ensure each one passes:

1. `cargo fmt` — all code must be formatted.
2. `cargo clippy -- -D warnings` — **zero warnings allowed**. If clippy reports any warnings or errors, fix them before continuing.
3. `cargo test` — all unit and integration tests must pass.

Do **not** commit or open a PR until all three pass. If a step fails, fix the issue and re-run from that step.

Before opening a PR, also check `README.md` — if the change adds or modifies CLI flags, commands, or user-visible behaviour, update the relevant section of the README.

## Project Structure

| Path | Purpose |
|------|---------|
| `src/main.rs` | TCP server, command parsing, request handling |
| `src/bin/rustikli.rs` | Interactive REPL client binary |
| `src/cli.rs` | `parse_command` — input parsing for rustikli, testable from `tests/` |
| `src/engine.rs` | `StorageEngine` trait (`get`/`set`/`delete`/`compact`/`list_keys` + `Send + Sync`) |
| `src/bffp.rs` | Binary frame fixed protocol — length-prefixed framing, encode/decode, op codes |
| `src/kvengine.rs` | Bitcask-style engine — hash index, append-only segments, hint files |
| `src/lsmengine.rs` | LSM-tree engine — memtable + sorted SSTable segments |
| `src/memtable.rs` | In-memory `BTreeMap` write buffer with size tracking and tombstones |
| `src/sstable.rs` | Sorted string table segment files with sparse index for fast lookups |
| `src/wal.rs` | Write-ahead log for LSM engine — append, replay on startup, reset after flush |
| `src/bloom.rs` | Hand-rolled Bloom filter — probabilistic membership test for fast negative lookups |
| `src/record.rs` | On-disk record format (header + CRC + key + value) |
| `src/crc.rs` | Hand-rolled CRC32 (IEEE polynomial, compile-time table) |
| `src/hash_index.rs` | In-memory hash map index (key → IndexEntry: segment + offset) |
| `src/hint.rs` | Hint file format for fast Bitcask startup (key + offset, no values) |
| `src/segment.rs` | Segment file naming, parsing, listing |
| `src/settings.rs` | CLI argument parsing, `EngineType`, `FSyncStrategy` enums |
| `src/stats.rs` | Atomic runtime counters |
| `src/worker.rs` | Background thread worker (periodic fsync, clean shutdown via `Drop`) |
| `tests/` | All tests (unit and integration) |

</adr>

<agents_md>
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
| `docs/adr/` | Architecture Decision Records — read before modifying public API |</agents_md>

<tasks_md_example>
# In Progress

# Open Tasks

## #68 — Client-side request pipelining + bench coverage

Verify and exercise pipelining over the existing BFFP framing. Pipelining is purely a client-side optimization for now: the client writes N request frames back-to-back without reading between them, then drains N responses in arrival order. TCP and the per-connection request loop already preserve order, so no protocol or server changes are expected — but the task includes a small integration test that asserts the server doesn't deadlock when many requests are buffered before the client reads, and that responses come back in the order they were issued. Adds `--pipeline N` (default `1`) to `redis-compare` so the bench can measure rustikv with RTT removed and compare against `redis-benchmark -P` numbers on the same workload. Update `bench/docker-compose.yml` usage docs accordingly. Out of scope: any tagged/multiplexed frames or out-of-order responses (see #69).

## #69 — Tagged/multiplexed BFFP frames (`correlation_id`)

Schema-evolve BFFP to carry a client-assigned `correlation_id: u32` in every request and response frame, allowing out-of-order responses on a single connection. Lets clients hold many requests in flight concurrently and matched purely by ID — Kafka/MongoDB-style. Reserve a sentinel value (e.g. `u32::MAX`) for connection-level error frames where the server can't parse a request well enough to know its ID. Bump the BFFP version byte and reject mixed-version frames on a connection. Server side: demonstrate the value by allowing the per-connection handler to dispatch reads concurrently (depends on #61 — engine-internal concurrency — for any meaningful throughput gain on multi-core; until then, the change is correctness-only). Client side: a small async-style client API that returns a future per request, resolved when the matching response arrives. Bench: extend `redis-compare` with `--inflight N` (distinct from `--pipeline N` from #68) — issues N concurrent in-flight requests with bookkeeping over `correlation_id`. Compare both modes head-to-head and against Redis-pipelined numbers. Depends on #68 landing (so the bench has something to compare against) and benefits from #61.

## #62 — Versioned snapshots for KVEngine (RocksDB-style)

Replace the `RwLock<HashIndex>` in KVEngine with `RwLock<Arc<Version>>` where `Version` holds the index + segment list. Readers clone the Arc and release the lock before file I/O, eliminating the concurrent throughput regression from #61. Old segments stay alive until the last reader drops its Arc. Compaction waits for in-flight readers before deleting old files.

## #26 — Persist Bloom filters and sparse index to disk (DDIA Ch. 3)

Bloom filters and sparse indexes are currently rebuilt by scanning every SSTable file on startup. Serialize them to sidecar files (similar to hint files for Bitcask) so that LSM startup skips the full-file scan. Natural companion to the existing hint file infrastructure.

## #28 — mmap for SSTable reads (DDIA Ch. 3)

Memory-map SSTable files so lookups become pointer arithmetic instead of `read()` syscalls. Combined with the sparse index, this eliminates per-lookup I/O overhead. Good exercise in OS-level I/O and `unsafe` Rust, with platform-specific considerations (Windows vs. Unix).

## #31 — Connection timeouts and limits

Currently there is no read timeout and unbounded thread spawning per TCP connection. Add `SO_TIMEOUT` on sockets, a maximum connection limit, and graceful backpressure when the limit is reached. Addresses real operational concerns without changing the threading model.

## #32 — Async I/O with tokio

Replace the thread-per-connection TCP model with async handling using tokio. Enables higher concurrency with lower resource usage. A major Rust learning exercise and a stepping stone toward replication and distributed features.

## #33 — Single-leader replication (DDIA Ch. 5)

Add a `--role leader|follower` flag. The leader streams its write-ahead log to followers over TCP; followers replay it to maintain a replica. Teaches replication logs, consistency models, and failover — core DDIA Ch. 5 material. Depends on WAL (#25).

## #34 — Consistent hashing / partitioning (DDIA Ch. 6)

Shard the keyspace across multiple kv-store instances using consistent hashing or range-based partitioning. A coordinator node routes requests to the correct shard. Teaches DDIA Ch. 6 partitioning concepts: rebalancing, hot spots, and partition-aware routing.

## #36 — Per-operation latency histograms

Extend `Stats` to track per-operation latency distributions (p50/p95/p99). Implement a streaming quantile estimator (e.g., DDSketch or simple histogram buckets). Surface the results via the `STATS` command. Good exercise in streaming algorithms.

## #37 — Crash-recovery and fault-injection tests

Write tests that simulate crashes mid-write and mid-compaction (e.g., truncated files, partial records, missing hint files) and verify the engine recovers correctly. Validates the durability guarantees of both engines and exercises the CRC integrity checks.

## #40 — Engine info in `STATS` output

Extend the `STATS` command to include which engine is active, segment count, total data size on disk, and (for LSM) current memtable size. Makes the storage internals visible during interactive exploration.

## #43 — `DBINFO` command

Add a TCP command that dumps internal storage state: segment file listing, index size, bloom filter stats (estimated false positive rate), hint file presence, sparse index entry count. Lets you observe compaction shrinking segments and see the sparse index in action.

## #50 — `PREFIX` command (LSM only)

Add a `PREFIX <prefix>` TCP command that returns all key-value pairs whose keys start with the given string. LSM-only — implemented as a range scan `[prefix, prefix\xff]` on the sorted memtable and SSTables. The KV engine returns an error. Depends on #48 (`RANGE`) since it's a specialisation of range scan. Depends on #30 (binary protocol).

## #51 — `COUNT` command (LSM only)

Add a `COUNT <start> <end>` TCP command that returns the number of live keys in the inclusive range `[start, end]` without returning the values themselves. LSM-only. Shares the same merge-scan logic as `RANGE` but only emits a count. Depends on #48.

## #52 — `FIRST` and `LAST` commands (LSM only)

Add `FIRST` and `LAST` TCP commands that return the lexicographically smallest and largest live keys (with their values). LSM-only — trivially answered from the `BTreeMap` memtable and the first/last entries of the oldest/newest SSTables. The KV engine returns an error.

## #55 — `INCR` command

Add an `INCR <key>` TCP command that atomically increments an integer value stored at a key (creating it at 1 if absent). Returns the new value. Teaches read-modify-write atomicity — must be handled under the engine's write lock to avoid races. Supported by both engines.

## #56 — `TTL` command

Add a `TTL <key> <seconds>` TCP command that associates an expiry timestamp with a key. Expired keys are invisible to reads and cleaned up during compaction. Requires storing the expiry alongside the value in the record format (or as a separate metadata field). Good exercise in extending the on-disk format and compaction logic.

## #58 — `FLUSH` command (LSM only)

Add a `FLUSH` TCP command that forces an immediate memtable flush to a new SSTable, regardless of whether the flush threshold has been reached. LSM-only. Useful for testing, observability, and ensuring durability on demand. The KV engine returns an error (it has no memtable to flush).

## #59 — `SCAN` command

Add a `SCAN <cursor> <count>` TCP command for stateless paginated key iteration. The cursor is an opaque offset into the sorted keyspace; the server returns up to `count` keys starting at that offset plus the next cursor (or `0` when iteration is complete). Both engines support it — LSM iterates the sorted keyspace naturally; KV sorts the hash index keys at query time. Teaches stateless pagination and the tradeoffs of offset-based vs. hash-based cursors. Depends on #30 (binary protocol).

## #62 — Block-based segment format with compression for KV engine (DDIA Ch. 3)

Apply the block-based compression format (from #29) to the KV engine's append-only segments. Partition segments into fixed-size blocks with optional LZ77 compression per block. Update the hash index rebuild to work with blocks. Enables compression benefits for the Bitcask-style engine and demonstrates that block layouts are engine-agnostic. Depends on #29 (block format, LZ77 codec).

## #63 — Upgrade LZ77 to control-byte encoding (compression optimization, low priority)

Replace the varint-based LZ77 encoding (from #29) with control-byte encoding (Deflate/zlib style). Each control byte represents 8 operations (literals or match references), reducing metadata overhead and improving compression ratio by ~5%. Depends on #29. Low priority—varint is "good enough" for most workloads; this is a performance/space optimization for production use.

## #64 — Extend block header with integrity checks and versioning (low priority)

Extend the block-based SSTable format (from #29) with per-block integrity checks and format versioning. Add optional fields to the block header: (1) per-block CRC32 for early corruption detection, (2) block format version byte for forward/backward compatibility. This enables graceful format evolution without breaking existing SSTables. Depends on #29. Low priority—task #29 uses record-level CRC as the primary safety mechanism; this is an enhancement for production robustness.

## #65 — Block compression optimization evaluation (low priority, research task)

Comprehensive evaluation of optimization strategies for block-based compression (from #29). Implement and benchmark: (1) block-level decompression caching (LRU in-memory cache), (2) lazy decompression (only decompress blocks on key access), (3) parallel decompression for range scans (decompress multiple blocks concurrently), (4) SIMD optimization for LZ77 match-finding and copying, (5) prefetching for sequential reads. Measure latency, throughput, and memory overhead against baseline. Generate comparison report. Depends on #29. Low priority—exploratory task to understand real-world performance gains and tradeoffs.

# Closed Tasks

## #31 — Connection timeouts and limits

Currently there is no read timeout and unbounded thread spawning per TCP connection. Add `SO_TIMEOUT` on sockets, a maximum connection limit, and graceful backpressure when the limit is reached. Addresses real operational concerns without changing the threading model.

PR: <https://github.com/SilvioPilato/rustikv/pull/40>

## #67 — LZ77 encoder performance: flat hash table + rolling hash

Replace the `HashMap<[u8;3], usize>` head-of-chain table in the LZ77 encoder with a flat `Vec<u32>` and a zlib-style rolling hash. The rolling hash feeds one byte at a time (`((prev << H_SHIFT) ^ byte) & mask`) so the literal branch advances in a single XOR+shift op. The match branch rolls through intermediate positions to keep state current. Eliminates per-lookup heap allocation and SipHash overhead, reducing encode cost at large payload sizes. Depends on #66.

PR: <https://github.com/SilvioPilato/rustikv/pull/39>

## #66 — Fix LZ77 compression quality on low-entropy input (incremental chain building)

The current `Lz77::encode` pre-built the entire hash chain before encoding began (`get_hash_chain`), storing the *last* occurrence of each 3-byte key across the whole input. At position `pos`, the hash table entry therefore pointed near N−3 (end of file), which is in the future relative to `pos`. All 128 MAX_CHAIN traversal steps were exhausted skipping forward-looking candidates without finding any valid back-reference. Result: uniform-byte input (e.g. `b'x'.repeat(N)` for large N) produced only literals — input doubled in size and encode was catastrophically slow (5 write ops/sec at 1 MB, disk 2×).

Fix: build the hash chain **incrementally during encoding** — insert `pos` only after processing it, so the chain always contains positions strictly less than `pos`. All MAX_CHAIN steps now evaluate real match candidates.

Benchmark results confirm the fix: at 1 MB payload, write throughput went from 5 → 21 ops/sec (+320%) and on-disk size went from 1,048 MB → 8 MB (−99.2%). See `docs/benchmark-comparison-2026-04-24-lz77fix.md` and `docs/benchmark-lz77-2026-04-24.md`.

PR: <https://github.com/SilvioPilato/rustikv/pull/38>

## #29 — Block-based SSTable format with compression (DDIA Ch. 3)

Partitioned SSTables into blocks with optional per-block LZ77 compression. Added a hand-rolled LZ77 codec (`src/lz77.rs`) using varint-encoded literal/match tokens, a 32 KB sliding window, and hash-chain match finding capped by `MAX_CHAIN`. New `src/block.rs` defines a 9-byte `BlockHeader` (`uncompressed_size`, `stored_size`, `compression_flag`), a `BlockWriter` that buffers records up to a target size and flushes compressed blocks, and a `BlockReader` that decompresses on read. Rewrote `SSTable::from_memtable`, `get`, `iter`, and `rebuild_index` to work block-by-block; the sparse index now points to block offsets instead of record offsets. New CLI flags `--block-size-kb` (default 4, range 1–1024) and `--block-compression` (`none`|`lz77`, default `lz77`) plumbed through `Settings` → `LsmShared` → `SizeTiered`/`Leveled` strategies. Breaking change: old record-only SSTables are not readable. Also fixed a latent `SSTableIter` bug (infinite loop yielding `Err` forever on CRC mismatch — discovered mid-implementation when the test binary hit 20+ GB RAM); iterator is now a fused iterator via a `done` flag.

PR: <https://github.com/SilvioPilato/rustikv/pull/37>

## #61 — Engine-internal concurrency: write buffering and fine-grained locking

Replace the single global `Arc<RwLock<Box<dyn StorageEngine>>>` with engine-internal locking so readers never wait for writers. `StorageEngine` trait methods change from `&mut self` to `&self` (interior mutability). KVEngine gets a write buffer (`RwLock<HashMap>`) with WAL for durability and batched flushes to disk. LsmEngine gets double-buffered memtables — an active `RwLock<Memtable>` and an immutable `RwLock<Option<Memtable>>` flushed to SSTable in a background thread. Both engines define explicit lock orderings to prevent deadlocks. `main.rs` drops the global lock entirely. Compaction blocking is out of scope (separate task). Design spec: `docs/superpowers/specs/2026-04-05-engine-concurrency-design.md`.

PR: <https://github.com/SilvioPilato/rustikv/pull/35>

## #60 — Extended kvbench scenarios (delete, overwrite, zipfian, mixed)

Added four new benchmark scenarios to `kvbench`: (1) DELETE phase — deletes a configurable fraction of keys and re-reads to measure tombstone overhead; (2) OVERWRITE phase — overwrites surviving keys N times to measure write amplification; (3) Zipfian read distribution — hot-key skewed reads via `--zipf <s>` to test Bloom filter effectiveness; (4) Mixed concurrent mode — writers and readers hit overlapping keys simultaneously for a configurable duration, exposing lock contention. Introduced `BenchConfig` struct to bundle parameters. New CLI flags: `--delete-ratio`, `--overwrite-rounds`, `--zipf`, `--mixed-duration`.

PR: <https://github.com/SilvioPilato/rustikv/pull/34>

## #27 — Leveled compaction (DDIA Ch. 3)

Implemented LevelDB-style leveled compaction as a `StorageStrategy`. Added `Level` struct with self-contained compaction triggers (L0: file count threshold, L1+: byte budget with 10x scaling per level). Cross-level merge via `compact_levels` merges source files with overlapping target files in one pass. Tombstones preserved on non-terminal levels, dropped on terminal. Leveled SSTable filenames encode the level (`{name}_L{n}_{timestamp}.sst`) for correct placement on restart. Wired into `main.rs` via `--storage-strategy leveled` with three new CLI flags (`-lnl`, `-ll0`, `-ll1`). 34 new tests in `tests/leveled.rs`.

PR: <https://github.com/SilvioPilato/rustikv/pull/33>

## #42 — Load generator / benchmark tool (`kvbench`)

Add a `cargo run --bin kvbench` binary that writes N random keys, reads them back, and prints throughput and latency stats. The key value: run it against `--engine kv` then `--engine lsm` to compare and make the write-amplification and read-amplification tradeoffs from DDIA Ch. 3 tangible.

PR: <https://github.com/SilvioPilato/rustikv/pull/32>

## #48 — `RANGE` command (LSM only)

Add a `RANGE <start> <end>` TCP command that returns all key-value pairs whose keys fall in the inclusive range `[start, end]`. Implement it only on the LSM engine — the KV (Bitcask) engine returns an error, making the hash-index limitation tangible. The LSM implementation merges results from the memtable (`BTreeMap::range`) and all SSTable iterators, applying tombstone suppression and returning the newest value per key in sorted order. Add `fn range(&self, start: &str, end: &str) -> Result<Vec<(String, String)>, io::Error>` to the `StorageEngine` trait. Depends on #30 (binary protocol) for clean multi-value response framing.

PR: <https://github.com/SilvioPilato/rustikv/pull/31>

## #53 — `MGET` command

Add a `MGET <key1> <key2> ...` TCP command that fetches multiple keys in a single round trip and returns their values (or null/missing markers for absent keys). Supported by both engines. Depends on #30 (binary protocol) for multi-value response framing.

PR: <https://github.com/SilvioPilato/rustikv/pull/30>

## #54 — `MSET` command

Add a `MSET <k1> <v1> <k2> <v2> ...` TCP command that writes multiple key-value pairs atomically in a single round trip. Supported by both engines. Reduces client-server overhead for bulk writes.

PR: <https://github.com/SilvioPilato/rustikv/pull/30>

## #57 — `PING` command

Add a `PING` TCP command that returns `PONG`. Trivial to implement — useful as a health check and connection keep-alive. Standard across Redis, Memcached, and most TCP servers.

PR: https://github.com/SilvioPilato/rustikv/pull/29

## #41 — CLI client (`rustikli`)

Add a `cargo run --bin rustikli` binary that connects to the server and provides a REPL-style interface for sending commands. Avoids the netcat "blank line after each command" friction and provides a nicer interactive experience.

PR: https://github.com/SilvioPilato/rustikv/pull/28

## #49 — `EXISTS` command

Add an `EXISTS <key>` TCP command that returns `1` if the key exists, `0` if not — without fetching the value. Both engines support it. On LSM, the bloom filter makes this especially efficient (fast negative lookups). Useful as a standalone command and as a building block for conditional operations.

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/27

## #39 — `LIST` command

There is currently no way to see what keys exist. Wire a `LIST` TCP command through the `StorageEngine` trait. `KVEngine` already has `ls_keys()` via `HashIndex`; `Memtable` has `entries()` for the LSM side. Return all keys to the client.

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/26

## #30 — Binary protocol with length-prefixed framing (DDIA Ch. 4)

Replace the text-based "line + blank line" TCP protocol with length-prefixed binary frames. Eliminates ambiguity around spaces in values, enables request pipelining, and is a good introduction to encoding formats and schema evolution (DDIA Ch. 4).

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/25

## #25 — WAL (Write-Ahead Log) for the LSM memtable (DDIA Ch. 3)

The LSM engine's memtable is currently volatile — a crash before flush loses all in-flight writes. Add a write-ahead log that persists every write before applying it to the memtable, and replays uncommitted entries on startup. This is a core LSM-tree concept directly from DDIA's discussion of log-structured storage.

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/24

## #35 — Automatic compaction trigger

Instead of manual `COMPACT` commands, trigger compaction automatically when dead-bytes / total-bytes exceeds a configurable threshold or when segment count exceeds a limit. The trigger runs in a background thread (matching the manual `COMPACT` pattern) so writes are never blocked. `segment_count()` added to the `StorageEngine` trait and implemented for both engines — KV tracks it via a field incremented on segment roll and reset on compaction; LSM returns `self.segments.len()`. Both conditions (ratio and segment count) are evaluated for every engine; natural zero-values disable the irrelevant condition per engine.

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/23

## #14 — Hardcoded port in integration tests

Integration tests now use OS-assigned port 0. Server writes actual bound address to a file that tests read back, with proper address conversion (0.0.0.0 → 127.0.0.1) for client connectivity. Thread-local storage and mutex poisoning recovery for reliable test execution.

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/22

## #47 — `LsmEngine::delete` always returns `Some(())`

Fixed `LsmEngine::delete` to return `Ok(None)` for nonexistent keys (matching KV engine behavior), not always `Ok(Some(()))`. Protocol consistency so TCP server says "Not found" for missing keys instead of always "OK". Updated corresponding test.

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/22

## #38 — `--help` usage message

Added proper help banner to `Settings::print_help()` that lists all CLI flags with descriptions and defaults. Running with no arguments or `-h/--help` displays usage instead of panicking.

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/22

## #46 — Concurrent `get()` races on shared file offset (Unix/Linux)

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/21

`KVEngine::get()` uses `try_clone()` on the active file for reads. On Unix/Linux, `dup()` shares the file offset across cloned descriptors, so concurrent readers (allowed by `RwLock::read()`) race on seek+read. Fixed by using `File::open()` for the active-segment read path instead of `try_clone()`, giving each reader an independent file descriptor. Added a concurrent-reads stress test (8 threads × 200 iterations × 100 keys) that exposed the race on Windows too.

## #45 — WRITE command loses whitespace fidelity

The `parse_message` function uses `split_whitespace` + `join(" ")` to reconstruct the value. This collapses consecutive spaces, tabs, and other whitespace into single spaces. For example, `WRITE key hello··world` (two spaces) stores `"hello world"` (one space). Fix by locating the value substring in the original input rather than splitting and re-joining.

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/20

## #44 — SSTableIter silently swallows all errors

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/19

`SSTableIter::next()` uses `.ok()` which converts **all** I/O and CRC errors into `None` (treated as EOF). This means corrupt records, CRC mismatches, and I/O failures are silently ignored. During `get()`, a corrupt record before the target key ends the scan early, returning "not found" even if the key exists. During `compact()`, corrupt records are silently dropped, causing **data loss**. The CRC32 integrity verification is effectively defeated for the entire LSM engine. The iterator should propagate errors instead of swallowing them.

## #19 — Bloom filter for key existence (DDIA Ch. 3)

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/18

Once there are multiple segments (from #16 or #18), checking every segment for a missing key is expensive. A per-segment **Bloom filter** lets you skip segments that definitely don't contain the key. Implementing one from scratch (bit array + k hash functions) is a good exercise in probabilistic data structures, directly referenced in DDIA's LSM-Tree discussion.

## #18 — Simple SSTable / sorted segments (DDIA Ch. 3)

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/17 segment format as a second storage engine alongside the existing Bitcask-style KVEngine.

**Architecture changes:**

- Extracted `StorageEngine` trait (`src/engine.rs`) with `get`, `set`, `delete`, `compact` + `Send + Sync` supertraits.
- Existing Bitcask DB renamed to `KVEngine` (`src/kvengine.rs`), implements `StorageEngine`.
- Added `--engine kv|lsm` CLI flag; `main.rs` uses `Box<dyn StorageEngine>` for runtime polymorphism.

**LSM implementation:**

- `Memtable` (`src/memtable.rs`): in-memory `BTreeMap<String, Option<String>>` with size tracking, tombstones, flush threshold.
- `SSTable` (`src/sstable.rs`): sorted segment files using the existing `Record` format. Sparse index (sampled every 64 keys) with `partition_point` binary search for fast offset-based lookups. BufReader for buffered I/O.
- `LsmEngine` (`src/lsmengine.rs`): wires memtable + SSTable segments. Reads check memtable first (distinguishing tombstones from missing), then segments newest-to-oldest. Compaction merge-sorts all segments + memtable, drops tombstones.

**Tests:** 36 new tests (13 memtable, 9 sstable, 14 lsmengine). Total: 83 tests passing.

## #23 — Background thread/timer infrastructure

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/16

Added a `BackgroundWorker` struct (`src/worker.rs`) that spawns a thread with a configurable tick interval, runs a job each tick via `park_timeout`, and shuts down cleanly on `Drop` (stop flag + `unpark` + `join`). Integrated as the first periodic job: `FSyncStrategy::Periodic(Duration)` opens a duplicate file descriptor each tick and calls `sync_all()`. The worker is restarted on segment rolls. Extracted `spawn_fsync_worker` helper to deduplicate the pattern across `new`, `from_dir`, and `roll_segment`. Updated `parse_fsync` to accept `every:Ns` syntax. Added 5 tests.

## #24 — Rust best practices cleanup

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/15

Applied idiomatic Rust improvements across the codebase:

1. `DB::new` returns `io::Result<DB>` instead of panicking on filesystem errors.
2. Reduced `unwrap()` in production paths — `main()` returns `io::Result<()>` and uses `?`; `roll_segment` maps `SystemTimeError` via `io::Error::other`.
3. Simplified `Record::read_next` — replaced verbose `match` with `let header = Record::read_header(file)?;`.
4. `Segment` derives `Clone` for cleaner usage in `from_dir`.
5. `ls_keys` returns `impl Iterator<Item = &String>` instead of leaking `hash_map::Keys`.
6. Removed redundant `parse::<String>()` calls in `settings.rs`.
7. Updated stale doc comments on `db.rs` methods and added docs to previously undocumented methods.

## #17 — Hint files for fast startup (DDIA Ch. 3, Bitcask paper)

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/14

Added hint files — sidecar `.hint` files written during compaction containing `(key_size, offset, tombstone, key)` tuples. On startup, `from_dir` loads the index from hint files when available (skipping value bytes), falling back to full record scan when no hint exists. Compaction writes one hint file per new segment and cleans up old hint files alongside old segments.

## #22 — Move Record free functions into impl block

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/13

Refactored `read_record`, `read_record_at`, and `append_record` from free functions in `record.rs` into methods on `Record`: `Record::read_next()`, `Record::read_record_at()`, `record.append()`. Updated all call sites in `db.rs`, `hash_index.rs`, and `tests/crc.rs`.

## #13 — Review sync strategy for write performance

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/12

`append_record` no longer calls `sync_all()` on every write. Durability is now controlled by a configurable `FSyncStrategy` enum (`Always`, `EveryN(n)`, `Never`) passed to `DB::new` / `DB::from_dir` and settable via `--fsync-interval` CLI flag. `Always` preserves the original behavior (default). Compaction unconditionally fsyncs before deleting old segments.

## #16 — Segment size limit + multi-segment reads (DDIA Ch. 3)

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/11

The DB uses a single segment that grows forever. DDIA describes how Bitcask rolls to a new segment file once the active one hits a size threshold, and compaction merges old segments. The work:

- Add a `max_segment_bytes` setting.
- When `append_record` would exceed the limit, close the current segment and open a new one.
- On read, if a key's offset refers to an older segment, open that file.
- Compaction merges all segments into one fresh segment.

This is the natural continuation of the existing segment infrastructure and teaches **log-structured storage lifecycle**.

## #15 — CRC checksums per record (DDIA Ch. 3)

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/10

The record format currently has no integrity check. Bitcask stores a CRC with every record so that corrupted bytes are detected on read rather than silently returning garbage. Add a CRC32 field to the record header (4 bytes, computed over key+value+tombstone), verify it in `read_record`, and return an error on mismatch. This teaches **data integrity at the storage layer** — a topic DDIA revisits in Chapters 3, 5, and 7.

## #21 — Fix clippy warnings

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/9

Fix all clippy warnings (`cargo clippy -- -D warnings`): redundant field name, identity op, needless borrows, needless `Ok(?)`  wrapper, missing `Default` impl, `SeekFrom::Current(0)` → `stream_position()`, missing `truncate` on `OpenOptions::create`, redundant `trim()` before `split_whitespace()`.

## #20 — Add agent config files and task backlog (#15–#19)

PR: https://github.com/SilvioPilato/Hash-Index-KV-Store/pull/8

Add `AGENTS.md`, `CLAUDE.md`, `.github/copilot-instructions.md`, and `.github/hooks/post-edit.json` to the repo so that AI coding agents follow project conventions. Also add tasks #15–#19 to `TASKS.md` as the next batch of planned work (CRC checksums, segment size limits, hint files, SSTables, Bloom filters) and a "Closed Tasks" section.
</tasks_md_example>