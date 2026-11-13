# derive_more

Upstream: https://github.com/JelteF/derive_more. Base: `2.1.1`.
Revision: `f7bb41ac054c060caaf5ff3212e74e42794cb4b4`.
Revision status: recorded.
License: `MIT`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Keep the macro facade aligned with the maintained implementation and case-conversion dependency.

## Retained changes

Unpublished 2.1.2 maintenance version; upstream Rust and feature definitions retained.
Local (issue #6 Phase C): empty `[lints]` clarifies non-inheritance for
cargo's `missing_lints_inheritance`; the optional `rustc_version` build
dependency was removed (and from `testing-helpers`) — dead weight here
since this fork ships `build = false` with no build script. The
`derive_more-impl` utils module carries `#[allow(dead_code, unused_mut)]`
because which helpers are dead depends on the consumer's feature set.
Read [the existing detail record](../derive_more/SOTF_FORK_PROVENANCE.json) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `math-audio/crates/3rdparties/derive_more`. Retained source repository/commit: `sotf-3rdparties@bfd07543a9f61040d70f8a1f5aa644fa80c7fb60`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo check --manifest-path sotf/Cargo.toml -p sotf-gpui --features shipping
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).

## macOS release validation (2026-10-30)

Restored 131 missing compile-fail stderr expectations unchanged from pinned upstream f7bb41ac054c060caaf5ff3212e74e42794cb4b4; archive SHA256 4767d5be0c899229f8f4536b2c38d54b5f2070fb82389aa6c1478486cbd6ca17. Fourteen require reviewed Rust 1.99 diagnostic wording/help/code-number updates; invalid programs still fail for the same reasons. Full rerun pending, derive-facade-verified.log.

Final native full-feature validation PASS: strict all-target Clippy, cargo tests/doctests, and nextest 871/871 with zero skipped (derive-facade-verified.log). Compiler snapshots were refreshed only after reviewing all fourteen diagnostic-only differences; final verification ran without TRYBUILD overwrite.
