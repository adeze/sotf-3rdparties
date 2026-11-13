# Local patches on top of zed v1.9.0

This vendored `gpui_macos` fork carries the following local delta. Keep this
file current when adding further renderer changes relative to upstream zed
v1.9.0.

## 1. `Primitive::Custom` Metal zero-copy dispatch (MeshPlot, 2026-08-09)

Adds the macOS Metal half of GPUI's custom GPU primitive extension. The
renderer resolves the Task 8 `gpui::CustomDrawId` registry at the exact
`PrimitiveBatch::Custom` paint position, ends GPUI's active render encoder,
invokes a registered Metal callback against the same command buffer and
drawable texture, and resumes GPUI rendering with a `Load` pass.

- **`src/custom_metal.rs` (new)**: `MetalCustomDraw` and
  `MetalCustomDrawAdapter`, using the adapter downcast pattern so the shared
  GPUI registry remains platform-agnostic.
- **`src/metal_renderer.rs`**: dispatches registered Metal adapters without
  submitting the command buffer, converts scene `ScaledPixels` bounds to
  logical `Pixels` plus the drawable scale factor, and preserves scene order
  across the custom pass.
- **`src/gpui_macos.rs`**: exports the Metal callback trait and adapter.

Metal callbacks own their render encoder: they must use a `Load` color
attachment action, scissor to the supplied bounds after applying
`scale_factor`, and leave command-buffer submission to the GPUI renderer.

## 2. Zero-warnings allowances (Zed v1.25.2 re-vendor, issue #6 Phase C)

- **`src/gpui_macos.rs`**: crate-level `#![allow(deprecated)]`. This
  vendored upstream crate is written against the deprecated `cocoa` crate
  (~2.5k deprecation diagnostics); the objc2 migration is an
  upstream-scale rewrite and is not cherry-picked here.
- **`Cargo.toml`**: `[lints.rust] unexpected_cfgs` declares
  `cfg(feature, values("cargo-clippy"))` because objc 0.2's
  `msg_send!`/`sel!` macros expand through
  `cfg_attr(feature = "cargo-clippy", ...)` in this crate.
- Known residual: cargo prints a future-incompatibility notice for the
  registry `block 0.1.6` crate (pulled by `cocoa`/`metal`;
  uninhabited-type static, rust#74840). It is not a `-D warnings`
  failure and cannot be patched to this repo's `sotf-block` fork
  (which carries the fix) because `[patch]` replacements must keep the
  original package name. It disappears with the cocoa→objc2 migration.
