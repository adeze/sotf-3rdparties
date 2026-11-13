# stacker

Upstream: https://github.com/pierreaubert/stacker.git. Base: `0.1.25`.
Revision: `48bf8e3abfb350de9387a675275a629e4daf9717`.
Revision status: recorded.
License: `MIT OR Apache-2.0`; preserved files: `LICENSE-APACHE`, `LICENSE-MIT`, `psm/LICENSE-APACHE`, `psm/LICENSE-MIT`.
License evidence status: preserved.

## Why retained

Retain psm assembly compatibility for watchOS and visionOS Mach-O targets.

## Retained changes

Pinned fork modifies target assembly directives; psm and supporting stacker workspace retained.
Read [the existing detail record](../stacker/psm/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `5a7e93b3e25da8fac35679949a6a6f69b55dc18366c9688ae397409c079a0e54`. Pinned commit subject: Support watchOS and visionOS Mach-O assembly.

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

Removed identity multiplication from the test stack-size constant without changing its value. Strict all-target Clippy, cargo tests and nextest pass on macOS: six nextest tests, zero skipped.
