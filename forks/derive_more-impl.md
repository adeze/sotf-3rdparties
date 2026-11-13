# derive_more-impl

Upstream: https://github.com/JelteF/derive_more. Base: `2.1.1`.
Revision: `f7bb41ac054c060caaf5ff3212e74e42794cb4b4`.
Revision status: recorded.
License: `MIT`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Maintain convert_case 0.11 compatibility without changing the public derive behavior.

## Retained changes

Dependency/MSRV metadata and unpublished 2.1.2 maintenance version; upstream Rust retained.
Read [the existing detail record](../derive_more-impl/SOTF_FORK_PROVENANCE.json) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `math-audio/crates/3rdparties/derive_more-impl`. Retained source repository/commit: `sotf-3rdparties@bfd07543a9f61040d70f8a1f5aa644fa80c7fb60`.

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

Strict all-target full-feature Clippy, cargo tests/doctests and nextest PASS (17 cases, zero skipped), derive-impl-docs-fixed.log. Constructor expansion emits field shorthand; equivalent Option/iterator lint fixes. Added the matching local macro facade as a dev dependency so public macro examples compile. Private issue sotf-3rdparties#13.
