# baseview

Upstream: https://github.com/RustAudio/baseview.git. Base: `0.1.0`.
Revision: `9a0b42c09d712777b2edb4c5e0cb6baf21e988f0`.
Revision status: recorded.
License: `MIT OR Apache-2.0`; preserved files: `LICENSE-APACHE`, `LICENSE-MIT`.
License evidence status: preserved.

## Why retained

NIH-plug's optional standalone backend pins baseview
from git, and crates.io forbids git dependencies, so the pinned source is
vendored here for `sotf-baseview` publication.

## Retained changes

Exact pinned snapshot; no SOTF Rust patch. SOTF manifest maintenance only:
crates.io description/repository added (description from the upstream registry
text), `.github` workflows omitted as non-source files, package renamed
`sotf-baseview` for publication. The pre-existing pin (nih-plug's
`579130e`) declares version `0.1.0`; the newer
`9a0b42c` macOS event-deferral revision is kept for the NIH-plug standalone backend.
The `objc` dependency stays on the upstream registry release: baseview mixes
`objc` types with `cocoa` types from crates.io, and a renamed fork cannot
share that type universe (see `RELEASE.md`).
Read [the existing detail record](../baseview/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `a82325d13bbd97e14e6b15098e1e21d9540176e6dc5dce805b1d8085b98ef2a6`. Pinned commit subject: defer certain events on macOS to avoid re-entrant calls (#189).

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo check --manifest-path sotf-3rdparties/baseview/Cargo.toml --lib
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-08;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
