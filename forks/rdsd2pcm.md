# rdsd2pcm

Upstream: https://github.com/clone206/rdsd2pcm. Base: `0.4.0`.
Revision: `not recorded`.
Revision status: exact upstream commit not recorded; retained source commit identifies the imported fork.
License: `GPL-3.0-or-later`; preserved files: `LICENSE`, `src/dsd_reader/LICENSE`.
License evidence status: preserved.

## Why retained

Provide DSD-to-PCM conversion without forcing file conversion dependencies into the DSP-only path.

## Retained changes

Release maintenance (2026-10-30): imported Rand 0.10's RngExt for random
sample methods. macOS default-feature strict Clippy, tests and release
nextest pass (six cases); dither distributions are unchanged.

file-io feature split, rand API/dependency and lint maintenance; nested dsd-reader retained.
Read [the existing detail record](../rdsd2pcm/README.md) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `symphonia-add-ons/rdsd2pcm`. Retained source repository/commit: `symphonia-add-ons@53c255d29ceaf61bbaa14f573a0b44341c5ae452`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-3rdparties/rdsd2pcm/Cargo.toml --no-default-features --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
