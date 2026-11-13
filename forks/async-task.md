# async-task

Upstream: https://github.com/smol-rs/async-task.git. Base: `4.7.1`.
Revision: `b4486cd71e4e94fbda54ce6302444de14f4d190e`.
Revision status: recorded.
License: `Apache-2.0 OR MIT`; preserved files: `LICENSE-APACHE`, `LICENSE-MIT`.
License evidence status: preserved.

## Why retained

Pin the compatible upstream async task implementation used by the GPUI dependency closure.

## Retained changes

Exact upstream snapshot; no SOTF Rust patch claimed. Pinned head updates a flume development dependency.
Read [the existing detail record](../async-task/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `74de925ea04c6e119a343691db231f83ca4c728c827a95c58f0304810ef96000`. Pinned commit subject: Update flume requirement from 0.11 to 0.12 (#99).

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

Release validation uses nightly for upstream unstable benchmark harnesses: strict all-target Clippy, all-target tests, doctests and nextest pass on macOS. Stable all-target Clippy cannot compile the upstream benchmark feature(test).
