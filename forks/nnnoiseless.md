# nnnoiseless

Upstream: https://github.com/jneem/nnnoiseless. Base: `0.1.1`.
Revision: `not recorded`.
Revision status: exact upstream commit not recorded; retained source commit identifies the imported fork.
License: `BSD-3-Clause`; preserved files: `COPYING`.
License evidence status: preserved.

## Why retained

Retain the RNNoise denoising implementation used by SOTF with fixed-size chunk and current dependency compatibility.

## Retained changes

Fixed-size chunk adaptation, rustfft alignment and lint/package maintenance; criterion made explicit during relocation.
Read [the existing detail record](../nnnoiseless/Cargo.toml.orig) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `sotf-daw/crates/3rdparties/nnnoiseless`. Retained source repository/commit: `sotf-daw@bbb0a8b924bbb368631f0d6a7426af25e66878f6`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-3rdparties/nnnoiseless/Cargo.toml --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).

## 2026-10-30 release validation

Corrected real-model test fixture paths for the sibling sotf-daw layout. Strict all-target Clippy, cargo test and nextest pass on macOS (25 nextest tests, zero skipped); asset hashes and independent transpose/inference checks preserved.
