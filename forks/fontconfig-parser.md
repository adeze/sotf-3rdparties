# fontconfig-parser

Upstream: https://github.com/pierreaubert/fontconfig-parser.git. Base: `0.5.8`.
Revision: `d247c91f5a57edd6ba6eb26e884d4904dbfbc87e`.
Revision status: recorded.
License: `MIT`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Accept roxmltree 0.21 in the Linux font discovery dependency closure.

## Retained changes

Release maintenance (2026-10-30): Criterion updated to 0.8.2; mechanical
Clippy fixes preserve enum defaults and path behavior. The property macro
marks only Family as its derived default. macOS strict Clippy,
tests/doctests and release nextest pass (10 cases).

Pinned fork manifest updates roxmltree compatibility.
Read [the existing detail record](../fontconfig-parser/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `9fb45b21214b691f2ad93cdc83e26d607217f187600fdafe5031e99fe939a0a6`. Pinned commit subject: Allow roxmltree 0.21 in fontconfig parser.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-3rdparties/fontconfig-parser/Cargo.toml --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
