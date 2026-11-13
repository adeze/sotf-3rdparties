# Vendored: scheduler

- Upstream: https://github.com/zed-industries/zed/tree/v1.25.2/crates/scheduler
- Base ref: v1.25.2
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- `src/executor.rs`, `src/scheduler.rs`, and `Cargo.toml` replace the three
  blocking `flume` channels with equivalent standard-library MPSC channels.
  This removes the transitive dependency on yanked `spin` 0.9.8 without
  changing scheduler APIs or channel capacity.

### Crate-root lint allows (clippy default lints, upstream code unchanged)

Added at the top of `src/scheduler.rs` (Task 6, `just lint-host` gate with `-D warnings`):

- `#![allow(clippy::new_without_default)]` — `TestClock::new()` in `src/clock.rs`.
- `#![allow(clippy::type_complexity)]` — boxed `FnOnce` types in `src/executor.rs` and `src/scheduler.rs`.
- `#![allow(clippy::nonminimal_bool)]` — `!env::var(...).is_ok()` in `src/test_scheduler.rs`.
- `#![allow(clippy::unnecessary_map_or)]` — `.map_or(false, ...)` in `src/test_scheduler.rs` (2 sites).
- `#![allow(clippy::let_unit_value)]` — `let _ = ...` on unit-valued expressions in `src/tests.rs:301,337`.

- `Cargo.toml` and `src/test_scheduler.rs` drop the `backtrace` 0.3
  dependency on Apple mobile targets (`ios`, `tvos`, `watchos`,
  `visionos`), where it no longer compiles with libc >= 0.2.190 (the
  dyld image functions became macOS-only). A cfg-gated
  `mobile_no_backtrace` module provides capture-less `Backtrace` /
  `BacktraceFrame` stand-ins covering the API subset used here, so
  `TestScheduler` keeps its API while non-determinism reports on
  mobile carry no frames. Host builds are unchanged.
