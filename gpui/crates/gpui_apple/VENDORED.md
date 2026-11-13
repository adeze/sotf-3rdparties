# Vendored: gpui_apple

- Upstream: https://github.com/zed-industries/zed/tree/v1.25.2/crates/gpui_apple
- Base ref: v1.25.2
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- Crate-root clippy allows in `src/gpui_apple.rs` (issue #6 Phase C,
  zero-warnings gate): `collapsible_if`, `missing_safety_doc`,
  `new_without_default` — style lints on unmodified upstream code.
- `src/metal_renderer.rs`: `system_default_device` gated to
  `cfg(target_os = "ios")` (its only caller) instead of warning as dead
  code on macOS.
- `Cargo.toml`: package metadata (description/readme/keywords/categories)
  inherited from `workspace.package` for `clippy::cargo_common_metadata`.
