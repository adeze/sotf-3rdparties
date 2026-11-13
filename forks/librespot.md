# librespot

Upstream: https://github.com/librespot-org/librespot. Base: `0.8.0`.
Revision: `not recorded`.
Revision status: exact upstream commit not recorded; retained source commit identifies the imported fork.
License: `MIT`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Maintain Spotify integration against the selected crypto and XML dependency families.

## Retained changes

Crypto API/dependency maintenance and quick-xml ProductInfo normalization; complete workspace retained.
Read [the existing detail record](../librespot/README.md) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `sotf/crates/3rdparties/librespot`. Retained source repository/commit: `sotf@a228c40da803c6d0c18ed0e22139878a3c58701f`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf/Cargo.toml -p sotf-player --lib --features spotify
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).

## macOS release validation (2026-10-30)

Default native strict all-target Clippy, workspace tests/docs and nextest pass (30 cases, zero skipped). Current vergen-gitcl 10.0.3 aligns build metadata types and requires Rust 1.96 (workspace minimum updated); Build/Gitcl builders preserve short SHA and date output. Equivalent condition/iterator lint repairs across core, audio, metadata, playback, connect and CLI. Evidence: aggregate tmp/release-qa/librespot-final-gates.log; private thirdparties#15. No authenticated playback tested. Consumer Git dependencies need adoption separately.
