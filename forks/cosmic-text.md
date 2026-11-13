# cosmic-text

Upstream: https://github.com/pop-os/cosmic-text.git. Base: `0.19.0`.
Revision: `59089955e1c8698c6b83b2e6ab6ebceff825ff96`.
Revision status: recorded.
License: `MIT OR Apache-2.0`; preserved files: `LICENSE-APACHE`, `LICENSE-MIT`.
License evidence status: preserved.

## Why retained

Retain the GPUI text renderer snapshot compatible with fontdb 0.24 and its selected bidi behavior.

## Retained changes

Exact upstream snapshot; no new SOTF Rust patch during consolidation. Upstream LFS test fonts and images are fully fetched at the pinned revision and stored in this repository's LFS storage. SOTF embedded fonts are separately preserved under gpui/assets.
Read [the existing detail record](../cosmic-text/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `514693a3ee54cfecd55fb349c71c22f15e3058ae9b337ceccdc8ccb22478ea3b`. Pinned commit subject: chore: bump fontdb to 0.24; drop ttf-parser cargo deny clause.

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

Strict all-target Clippy, default-feature cargo tests/doctests and nextest PASS (42 cases, zero skipped), cosmic-text-fixed.log. Equivalent Option fallback/formatting/iterator fixes and explicit Apple-font test expectations; benchmarks use std::hint::black_box. Shared test helper dead-code allowance reflects separate integration binaries, not production. Private issue sotf-3rdparties#14.
