# gpui

Upstream: https://github.com/zed-industries/zed. Base: `Zed v1.25.2 plus pinned support fork`.
Revision: `v1.25.2`.
Revision status: recorded.
License: `See individual GPUI/Zed package and font licenses`; preserved files: `LICENSE-APACHE`, `assets/fonts/ibm-plex-sans/license.txt`, `crates/bench_metrics/LICENSE-APACHE`, `crates/collections/LICENSE-APACHE`, `crates/gpui/LICENSE-APACHE`, `crates/gpui_apple/LICENSE-APACHE`, `crates/gpui_linux/LICENSE-APACHE`, `crates/gpui_macos/LICENSE-APACHE`, `crates/gpui_macros/LICENSE-APACHE`, `crates/gpui_shared_string/LICENSE-APACHE`, `crates/gpui_util/LICENSE-APACHE`, `crates/gpui_web/LICENSE-APACHE`, `crates/gpui_wgpu/LICENSE-APACHE`, `crates/gpui_windows/LICENSE-APACHE`, `crates/http_client/LICENSE-APACHE`, `crates/path/LICENSE-APACHE`, `crates/perf/LICENSE-APACHE`, `crates/refineable/LICENSE-APACHE`, `crates/refineable/derive_refineable/LICENSE-APACHE`, `crates/scheduler/LICENSE-APACHE`, `crates/sum_tree/LICENSE-APACHE`, `crates/util/LICENSE-APACHE`, `crates/util_macros/LICENSE-APACHE`, `previous-notes/LICENSE-APACHE`.
License evidence status: preserved.

## Why retained

Own the patched GPUI framework, renderer, platform backends and Zed support sources in one self-contained group.

## Retained changes

Fourteen retained toolkit vendor crates plus seven exact compatibility-fork support crates; former workspace policies and embedded fonts retained. Per-crate VENDORED/VENDORING notes describe the behavioral patches. Zed v1.25.2 re-vendor tracked in issue #6; see gpui/UPGRADE-v1.25.2-PLAN.md.
Read [the existing detail record](../gpui/README.md) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `gpui-toolkit/crates/3rdparties`. Retained source repository/commit: `gpui-toolkit@a145af14bd8d9679487f42c0dfb3f50897668a02`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo check --manifest-path sotf/Cargo.toml -p sotf-gpui --features shipping
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-22;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).
