//! Tests of what the gpui-fast port adds, kept out of upstream files' test
//! modules. Ported from gpui-fast's `fast/tests/`; only the modules
//! covering dynamic textures and window composition are absent, with the
//! features themselves (see `crates/gpui/PATCHES.md`).

mod a11y;
mod cached_layout;
mod dependencies;
mod dispatch;
mod em_metrics;
mod global_id;
mod interactive_text;
mod layers;
mod layers_lists;
mod layers_oracle;
mod layout;
mod number_shaping;
mod oracle;
mod path_cache;
mod retained;
mod retained_bench;
mod scene_order;
mod scroll_to_bottom;
mod splice;
mod splice_late_layout;
mod support;
mod text;
mod text_shaping;
mod tracked_entities;
mod wheel_notifies;
