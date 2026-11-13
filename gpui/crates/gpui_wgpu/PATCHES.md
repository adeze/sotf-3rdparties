# Local patches on top of zed v1.9.0

This vendored `gpui_wgpu` crate carries the following local delta. Keep this
file current when refreshing the upstream renderer.

## 1. `PrimitiveBatch::Custom` zero-copy dispatch (MeshPlot, 2026-08-09)

Adds the WGPU platform side of GPUI's custom-draw registry. Registered
`WgpuCustomDrawAdapter` values are resolved in scene order and record directly
into the renderer's command encoder and current frame texture. The renderer
ends its normal pass before each custom batch and resumes it afterward; custom
draws must not submit the encoder themselves.

- **`src/custom.rs` (new)**: `WgpuCustomDraw` and
  `WgpuCustomDrawAdapter`.
- **`src/wgpu_renderer.rs`**: dispatches `PrimitiveBatch::Custom`, skips
  missing or non-WGPU registrations, and hands the shared `WgpuContext` to
  matching draws.
- **`src/gpui_wgpu.rs`**: exports the custom-draw API from the crate root.

## 2. `draw_wgpu` `target_format` param + capability probe init (vello 2D charts, 2026-08-16)

Extends the custom-draw seam for the vello 2D chart backend: implementations
need the frame texture's format to build compositing pipelines, and elements
need to know whether custom draws dispatch at all before choosing GPU vs CPU
rasterization.

- **`src/custom.rs`**: `WgpuCustomDraw::draw_wgpu` gains a
  `target_format: wgpu::TextureFormat` parameter after `target`.
- **`src/wgpu_renderer.rs`**: the `PrimitiveBatch::Custom` dispatch site passes
  `self.surface_config.format` as `target_format`; `new_internal` calls
  `gpui::set_wgpu_custom_draw_available(has_context)` gated on
  `gpu_context.is_some()` — native `new()` passes `Some(..)` and advertises
  custom draw, wasm `new_from_canvas` passes `None` and leaves the probe false
  (its dispatch arm skips custom batches anyway when `self.context` is `None`).

## 3. `draw_wgpu` `full_bounds` param + premultiplied sprite shader fix (vello 2D charts, 2026-08-17)

Custom draws receive scene bounds in device pixels at unit scale factor.
Offscreen-buffer draws (vello 2D charts) need the unclipped element extent to
size that buffer and locate the visible sub-region, and polychrome sprites
must honor GPUI's premultiplied-atlas contract.

- **`src/custom.rs`**: `WgpuCustomDraw::draw_wgpu` gains a
  `full_bounds: Bounds<Pixels>` parameter after `bounds` — the unclipped
  element bounds, while `bounds` stays the content-mask-clipped visible
  region.
- **`src/wgpu_renderer.rs`**: the dispatch site passes
  `custom.bounds` (unclipped) as `full_bounds` alongside the clipped bounds.
- **`src/shaders.wgsl`**: `fs_poly_sprite` no longer routes the atlas sample
  through `blend_color` (which multiplied rgb by alpha a second time on
  premultiplied surfaces, darkening translucent sprite content). It now
  matches the Metal renderer's `polychrome_sprite_fragment`: premultiplied
  sample passes through, only `color.a` is scaled by opacity/edge coverage.

## 4. `PaddedBool32` shader mask simplification (upstream port, 2026-10-04)

Ports the `gpui_wgpu` hunk of upstream zed PR #60482 (merged 2026-07-09):
`fs_underline` / `fs_poly_sprite` compare `wavy` / `grayscale` directly
instead of masking with `& 0xFFu`, since the Rust side now guarantees
padding-free 0-or-1 `u32` values.

## 5. Atlas texture lookup returns `Option` (upstream port, 2026-10-04)

Ports the `gpui_wgpu` hunks of upstream zed PR #64623 (merged 2026-09-22):
`get_texture_info()` returns `Option` and the three sprite draws skip
batches whose texture was released (adapted to this tree's renderer
shape, which predates upstream's bind-group caching; the skip returns
`true`, matching the empty-batch convention). The `Index` impl is kept
because `flush_uploads` still uses it. No test ported: this tree has no
`wgpu_atlas` test module (upstream added asserts to its generation
test).

## 6. Web graphics preparation on local wgpu 29 (Zed v1.25.2 re-vendor, issue #6)

Upstream v1.25.2 `gpui_web` initializes graphics through
`WgpuContext::new_web(canvas, WebBackendPreference) -> PreparedWebGraphics`
(WebGPU with WebGL2 fallback) against wgpu 30. This tree keeps local
`sotf-wgpu` 29 (vello constraint), so the API surface is ported onto the
retained local context instead of taking upstream's `wgpu_context.rs`
wholesale:

- **`src/wgpu_context.rs`**: new `WgpuBackend` enum + `WgpuContext::backend()`;
  `WgpuContext` is now `Clone` (shared `Arc` device/queue, shared
  device-lost flag). The old zero-argument `new_web()` is replaced by the
  upstream-shaped `new_web(canvas, preference)` returning
  `PreparedWebGraphics { context, surface }`: the surface is created before
  adapter selection (`compatible_surface`), `Auto` uses
  `wgpu::util::new_instance_with_webgpu_detection`, and the device-lost
  callback is installed (the old local `new_web` never installed one).
  `WebDisplaySource` implements `raw_window_handle::HasDisplayHandle`
  (wgpu 29 blanket-implements `WgpuHasDisplayHandle`; upstream's wgpu 30
  trait method does not exist here). Local-only behavior is retained:
  `headless()`, timestamp queries (`SOTF_GPU_TIMESTAMPS`), and the
  `device_lost` flag — upstream's `DeviceErrorState` is not ported.
- **`src/wgpu_renderer.rs`**: new `new_from_surface(&WgpuContext, Surface, config)`
  wasm constructor (upstream signature), built on the retained
  `new_internal` with a cloned context in the shared `GpuContext` slot so
  device-lost recovery keeps working. `new_from_canvas` is retained.

## 7. Cosmic text helpers for the web Canvas fallback (upstream port, issue #6)

Upstream v1.25.2 `gpui_web::WebTextSystem` calls two inherent helpers on
`CosmicTextSystem` that this tree's modularized local cosmic system did
not have. Ported from upstream `crates/gpui_wgpu/src/cosmic_text_system.rs`:

- `CosmicTextSystem::font_weight_and_style(font_id)` — selected face's
  actual weight/style from the fontdb face.
- `CosmicTextSystem::missing_glyphs(text, runs, indices)` — grapheme-level
  `MissingGlyph` reports with `FallbackFontClass` from the face's
  `monospaced` flag; implemented on `CosmicTextSystemState` in
  `src/cosmic_text_system/cosmic_text_system_state.rs`.
- New dependency `unicode-segmentation = "1.10"` (same version as the
  `gpui` crate) for `grapheme_indices`.

## 8. `draw` unreachable-tail fix (issue #6 Phase C)

`WgpuRenderer::draw` ended with a `true` that is unreachable in
non-`headless-qa` builds (the interactive render loop only exits via
`return`). The two mutually exclusive cfg blocks now form the tail
expression: the `headless-qa` block evaluates to `true`, the interactive
block's loop diverges. No behavior change; both feature states check
warning-free.
