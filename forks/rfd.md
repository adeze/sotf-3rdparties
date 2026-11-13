# rfd

Upstream: https://github.com/pierreaubert/rfd.git. Base: `0.17.2`.
Revision: `b049aa9c12c6cf5ac72a4ea0dffdeed6f7427f0d`.
Revision status: recorded.
License: `MIT`; preserved files: `LICENSE`, `src/backend/xdg_desktop_portal/window_identifier/LICENSE`.
License evidence status: preserved.

## Why retained

Align optional pollster with the selected version 1 family.

## Retained changes

Pinned fork commit updates pollster manifest compatibility; imported Rust is unchanged by relocation.
Read [the existing detail record](../rfd/Cargo.toml) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `pinned-git-snapshot`.
Archive SHA-256: `339c614a4ace42f576b4918c384182ac69b8c8ef4c510957b0cd4a32c41c1f51`. Pinned commit subject: Align optional pollster dependency with version 1.

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

## macOS release validation (2026-10-30)

Strict all-target Clippy, cargo tests/doctests and nextest pass (one nextest case, two doctests). Updated CFUserNotification::display_alert spelling and removed compiler-confirmed redundant unsafe blocks for current objc2 APIs. Matched optional parent window directly, preserving fallback. No dialogs launched, so visual/interaction validation remains a consumer check. Evidence: aggregate tmp/release-qa/rfd-native-fixed.log; private sotf-3rdparties#13.
