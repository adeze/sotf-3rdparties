# tflitec

Upstream: https://github.com/kali/tflitec-rs.git. Base: `0.6.0`.
Revision: `9ceb838839d0481030aa12e95d8cb28f659f7a48`.
Revision status: recorded.
License: `MIT`; preserved files: none in upstream repository.
License evidence status: upstream manifest declares MIT; the pinned upstream revision contains no separate license file.

## Why retained

The tract test harness pins tflitec from git, and crates.io forbids git
dependencies, so the pinned source is vendored here for `sotf-tflitec`
publication. Its only in-collection consumer is the private
`tract/test-rt/test-tflite` harness.

## Retained changes

Package renamed `sotf-tflitec` for publication. Release preparation updates
bindgen to 0.73.2, makes lifetimes explicit, and resolves current strict Rust
lints without suppressing diagnostics. The native macOS build uses Bazel 5
C++ toolchain defaults with the Xcode SDK and Apple linker, disables GNU
start/end-lib, and patches the pinned TensorFlow float abs call for current
libc++. User-provided Bazel environment options retain precedence. Native
build and full test validation are tracked in the aggregate release report.
Read [the existing detail record](../tflitec/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `ade04b55d92199a0e97a45bbe0c9b3308c6d3e8aa2e83cbbc87e676c4b5bf027`. Pinned commit subject: workaround weird uint32_t usage.

## Validation and removal

Native macOS default-feature validation on 2026-10-30 passed strict Clippy,
7 unit tests, 4 doctests, and 7 nextest tests with the checked-in build setup.
Optional XNNPACK configurations remain outside that result. The aggregate
release report contains the commands and log paths.

From the aggregate `all_of_sotf` directory:

```sh
cargo metadata --manifest-path sotf-3rdparties/tflitec/Cargo.toml --no-deps
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-08;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
