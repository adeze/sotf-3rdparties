# Vendored: gpui_web

- Upstream: https://github.com/zed-industries/zed/tree/v1.25.2/crates/gpui_web
- Base ref: v1.25.2
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, reqwest_client, zlog, ztracing, ztracing_macro

## Zed v1.25.2 re-vendor deltas (issue #6, 2026-10-22)

The three-way re-vendor took upstream v1.25.2 `gpui_web` as the base.
Deltas that remain local after the merge:

- `src/canvas_size.rs` + the `contentRect` logical-size rule in
  `src/window.rs` are retained (sotf-3rdparties#1, below); upstream
  `src/viewport.rs` (`WebViewport`) is used alongside for visual-viewport
  and safe-area reporting.
- `src/text_system.rs::layout_line` returns `Arc<LineLayout>` (unwrap the
  native `Arc`, apply Canvas fallback replacements, re-wrap) because this
  fork's `gpui::PlatformTextSystem::layout_line` returns `Arc<LineLayout>`
  where upstream returns an owned `LineLayout`. The
  `font_weight_and_style` / `missing_glyphs` helpers it calls on
  `CosmicTextSystem` were ported into `gpui_wgpu` (see its PATCHES.md §7).
- Clipboard follows upstream: synchronous `read_from_clipboard` returns
  `None`, real reads go through `read_from_clipboard_async`, writes are
  fire-and-forget `navigator.clipboard.write_text`. The pre-1.25.2 local
  in-memory clipboard cache (`self.clipboard`) was dropped.
- Graphics init uses upstream `PreparedWebGraphics` flow; the
  wgpu-29-compatible side lives in `gpui_wgpu` (see its PATCHES.md §6).
- `Drop for WebWindow` is upstream's (cancels rAF, disconnects observers,
  breaks the DPR media-query Rc cycle, removes canvas, drops the IME
  mirror, marks the lifecycle closed). The pre-1.25.2 local Drop and its
  hidden input element are gone; upstream uses `ime_mirror` instead.
- Crate-root `#![allow(clippy::unnecessary_lazy_evaluations)]` in
  `src/gpui_web.rs` (issue #6 Phase C, zero-warnings gate; upstream code
  unchanged).

The "Local patches" and "Deliberately not ported" sections below describe
the pre-1.25.2 (v1.9.0-based) state. Several items listed there (e.g. the
#61707 callback rework, the WebGPU-failure DOM message) are upstream-native
at v1.25.2 and arrived with the re-vendor; treat those sections as history
until the per-hunk audit (issue #6, GAP-14) rewrites them.

## Local patches

- `src/window.rs` ports the reentrancy-safe callback invocation from upstream
  zed PR #61707 (commit c97b7c0ea4): new `WebWindowInner::with_callback`
  (take/call/restore so `callbacks` is not borrowed while GPUI code runs),
  applied to the rAF `request_frame`, ResizeObserver `resize` (both call
  sites), `active_status_change`, and `appearance_changed` invocations;
  plus `raf_id` tracking and a `Drop for WebWindow` that cancels the pending
  rAF, disconnects the ResizeObserver, breaks the DPR media-query Rc cycle,
  and removes the canvas/input elements — otherwise a dropped window's pending
  callbacks throw "closure invoked after being dropped".
- `src/dispatcher.rs` ports the `MainThreadMailbox::run_waker_loop` fixes from
  the same commit: the `waitAsync` result is now only awaited when
  `is_async` is true (a synchronous "not-equal" return is a benign race, not
  a fatal error that kills the waker loop), and the mailbox is drained again
  after re-arming the signal with `Atomics::store(0)` so items posted between
  the previous drain and the re-arm are not lost. Also ported from the same
  file: `shared_memory_supported` is no longer `cfg`-gated on the
  `multithreaded` feature, and the `supports_threads` selection uses
  `cfg!(feature = "multithreaded")` so the SharedArrayBuffer warning is only
  emitted when threading was actually requested.
- `src/window.rs` no longer derives the logical (CSS px) window size as
  `devicePixelContentBoxSize / devicePixelRatio` (sotf-3rdparties#1): under
  DevTools DPR emulation the box tracks the real display while the ratio
  reports the emulated value, which produced a garbage logical size and a
  too-small (blurry) canvas backing store. The logical size now always comes
  from `contentRect` (CSS px on every browser) and new
  `src/canvas_size.rs::resolve_physical_size` keeps the browser-reported
  device pixels when they agree with `round(css * dpr)`, falling back to the
  computed size on divergence. Upstream `zed/main` still mixes the two
  sources, so there is no upstream commit to port yet. `src/gpui_web.rs`
  gates each `wasm`-only module individually so the dependency-free
  `canvas_size` module (with host unit tests) also compiles under
  `cfg(test)`; the crate is otherwise unchanged on other targets.

## Deliberately not ported (from upstream zed PR #61707, commit c97b7c0ea4)

- `src/platform.rs` WebGPU-failure DOM message (`show_webgpu_unavailable_message`,
  replaces `on_finish_launching` on init failure) — not ported, follow-up.
- `src/window.rs` `fullscreenchange` listener / `toggle_fullscreen` rework —
  not ported: the vendored `toggle_fullscreen` flips a local flag, so an
  Esc-exit desyncs the stale flag; follow-up.
- `src/window.rs` clamped logical size hunk (recompute logical size from the
  clamped physical size) — not ported; only matters beyond
  `max_texture_dimension_2d`.
- `src/window.rs` `handle` field removal — not ported; the field stays with a
  harmless `#[allow(dead_code)]`.
- `src/events.rs` `EventListenerHandle` rework (drop-unregisters listeners,
  removes the `smallvec` dependency) — not ported; the leak only matters on
  window drop.

## Deliberately not ported (other upstream changes)

- The on-demand `frame_waker` rework (upstream PR #62327) was deliberately
  not ported: it requires `PlatformWindow` trait changes in gpui core and is
  a power optimization, not a correctness fix.
