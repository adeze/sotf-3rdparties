# Vendored: bench_metrics

- Upstream: https://github.com/zed-industries/zed/tree/v1.25.2/crates/bench_metrics
- Base ref: v1.25.2
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- `#[allow(clippy::let_unit_value)]` on the
  `erased_measurement_preserves_wrapped_values` test (issue #6 Phase C):
  FakeCounter's Intermediate is `()`, bound to mirror the real start/end
  pattern.
- `Cargo.toml`: package metadata (description/readme/keywords/categories)
  inherited from `workspace.package` for `clippy::cargo_common_metadata`.
