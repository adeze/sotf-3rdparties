# predicates

Upstream: https://github.com/assert-rs/predicates-rs. Base: `3.1.4`.
Revision: `f54f2cd016a1df6c88a022c867e3ee04c70f9a3a`.
Revision status: recorded.
License: `MIT OR Apache-2.0`; preserved files: `LICENSE-APACHE`, `LICENSE-FLOAT-CMP`, `LICENSE-MIT`.
License evidence status: preserved.

## Why retained

Preserve the modern f64 comparison behavior and diagnostics used by tests.
The original fork consolidated float-cmp at 0.9; the release dependency refresh
restores 0.10.0 while retaining the comparison helpers.

## Retained changes

Release maintenance (2026-10-30): restored the nonempty and empty file
fixtures referenced by the file-content doctest and included them in
Cargo packaging. macOS Clippy, all 48 doctests, and release nextest passed before
the float-cmp dependency refresh; those executable gates must be rerun.

Private modernf64 helpers retained from float-cmp 0.10; optional dependency now
uses 0.10.0; local unpublished version. Offline all-feature resolution passes.
Read [the existing detail record](../predicates/SOTF_FORK_PROVENANCE.json) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `math-audio/crates/3rdparties/predicates`. Retained source repository/commit: `sotf-3rdparties@bfd07543a9f61040d70f8a1f5aa644fa80c7fb60`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-3rdparties/predicates/Cargo.toml --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
