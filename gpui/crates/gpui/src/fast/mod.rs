//! Performance work ported from gpui-fast (https://github.com/longbridge/gpui-fast),
//! kept in one module tree so upstream files only carry small hooks into it.
//! See `crates/gpui/PATCHES.md` for the stage-by-stage inventory.

pub(crate) mod a11y;
pub(crate) mod composition;
pub(crate) mod dependencies;
pub(crate) mod dispatch;
pub(crate) mod em_metrics;
pub(crate) mod global_id;
pub(crate) mod glyphs;
pub(crate) mod interactive_text;
pub(crate) mod interactivity;
pub(crate) mod layers;
pub(crate) mod layout;
pub(crate) mod layout_bounds;
pub(crate) mod layout_key;
pub(crate) mod path_cache;
pub(crate) mod retained;
pub(crate) mod rounded_clip;
// `rounded_clip_fixtures.rs` is shared test code included by renderer
// crates via `#[path]` (as in gpui-fast); it is not a module here.
pub(crate) mod rounded_clip_scene;
pub(crate) mod scene;
pub(crate) mod splice;
pub(crate) mod stats;
pub(crate) mod text_style;
pub(crate) mod tracked_entities;

pub(crate) mod number_shaping;
#[cfg(test)]
mod tests;
pub(crate) mod text;
