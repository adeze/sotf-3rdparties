# Shared third-party sources

This repository owns the patched and pinned third-party sources used by the
sibling Rust workspaces in `all_of_sotf`, including Librespot and the GPUI
framework/platform/support closure. SOTF-owned toolkit crates remain in
`gpui-toolkit`. There is no root Cargo
workspace: consumers use paths to individual manifests, while imported upstream
workspaces retain their internal layout.

[`FORKS.json`](FORKS.json) is the maintenance inventory. Each entry identifies the
upstream source/base, exact revision when known, retained source commit, licenses,
reason, local delta, owner, update checks, and removal condition. Missing upstream
commit information is explicit; a local source commit is not an upstream revision.
Per-fork notes below complement the original upstream README, licenses, manifests,
and existing `VENDORING.md` / `SOTF_FORK_PROVENANCE.json` records, which are preserved.

| Source | Why retained |
| --- | --- |
| [async-task](forks/async-task.md) | Pin the compatible upstream async task implementation used by the GPUI dependency closure. |
| [baseview](forks/baseview.md) | Supply the pinned baseview revision required by NIH-plug standalone without git dependencies. |
| [block](forks/block.md) | Avoid the uninhabited extern-static future incompatibility in Apple Objective-C blocks. |
| [coreaudio-rs](forks/coreaudio-rs.md) | Preserve CoreAudio input buffer allocation capacity and initialize WatchOS callback buffers correctly. |
| [cosmic-text](forks/cosmic-text.md) | Retain the GPUI text renderer snapshot compatible with fontdb 0.24 and its selected bidi behavior. |
| [derive_more](forks/derive_more.md) | Keep the macro facade aligned with the maintained implementation and case-conversion dependency. |
| [derive_more-impl](forks/derive_more-impl.md) | Maintain convert_case 0.11 compatibility without changing the public derive behavior. |
| [dst-decoder](forks/dst-decoder.md) | Supply DST decoding for SOTF SACD integration with retained fixture and lint fixes. |
| [fontconfig-parser](forks/fontconfig-parser.md) | Accept roxmltree 0.21 in the Linux font discovery dependency closure. |
| [gpui](forks/gpui.md) | Own the patched GPUI framework, renderer, platform backends and Zed support sources in one self-contained group. |
| [headers](forks/headers.md) | Keep the HTTP header dependency on the SHA-1 0.11 crypto family used by the maintained streaming sources. |
| [librespot](forks/librespot.md) | Maintain Spotify integration against the selected crypto and XML dependency families. |
| [metaheuristics-nature-rs](forks/metaheuristics-nature-rs.md) | Keep seeded optimization generators compatible with Rand 0.10 and Rand Distr 0.6. |
| [nalgebra](forks/nalgebra.md) | Use the supported glam 0.33 conversion without retaining older conversion dependencies. |
| [nih-plug](forks/nih-plug.md) | Preserve native plugin automation, transport, state return, tail bounds and auxiliary bus contracts. |
| [nnnoiseless](forks/nnnoiseless.md) | Retain the RNNoise denoising implementation used by SOTF with fixed-size chunk and current dependency compatibility. |
| [objc](forks/objc.md) | Keep the Apple Objective-C runtime bindings compatible with modern Rust and nil message dispatch. |
| [oo7](forks/oo7.md) | Keep Linux keyring crypto dependencies compatible with the selected RustCrypto family. |
| [predicates](forks/predicates.md) | Consolidate float-cmp dependencies while preserving the modern f64 comparison behavior used by tests. |
| [rdsd2pcm](forks/rdsd2pcm.md) | Provide DSD-to-PCM conversion without forcing file conversion dependencies into the DSP-only path. |
| [rfd](forks/rfd.md) | Align optional pollster with the selected version 1 family. |
| [rubato](forks/rubato.md) | SOTF resampling needs prepared cutoff tables and exact variable-ratio output sizing. |
| [rust-assert-no-alloc](forks/rust-assert-no-alloc.md) | Preserve correct nested assert_no_alloc and permit_alloc behavior in native realtime tests. |
| [rusty-fork](forks/rusty-fork.md) | Align quick-error to version 2 for the test dependency closure. |
| [scap](forks/scap.md) | Retain the Zed screen capture fork used by GPUI platform backends. |
| [stacker](forks/stacker.md) | Retain psm assembly compatibility for watchOS and visionOS Mach-O targets. |
| [tflitec](forks/tflitec.md) | Supply the pinned tflitec revision required by the tract test harness without git dependencies. |
| [tract](forks/tract.md) | Keep ONNX random operators compatible with rand 0.10 and deterministic replay tests. |
| [vello](forks/vello.md) | Keep Vello GPU types on the same renamed local wgpu package as GPUI. |
| [vst3-sys](forks/vst3-sys.md) | Retain the VST3 bindings drop fix used by NIH and the external plugin host. |
| [wgpu](forks/wgpu.md) | Keep Vello and GPUI on one Zed WGPU 29 dependency family including its EGL fix. |
| [xim-rs](forks/xim-rs.md) | Retain the Zed XIM fork used by the Linux GPUI backend. |
| [zed-font-kit](forks/zed-font-kit.md) | Preserve Apple target support, bitmap expansion, and CSS generic font family behavior. |

