# rusty-fork

Upstream: https://github.com/pierreaubert/rusty-fork.git. Base: `0.3.1`.
Revision: `ee62bcd64fa6afe7fae0e52a448d716dc294e98d`.
Revision status: recorded.
License: `MIT/Apache-2.0`; preserved files: `LICENSE-APACHE`, `LICENSE-MIT`.
License evidence status: preserved.

## Why retained

Align quick-error to version 2 for the test dependency closure.

## Retained changes

Release maintenance (2026-10-30): mechanical import/borrow/return lint
fixes, current I/O-error constructor in a test, and doc-list indentation.
macOS strict Clippy, tests/doctests and release nextest pass (14 cases).

Pinned fork updates quick-error manifest and macro compatibility.
Read [the existing detail record](../rusty-fork/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `d3090b85a08153c3e23a86a3f7f2c08afd20d31ca1dd729ca075f27b52687b38`. Pinned commit subject: Port error source macro to quick-error 2.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-3rdparties/rusty-fork/Cargo.toml --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
