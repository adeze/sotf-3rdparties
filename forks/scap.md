# scap

Upstream: https://github.com/zed-industries/scap. Base: `0.0.8-zed`.
Revision: `4afea48c3b002197176fb19cd0f9b180dd36eaac`.
Revision status: recorded.
License: `MIT`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Retain the Zed screen capture fork used by GPUI platform backends.

## Retained changes

Exact selected Zed fork with macOS compilation fixes; platform support still requires target checks.
Read [the existing detail record](../scap/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `8cd50e118ee2e7b2aed8af011a7de588908eb55816336ce650f28cc452c19b33`. Pinned commit subject: Get macos version compiling.

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

Strict all-target Clippy, tests/doctests and nextest pass (three cases, zero skipped); registry block future-compatibility warning remains. Equivalent From conversion, explicit borrowed lifetimes, and direct typed pointer cast replace linted forms. Evidence: aggregate tmp/release-qa/scap-fixed.log; private thirdparties#15. No screen capture started.
