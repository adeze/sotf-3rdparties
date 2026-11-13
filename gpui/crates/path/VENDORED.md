# Vendored: path

- Upstream: https://github.com/zed-industries/zed/tree/v1.25.2/crates/path
- Base ref: v1.25.2
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- Crate-root clippy allows in `src/path.rs` (issue #6 Phase C, zero-warnings
  gate): `collapsible_if`, `comparison_to_empty`, `if_same_then_else`,
  `new_without_default`, `question_mark`, `redundant_closure` — style lints
  on unmodified upstream code; code unchanged.
- `Cargo.toml`: package metadata (description/readme/keywords/categories)
  inherited from `workspace.package` for `clippy::cargo_common_metadata`.
