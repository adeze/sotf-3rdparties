# vst3-sys

Upstream: https://github.com/robbert-vdh/vst3-sys.git. Base: `0.1.0`.
Revision: `b3ff4d775940f5b476b9d1cca02a90e07e1922a2`.
Revision status: recorded.
License: `GPLv3`; preserved files: `com/LICENSE`, `license.md`.
License evidence status: preserved.

## Why retained

Retain the VST3 bindings drop fix used by NIH and the external plugin host.

## Retained changes

Exact fix/drop-box-from-raw branch revision explicitly drops Box::from_raw results; internal COM crates retained.
Read [the existing detail record](../vst3-sys/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `bbf34cd1cb235064f1b34a0d92b67b4b559b70ef5447ea2234cdd0a5540186e9`. Pinned commit subject: Explicitly drop Box::from_raw result.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo check --manifest-path sotf-daw/Cargo.toml -p sotf-host --features external-plugin-vst3
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).

## macOS release validation (2026-10-30)

Workspace strict all-target Clippy, cargo tests/docs and nextest pass (one test across six binaries). Fixed macro helpers, removed expression-position vtable trailing semicolon, documented COM safety contracts and repaired example lints. Example bus-input helper is safe because it only mutates owned RefCell data. Evidence: aggregate tmp/release-qa/vst3-example-fixed.log; private thirdparties#15. Actual hosted-plugin behavior needs consumer QA.
