# nalgebra

Upstream: https://github.com/pierreaubert/nalgebra.git. Base: `0.35.0`.
Revision: `a42573dbc9649af62fb07b469c98c852e62720fd`.
Revision status: recorded.
License: `Apache-2.0`; preserved files: `LICENSE`, `nalgebra-glm/LICENSE`, `nalgebra-lapack/LICENSE`, `nalgebra-macros/LICENSE`, `nalgebra-sparse/LICENSE`.
License evidence status: preserved.

## Why retained

Use the supported glam 0.34 conversion without retaining older conversion dependencies.

## Retained changes

Pinned fork removes unsupported older glam conversion manifest/source entries.
The release update advances the sole conversion dependency to glam 0.34.1 and
renames `convert-glam033` to `convert-glam034`, reusing the common conversion
implementations. No aggregate workspace manifest selected the old feature.
Conversion-enabled validation is tracked in private issue
http://192.168.1.32:3001/pierre/sotf-3rdparties/issues/34; the earlier default-feature
passes below do not validate this update.
Read [the existing detail record](../nalgebra/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `abad57157995caf98e6365d5f0c6b8890f49d7575576d82c80a3db59f0512ecf`. Pinned commit subject: Keep only supported glam 0.33 conversion.

## Validation and removal

The glam 0.34.1 update passed macOS strict all-target Clippy, 225 reference
tests, 509 doctests (two existing ignored examples), the no-default-features
`libm,convert-glam034` compile, and 225 nextest cases with zero skips.
Two conversion regressions cover matrix ordering/transformation and vector
precision/range. Enabling the conversion path exposed two lint-only repairs:
remove a redundant static lifetime and unnecessary private visibility qualifier.
Evidence: `tmp/release-qa/nalgebra-glam034-fixed-gates.log` in the aggregate
checkout. Shipping-source adoption still requires committing the collection
revision and updating consumer Git pins.

macOS release validation (2026-10-30): strict all-target Clippy, cargo tests and
doctests, explicit nalgebra-macros tests, and 221 nextest cases pass. Repairs
remove equivalent lint patterns, load benchmark macros once, retain single
evaluation and temporary borrowing in stack expansions, and refresh two reviewed
compile-fail diagnostic snapshots for the current compiler. Private issue:
http://192.168.1.32:3001/pierre/sotf-3rdparties/issues/16.
Logs: `tmp/release-qa/nalgebra-verified.log` and
`tmp/release-qa/nalgebra-diagnostics-fixed.log` in the aggregate checkout.
These local changes have not yet been adopted by consumer Git dependencies.

From the aggregate `all_of_sotf` directory:

```sh
cargo check --manifest-path sotf-daw/Cargo.toml -p sotf-plugins
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
