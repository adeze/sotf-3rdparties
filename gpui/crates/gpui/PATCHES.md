# Local patches on top of zed v1.25.2

This vendored `gpui` fork carries the following local deltas. Keep this file
current: every change relative to upstream zed v1.25.2 must be listed here.

## 1. `Primitive::Custom` scene hook + custom draw registry (MeshPlot, 2026-08-09)

Adds an extension point that lets embedders render their own GPU content
inside the GPUI scene graph, in correct paint order, clipped like any other
primitive. Used by the MeshPlot feature; platform renderer crates
(`gpui_wgpu`, `gpui_macos`) dispatch these batches in follow-up patches.

- **`src/custom_draw.rs` (new)**: `CustomDraw` base trait (`as_any` downcast
  hook), `CustomDrawId` (`u64`), and a main-thread (`thread_local!` + `Rc`)
  registry: `register_custom_draw`, `unregister_custom_draw`,
  `lookup_custom_draw`. `lookup_custom_draw` is `pub` (not `pub(crate)`) so the
  separate renderer crates can resolve ids.
- **`src/scene.rs`**: new `Primitive::Custom(CustomPrimitive)` variant with
  `CustomPrimitive { order, id, bounds, content_mask }` (bounds in
  `ScaledPixels`, mirroring the other scene primitives), threaded through
  `Scene` (`custom_primitives` vec, `clear`, `insert_primitive`, `finish`),
  `PrimitiveKind`, `BatchIterator`, and a new `PrimitiveBatch::Custom(range)`
  batch kind so renderers can dispatch at the right point in scene order.
  Includes a `#[cfg(test)]` scene retention/ordering test.
