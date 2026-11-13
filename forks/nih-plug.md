# nih-plug

Upstream: https://github.com/robbert-vdh/nih-plug. Base: `de421011f41a6d10fc8c7a6084e4f4dee0143683`.
Revision: `de421011f41a6d10fc8c7a6084e4f4dee0143683`.
Revision status: recorded.
License: `ISC`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Preserve native plugin automation, transport, state return, tail bounds and auxiliary bus contracts.

## Retained changes

Detailed timing, state, tail and routing fixes are recorded in the fork README; no GUI re-vendor.
Read [the existing detail record](../nih-plug/README.md) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `sotf-daw/crates/3rdparties/nih-plug`. Retained source repository/commit: `sotf-daw@bbb0a8b924bbb368631f0d6a7426af25e66878f6`.

## Validation and removal

macOS release validation (2026-10-30): workspace strict all-target Clippy,
serial cargo tests/doctests, and 106 process-isolated nextest cases pass.
Run cargo tests with `-- --test-threads=1`: the existing no-heap admission
probe uses process-wide allocation counters. Repairs cover compiler lints,
state-test comparisons, feature gating for the macOS event loop, and rejection
of malformed host state without debug-build panics. The intentional allocating
admission probe remains allocating. Private issue:
http://192.168.1.32:3001/pierre/sotf-3rdparties/issues/17.
Evidence: aggregate `tmp/release-qa/nih-plug-state-fixed.log`.
Local fork changes are not yet adopted by consumer Git dependencies.

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-daw/Cargo.toml -p plugins-nih --no-default-features --features gain --lib wrapper::transport
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
