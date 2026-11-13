# block

Upstream: http://github.com/SSheldon/rust-block. Base: `0.1.6`.
Revision: `not recorded`.
Revision status: exact upstream commit not recorded; retained source commit identifies the imported fork.
License: `MIT`; preserved files: none in original import.
License evidence status: upstream manifest declares MIT; imported crate and upstream repository do not contain a separate license file.

## Why retained

Avoid the uninhabited extern-static future incompatibility in Apple Objective-C blocks.

## Retained changes

Opaque runtime symbol, addr_of!, explicit C ABI, packaged dev-dependency cleanup.
Read [the existing detail record](../block/VENDORING.md) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `gpui-toolkit/crates/3rdparties/block`. Retained source repository/commit: `gpui-toolkit@a145af14bd8d9679487f42c0dfb3f50897668a02`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo check --manifest-path sotf-3rdparties/block/Cargo.toml --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).

## 2026-10-30 release validation

Restored the omitted upstream C interop test helper with provenance recorded in
[VENDORING.md](../block/VENDORING.md). Added Safety headings and explicit types
for existing function-pointer casts. Strict workspace all-target Clippy, tests,
doctests and nextest pass on macOS, including the helper. No ABI or block
ownership behavior changed.
