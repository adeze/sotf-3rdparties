# tract

Upstream: https://github.com/pierreaubert/tract.git. Base: `None`.
Revision: `706cccb56b2de6bfcbd0016b9c260aef183e41d7`.
Revision status: recorded.
License: `See preserved per-package license declarations`; preserved files: `LICENSE`, `LICENSE-APACHE`, `LICENSE-MIT`, `api/LICENSE`, `api/LICENSE-APACHE`, `api/LICENSE-MIT`, `api/proxy/LICENSE`, `api/proxy/LICENSE-APACHE`, `api/proxy/LICENSE-MIT`, `api/rs/LICENSE`, `api/rs/LICENSE-APACHE`, `api/rs/LICENSE-MIT`, `cli/LICENSE`, `cli/LICENSE-APACHE`, `cli/LICENSE-MIT`, `core/LICENSE`, `core/LICENSE-APACHE`, `core/LICENSE-MIT`, `data/LICENSE`, `data/LICENSE-APACHE`, `data/LICENSE-MIT`, `hir/LICENSE`, `hir/LICENSE-APACHE`, `hir/LICENSE-MIT`, `linalg/LICENSE`, `linalg/LICENSE-APACHE`, `linalg/LICENSE-MIT`, `nnef/LICENSE`, `nnef/LICENSE-APACHE`, `nnef/LICENSE-MIT`, `onnx-opl/LICENSE`, `onnx-opl/LICENSE-APACHE`, `onnx-opl/LICENSE-MIT`, `onnx/LICENSE`, `onnx/LICENSE-APACHE`, `onnx/LICENSE-MIT`, `pulse-opl/LICENSE`, `pulse-opl/LICENSE-APACHE`, `pulse-opl/LICENSE-MIT`, `pulse/LICENSE`, `pulse/LICENSE-APACHE`, `pulse/LICENSE-MIT`, `tensorflow/LICENSE`, `tensorflow/LICENSE-APACHE`, `tensorflow/LICENSE-MIT`.
License evidence status: preserved.

## Why retained

Keep ONNX random operators compatible with rand 0.10 and deterministic replay tests.

## Retained changes

SmallRng feature/API migration and explicit f32 replay tests on tract 0.22.4.
Release preparation updates derive-new to 0.7, replaces deprecated APIs,
resolves current strict lints, and refreshes TensorFlow documentation examples.
Generated constructor lint allowances are limited to affected derived types.

Read [the existing detail record](../tract/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `c5bce401c3ea5c2ec72091062672f3683657b639341e3689009b76729f3de9f9`. Pinned commit subject: Specify f32 in Tract random replay tests.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo check --manifest-path sotf-daw/Cargo.toml -p sotf-plugins --features onnx
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
