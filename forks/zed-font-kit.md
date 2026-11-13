# zed-font-kit

Upstream: https://github.com/servo/font-kit. Base: `94b0f28166665e8fd2f53ff6d268a14955c82269`.
Revision: `110523127440aefb11ce0cf280ae7c5071337ec5`.
Revision status: recorded.
License: `MIT OR Apache-2.0`; preserved files: `LICENSE-APACHE`, `LICENSE-MIT`.
License evidence status: preserved.

## Why retained

Preserve Apple target support, bitmap expansion, and CSS generic font family behavior.

## Retained changes

Apple mobile cfg/manifest fixes, canvas/source changes; remaining deltas need classification as documented.
Read [the existing detail record](../zed-font-kit/VENDORING.md) before updating.
This inventory review does not claim a fresh upstream parity or security audit.

## Source provenance

Import kind: `retained-local-fork`.
Original path: `gpui-toolkit/crates/3rdparties/zed-font-kit`. Retained source repository/commit: `gpui-toolkit@a145af14bd8d9679487f42c0dfb3f50897668a02`.

## Validation and removal

From the aggregate `all_of_sotf` directory:

```sh
cargo test --manifest-path sotf-3rdparties/zed-font-kit/Cargo.toml --lib canvas
```

These are update gates, not a statement that all platform-specific tests ran for
this relocation. Use the owning consumer's feature set and locked resolution.
Replace this source only when a selected upstream release covers the retained behavior and the listed consumer regression checks pass.

Owner: SOTF dependency maintainers. Inventory reviewed 2026-10-07;
re-evaluate the upstream delta within 90 days and on every update.
Follow [the collection update procedure](../README.md#updating-a-fork).

## macOS release repair (2026-10-30)

Strict all-target Clippy fixes preserve bitmap order and font loading behavior. Tests/examples/docs now reference the renamed zed_font_kit library. Restored resources/tests unchanged from zed-industries/font-kit revision 94b0f28166665e8fd2f53ff6d268a14955c82269, including font licenses. Source archive SHA-256: 902a9466b39fdcaa4a428b487474c4107028cbc92a8d2eee281193879f25f7a0. These fixtures were absent from the normalized package (which excludes resources). Private tracking: sotf-3rdparties#12. Validation evidence is recorded in aggregate release-progress.md.
