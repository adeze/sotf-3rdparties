# wgpu

Upstream: https://github.com/zed-industries/wgpu.git. Base: `29.0.3`.
Revision: `357a0c56e0070480ad9daea5d2eaa83150b79e88`.
Revision status: recorded.
License: `MIT OR Apache-2.0`; preserved files: `LICENSE.APACHE`, `LICENSE.MIT`, `deno_webgpu/LICENSE.md`, `naga-cli/LICENSE.APACHE`, `naga-cli/LICENSE.MIT`, `naga/LICENSE.APACHE`, `naga/LICENSE.MIT`, `wgpu-core/LICENSE.APACHE`, `wgpu-core/LICENSE.MIT`, `wgpu-core/platform-deps/apple/LICENSE.APACHE`, `wgpu-core/platform-deps/apple/LICENSE.MIT`, `wgpu-core/platform-deps/emscripten/LICENSE.APACHE`, `wgpu-core/platform-deps/emscripten/LICENSE.MIT`, `wgpu-core/platform-deps/wasm/LICENSE.APACHE`, `wgpu-core/platform-deps/wasm/LICENSE.MIT`, `wgpu-core/platform-deps/windows-linux-android/LICENSE.APACHE`, `wgpu-core/platform-deps/windows-linux-android/LICENSE.MIT`, `wgpu-hal/LICENSE.APACHE`, `wgpu-hal/LICENSE.MIT`, `wgpu-info/LICENSE.APACHE`, `wgpu-info/LICENSE.MIT`, `wgpu-naga-bridge/LICENSE.APACHE`, `wgpu-naga-bridge/LICENSE.MIT`, `wgpu-types/LICENSE.APACHE`, `wgpu-types/LICENSE.MIT`, `wgpu/LICENSE.APACHE`, `wgpu/LICENSE.MIT`.
License evidence status: preserved.

## Why retained

Keep Vello and GPUI on one Zed WGPU 29 dependency family including its EGL fix.

## Retained changes

Zed fork snapshot with all workspace support crates retained. Release QA
corrects package selectors in xtask, dependency tests, and the CTS launcher
for the SOTF package names. Ray-tracing expected failures are limited to the
reported Apple M3 adapter; the stale Metal subgroup expected failure is
removed after native M4 validation. No backend rendering implementation changes.
Read [the existing detail record](../wgpu/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `3adb74d326d62a197c669fa1342990cd97e8d1be8f04146d6a3549ee63a2a2cd`. Pinned commit subject: Merge branch 'gfx-rs:v29' into v29.

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
