# headers

Upstream: https://github.com/hyperium/headers. Base: `0.4.3`.
Revision: `50bfa1e1a184a6328c108edabe3d1d347d3d0e27`.
Revision status: recorded.
License: `MIT`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Keep the HTTP header dependency on the SHA-1 0.11 crypto family used by the maintained streaming sources.

## Retained changes

Release maintenance (2026-10-30): base64 updated to 0.23.1; added
CacheControl::default and Cookie::is_empty and simplified the entity-tag
match guard for strict Clippy. macOS Clippy, tests/doctests and release
nextest pass (115 cases).

sha1 dependency updated from 0.10 to 0.11; compare Cargo.toml.orig and src/common/authorization.rs.
Read [the existing detail record](../headers/Cargo.toml.orig) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `sotf/crates/3rdparties/headers`. Retained source repository/commit: `sotf@a228c40da803c6d0c18ed0e22139878a3c58701f`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-3rdparties/headers/Cargo.toml --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
