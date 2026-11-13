# rust-assert-no-alloc

Upstream: https://github.com/robbert-vdh/rust-assert-no-alloc.git. Base: `1.1.2`.
Revision: `a6fb4f62b9624715291e320ea5f0f70e73b035cf`.
Revision status: recorded.
License: `BSD-1-Clause`; preserved files: `LICENSE`.
License evidence status: preserved.

## Why retained

Preserve correct nested assert_no_alloc and permit_alloc behavior in native realtime tests.

## Retained changes

Exact feature/nested-permit-forbid branch revision fixes nested allocation guard behavior.
Read [the existing detail record](../rust-assert-no-alloc/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `54226d203836d1f73d12df9c468b9ed1e541aba3e8d119c962cbc07da1ef36de`. Pinned commit subject: Fix nested assert_no_alloc and permit_alloc.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-daw/Cargo.toml -p plugins-nih --no-default-features --features gain --lib gui_state_return_tests
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).

## 2026-10-30 release validation

Strict all-target Clippy, tests/doctests and nextest pass with the upstream
required warn_debug feature: nine nextest tests, zero skipped. Const TLS
initializers and equivalent assertion/tail-expression syntax preserve guard
behavior. The layout argument remains used in abort configurations. Example
vector pushes intentionally demonstrate allocation and carry a scoped lint
expectation. Documentation examples now compile and use concrete values/imports.
