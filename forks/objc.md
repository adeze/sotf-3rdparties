# objc

Upstream: http://github.com/SSheldon/rust-objc. Base: `0.2.7`.
Revision: `858f92f7c11deb4cd95fe8c8b58ff8e1a6758f5e`.
Revision status: recorded.
License: `MIT`; preserved files: `LICENSE.txt`.
License evidence status: preserved.

## Why retained

Keep the Apple Objective-C runtime bindings compatible with modern Rust and nil message dispatch.

## Retained changes

Explicit C ABI, cfg/lint modernization, addr_of! for nil raw pointers, test cleanup.
Read [the existing detail record](../objc/VENDORING.md) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `gpui-toolkit/crates/3rdparties/objc`. Retained source repository/commit: `gpui-toolkit@a145af14bd8d9679487f42c0dfb3f50897668a02`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo check --manifest-path sotf-3rdparties/objc/Cargo.toml --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).

## 2026-10-30 release validation

Strict default/all-feature Clippy, tests/doctests and nextest pass on macOS:
23 default and 22 all-feature nextest cases, zero skipped. Added existing unsafe
contract documentation, equivalent mechanical style fixes, explicit Rust 2015,
and a feature-specific test import. The arm64 generic dispatcher interface is
intentionally retained with scoped lint expectations: all return types use the
same runtime entry point on that architecture.