- **`src/window.rs`**: `Window::paint_custom(id, bounds)`, mirroring
  `paint_image` / `paint_surface` (snaps bounds, applies current content mask,
  inserts into the next frame's scene).
- **`src/gpui.rs`** (crate root): `mod custom_draw;` and re-export of
  `CustomDraw`, `CustomDrawId`, `register_custom_draw`,
  `unregister_custom_draw`, `lookup_custom_draw`.

**Known downstream impact**: `gpui_macos::metal_renderer` still matches on
`PrimitiveBatch` exhaustively without a custom dispatch arm until MeshPlot Task
10 lands. The WGPU arm is provided by Task 9, and the Windows renderer has a
minimal no-op arm because it has no WGPU custom-draw backend. `cargo check -p
gpui` itself is clean.

## 2. `WgpuCustomDraw` capability probe (vello 2D charts, 2026-08-16)

Adds a process-wide flag so chart elements can probe whether the active
renderer actually dispatches `WgpuCustomDraw` primitives, and pick GPU vs
CPU rasterization accordingly. Set by `gpui_wgpu::WgpuRenderer` on init
(native only — wasm canvas renderers store no `GpuContext` and silently
skip custom batches, so they leave the flag false).

- **`src/custom_draw.rs`**: `static WGPU_CUSTOM_DRAW_AVAILABLE: AtomicBool`,
  `wgpu_custom_draw_available() -> bool`, and
  `#[doc(hidden)] set_wgpu_custom_draw_available(bool)` (renderer-only, not
  app API). Includes a `#[cfg(test)]` flag roundtrip test.
- **`src/gpui.rs`** (crate root): re-export of `wgpu_custom_draw_available`
  and `set_wgpu_custom_draw_available` alongside the registry functions.

## 3. Nested `ScrollAnchor::scroll_into_view` (keyboard focus, 2026-10-08)

Tracks the anchor's size as well as origin, then adjusts only the vertical
offset of its associated scrollport by the minimum amount needed to reveal the
descendant. It applies the offset synchronously during the key event and
refreshes the window; scheduling the adjustment with `on_next_frame` alone did
not run when focus changed without another frame request. The API is opt-in:
the SOTF Channel Mute/Solo rack buttons attach anchors so Tab and Shift-Tab
keep lower-channel controls visible, while graph-editor buttons have no rack
anchor. Native 12-channel focus and state verification is recorded in
`reviews/ui-plugins-p1-remediation-status.md`.

- **`src/elements/div.rs`**: `ScrollAnchor` stores the descendant size and
  exposes `scroll_into_view`, preserving horizontal scroll and ordinary
  re-render behavior.

## 4. gpui-fast port: bounds grid, path/glyph caches (perf-cache stage 3, 2026-11-07)

Ports [longbridge/gpui-fast](https://github.com/longbridge/gpui-fast)
(import base `957e659`, zed `decd239`) onto this fork, stage by stage, keeping
ported logic under `src/fast/` like upstream-of-the-port does, with small
hooks in shared files. Stage 3 is the standalone kept-work items:

- **`src/fast/bounds_tree.rs`** replaces `src/bounds_tree.rs` (via `#[path]`
  in `gpui.rs`): uniform-grid bounds tree with ordering replay instead of
  an R-tree. One local fix: `rand::RngExt` import in its tests (rand 0.10).
- **`src/fast/path_cache.rs`**: tessellation cache keyed by path shape;
  `path_builder.rs` forwards `build()` to it (fields widened to
  `pub(crate)`).
- **`src/fast/glyphs.rs`**: per-run glyph painter with bounds cache;
  hooks in `window.rs` (`fast_glyph_bounds` field, subpixel quantization,
  frame-end replaced by `finish_retained_frame` in stage 5) and
  `text_system/line.rs` (painter, bounding box, background early-out).
  `Window::paint_custom` also notes its container uncarried (stage 6).

## 5. Retained Taffy layout nodes (perf-cache stage 4a)

Layout nodes survive between frames, keyed by element path; an unchanged
request (style, children, measure identity) writes nothing to Taffy, so
its per-node caches survive and layout cost tracks what moved.

- **`src/fast/layout.rs`, `layout_key.rs`, `layout_bounds.rs`** copied;
  `taffy.rs` and `element.rs` replaced with gpui-fast's versions (sotf had
  no local deltas there; one adaptation: `A11ySubtreeBuilder::new` keeps
  this fork's third `action_listeners` argument).
- **`src/fast/global_id.rs`**: frame-on-frame `GlobalElementId` cache;
  `Window.fast_layout` / `Window.global_ids` fields and frame-end calls in
  `window.rs`; `layout_as_list_item` wraps every row layout in
  `elements/list.rs` / `uniform_list.rs`; `div.rs` requests layout through
  `fast::layout::request_layout`.
- **`src/fast/scene.rs`** (drawing-order sort): `Scene::finish` forwards
  to it; test oracle helpers (`describe`, `forget_orderings`,
  `replay_balanced` — the last moved here from gpui-fast's composition
  module) describe custom primitives and surfaces as well.

## 6. Retained views + splicing + dependency tracking (perf-cache stage 5)

Any view whose dependencies have not changed is drawn from last frame's
output ranges instead of being rendered, laid out, prepainted and painted
again; a changed descendant is spliced into its clean ancestors' output.
`GPUI_VIEW_RETENTION=0` (or `Window::set_view_retention(false)`) restores
upstream's drawing model.

- **`src/fast/{retained,dependencies,splice,a11y,dispatch,tracked_entities,interactivity,text_style}.rs`**
  copied; `view.rs` replaced with gpui-fast's version; `PaintIndex` gains
  `fast::FastPaintIndex` and keeps this fork's `pointer_listeners_index`
  through shift/equality (local additions in `retained.rs`).
- Hooks: `window.rs` (`retained_state`, deferred-draw recording,
  `begin_frame`/`finish_retained_frame`, ambient pointer/key reads,
  `text_style()` via the caching stack), `app.rs` + `app/entity_map.rs`
  (global/entity access recording; `extend_accessed` now takes
  `impl IntoIterator`), `key_dispatch.rs` (bulk subtree copy, `Listeners`
  newtype), `window/a11y.rs` (operation log), `input.rs` (read-only IME
  queries via `dependencies::query`), `elements/div.rs` (group-hover and
  wheel invalidation of enclosing retained subtrees), `elements/list.rs` /
  `div.rs` `StateVersion` on list/scroll state, adapted to this fork's
  list-state shape (`logical_scroll_top` / `pending_scroll` fields in
  `fast/dependencies.rs`). `platform.rs`: `PlatformInputHandler` gains
  `focus_id` (`with_focus`), set at registration in `window.rs`.
- **`src/fast/layers/`** at this stage held conservative stubs for the
  scroll-layer hooks retained views call; stage 7 replaced them.
  `src/fast/composition.rs` is a stub: window composition is not ported,
  so no view is ever drawn in a native surface. One upstream test
  (`randomized_entities_and_work_counters_…`) keeps upstream semantics via
  `fast::retained::upstream_work_counter_test`, exactly as gpui-fast does.

## 7. Scroll layers + list layers (perf-cache stage 6)

`src/fast/layers/*` and `src/fast/rounded_clip*.rs` copied in full;
`Window.fast_layers`, `Scene.layers` / `sort_scratch` / `fast_rounded_clip`,
`PaintOperation::FastRoundedClip`, div `begin_children`/`paint_children`
and wheel/hover-freeze hooks, list/uniform-list row hooks
(`begin_list`, `keeps_row`, `paint_row`, …), `anchored` note, window
dispatch/present-adjacent notes, `hitbox_mask` in `insert_hitbox`,
`set_scroll_layers` test switch. Layer scene contract types are exported
from the crate root (`SceneLayers`, `LayerKey`, `TileCoord`, …).

Local adaptations:

- `Primitive::Custom` arms in the layers' primitive matches
  (`background`, `overlay` rank 8 — customs batch after surfaces here,
  `paint` clip, `scene` move, `tiles` hash, `verify` describe/clip). A
  custom draw's pixels come from outside the scene, so
  `Window::paint_custom` notes its container *uncarried*: tile layers
  never hold one. In list layers a custom primitive replays like any
  other (resolved by id at draw time, bounds moved).
- **Tile gate**: no renderer in this fork rasterizes `Scene.layers` yet.
  `fast::layers::scene::set_tile_layers_supported(bool)` (exported) is a
  renderer opt-in in the style of `set_wgpu_custom_draw_available`;
  `policy::decide` bypasses non-list containers until a renderer sets it.
  List layers replay rows and need no tiles, so they are on. Headless
  tests (`cfg(test)`) always count as supported. Porting a tile
  rasterizer (gpui-fast's `gpui_apple`/`gpui_wgpu` `fast/layers`) lifts
  the gate.
- `PlatformInput::Scroll`/`ScrollEvent` in ported code renamed to this
  fork's `ScrollWheel`/`ScrollWheelEvent`.

## 8. Text carry/replay (perf-cache stage 4b)

`src/fast/{text,number_shaping,em_metrics,interactive_text}.rs` copied;
`elements/text.rs` replaced with gpui-fast's version. A retained text leaf
takes over last frame's measurement when its text measures the same under
every constraint Taffy used, and lines shaped lately are answered from
`RecentShapes` instead of the platform. Generic measure replay in
`fast/layout.rs` is on (`true`), as upstream-of-the-port: routing text
through carried measurements is what makes replay sound here (before 4b
it was locally disabled, because this fork's then-current text measure
closure left state replay could disagree with — the retained oracle
caught it).

Local adaptations in `fast/text.rs`: this fork's font cache stores
`Result<FontId>` (no direct/fallback distinction), so
`resolve_font_uncached`/`cached_font_id` are rewritten over it (fallback
results are not cached under the asked-for font); `carry_over_line_layouts`
also carries the fork's by-hash caches; the duplicate
`LineLayoutIndex::shifted`/`PartialEq` were dropped in favour of the
by-hash-extended ones in `line_layout.rs` (its `Debug` was extended);
`shape_line_uncached` clones out of the `Arc<LineLayout>` this fork's
`PlatformTextSystem::layout_line` returns. Hooks: `text_system.rs`
(`fonts_changed` on `add_fonts`, `resolve_font` forwarding, em metrics,
`decoration_runs`), `text_system/line_layout.rs` (`shaping` field,
`FrameCache` fields `pub(crate)`, `finish_frame` carry-over, shaping on
cache miss in both keyed and hashed paths).

**Tests**: gpui-fast's `fast/tests/` are ported under `src/fast/tests/`
(all but `dynamic_textures`, whose feature is not ported), adapted for
rand 0.10 (`RngExt`), `ScrollWheelEvent`, and `Arc<LineLayout>` fakes.
723 lib tests pass (4 ignored, all pre-existing/upstream).