`rdsd2pcm/src/dsd_reader` moves with its parent and retains its own MIT/Apache
licenses; `rdsd2pcm` itself remains GPL-3.0-or-later. Multi-package sources such as
WGPU, Tract and Librespot retain their workspace manifests and supporting files.

## Updating a fork

1. Start from the exact recorded upstream revision or registry archive. Recover
   missing upstream commit information before claiming parity. Keep a pristine
   reference outside the active checkout for comparison.
2. Read the fork note and retained provenance. Diff the selected upstream source
   against this copy, including Cargo manifests and features. Reapply only changes
   still required; keep behavioral fixes separate from dependency compatibility.
3. Preserve all upstream license/copyright files and attribution. Record the new
   revision/archive digest, patch reason, exact delta, and removal condition in
   `FORKS.json` and the corresponding note. Do not claim unknown upstreaming status.
4. Update every consumer path/patch table together. Root Cargo patches are not
   inherited across sibling workspaces. Preserve existing dependency versions
   when refreshing locks for a source relocation; review any unexpected upgrades.
5. Run `python3 sotf-3rdparties/scripts/verify.py` from the aggregate directory,
   the fork's listed checks, and affected SOTF consumer checks on the relevant
   target. Native/streaming tests that need accounts or audio devices are separate
   explicit acceptance checks. Compile success alone does not prove UI/audio behavior.
6. Record commands, target, results and limitations in the change review. Update
   the aggregate source pin only after the collection revision is committed.

Review upstream deltas at least every 90 days and on each update. Remove a patch
once the selected upstream source supplies equivalent behavior and the consumer
regressions pass. Pinned but otherwise unchanged snapshots are identified as such;
they do not imply a SOTF source patch.

## Relocation

The original shared Rubato/derive/predicate sources came from `math-audio`.
The remaining local forks came from SOTF, SOTF DAW, GPUI Toolkit and Symphonia
Add-ons; pinned Git forks were imported at their existing revisions from local
Cargo caches. The relocation preserves Rust source bytes. Only `dst-decoder`'s
inherited edition/anyhow and `nnnoiseless`'s inherited criterion were made explicit
so those crates no longer rely on their former workspace parents.
Migration integrity and consumer acceptance are recorded in
[`VALIDATION.md`](VALIDATION.md).

## Lint output in SOTF builds

SOTF's `just` build/check/clippy/run/test recipes (including `just gpui`) use
`../scripts/cargo/quiet-thirdparty-lints.py` on macOS and Linux. It reads Cargo's
JSON diagnostics and hides warnings only when every primary source span resolves
beneath this collection. This also filters warnings replayed from Cargo's cache;
no clean or rebuild is needed. First-party warnings, all errors, build-script
messages, and application/test output remain visible. Compiler lint levels and
imported sources are unchanged. The script requires Python 3.9 or newer.

Direct Cargo commands retain full diagnostics for fork maintenance. To apply
the same filtering manually from `sotf`, use:

```sh
python3 ../scripts/cargo/quiet-thirdparty-lints.py cargo check
```

Existing `RUSTC_WRAPPER`/sccache settings are untouched. An explicit
`--message-format` is respected and bypasses filtering. Cargo subcommands without
compiler diagnostics and Windows recipes retain their existing behavior.
