# metaheuristics-nature-rs

Upstream: https://github.com/pierreaubert/metaheuristics-nature-rs.git. Base: `10.1.0`.
Revision: `4f0b603c521e0b4e20239c827f1a0975442bb740`.
Revision status: recorded.
License: `MIT`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Keep seeded optimization generators compatible with Rand 0.10 and Rand Distr 0.6.

## Retained changes

Exact pinned fork ports seeded RNG APIs and dependency versions; no additional algorithm change during relocation.
Read [the existing detail record](../metaheuristics-nature-rs/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `dcfd70b7cdabe9b13da7d2192423a06b666dbbe85289fdb8a571a198e0c4219b`. Pinned commit subject: Port seeded generators to Rand 0.10 and Rand Distr 0.6.

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

## 2026-10-30 release validation

Recognized the existing documentation cfg, updated lint key spellings, removed an unused rand import and derived Default for unit test fixtures. Solver algorithms unchanged. Strict all-target Clippy, tests/doctests and nextest pass on macOS: six nextest tests, zero skipped.
