//! Utilities for saving a [`crate::plugin::Plugin`]'s state. The actual state object is also exposed
//! to plugins through the [`GuiContext`][crate::prelude::GuiContext].

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::params::ParamMut;
use crate::prelude::{BufferConfig, Param, ParamPtr, Params, Plugin};

// These state objects are also exposed directly to the plugin so it can do its own internal preset
// management

/// A plain, unnormalized value for a parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamValue {
    F32(f32),
    I32(i32),
    Bool(bool),
    /// Only used for enum parameters that have the `#[id = "..."]` attribute set.
    String(String),
}

/// A plugin's state so it can be restored at a later point. This object can be serialized and
/// deserialized using serde.
///
/// The fields are stored as `BTreeMap`s so the order in the serialized file is consistent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    /// The plugin version this state was saved with. Right now this is not used, but later versions
    /// of NIH-plug may allow you to modify the plugin state object directly before it is loaded to
    /// allow migrating plugin states between breaking parameter changes.
    ///
    /// # Notes
    ///
    /// If the saved state is very old, then this field may be empty.
    #[serde(default)]
    pub version: String,

    /// The plugin's parameter values. These are stored unnormalized. This means the old values will
    /// be recalled when when the parameter's range gets increased. Doing so may still mess with
    /// parameter automation though, depending on how the host implements that.
    pub params: BTreeMap<String, ParamValue>,
    /// Arbitrary fields that should be persisted together with the plugin's parameters. Any field
    /// on the [`Params`][crate::params::Params] struct that's annotated with `#[persist =
    /// "stable_name"]` will be persisted this way.
    ///
    /// The individual fields are also serialized as JSON so they can safely be restored
    /// independently of the other fields.
    pub fields: BTreeMap<String, String>,
}

/// Create a parameters iterator from the hashtables stored in the plugin wrappers. This avoids
/// having to call `.param_map()` again, which may include expensive user written code.
pub(crate) fn make_params_iter<'a>(
    param_by_hash: &'a HashMap<u32, ParamPtr>,
    param_id_to_hash: &'a HashMap<String, u32>,
) -> impl IntoIterator<Item = (&'a String, ParamPtr)> {
    param_id_to_hash.iter().filter_map(|(param_id_str, hash)| {
        let param_ptr = param_by_hash.get(hash)?;
        Some((param_id_str, *param_ptr))
    })
}

/// Create a getter function that gets a parameter from the hashtables stored in the plugin by
/// string ID.
pub(crate) fn make_params_getter<'a>(
    param_by_hash: &'a HashMap<u32, ParamPtr>,
    param_id_to_hash: &'a HashMap<String, u32>,
) -> impl Fn(&str) -> Option<ParamPtr> + 'a {
    |param_id_str| {
        param_id_to_hash
            .get(param_id_str)
            .and_then(|hash| param_by_hash.get(hash))
            .copied()
    }
}

/// Serialize a plugin's state to a state object. This is separate from [`serialize_json()`] to
/// allow passing the raw object directly to the plugin. The parameters are not pulled directly from
/// `plugin_params` by default to avoid unnecessary allocations in the `.param_map()` method, as the
/// plugin wrappers will already have a list of parameters handy. See [`make_params_iter()`].
pub(crate) unsafe fn serialize_object<'a, P: Plugin>(
    plugin_params: Arc<dyn Params>,
    params_iter: impl IntoIterator<Item = (&'a String, ParamPtr)>,
) -> PluginState {
    // We'll serialize parameter values as a simple `string_param_id: display_value` map.
    // NOTE: If the plugin is being modulated (and the plugin is a CLAP plugin in Bitwig Studio),
    //       then this should save the values without any modulation applied to it
    let mut params: BTreeMap<_, _> = params_iter
        .into_iter()
        .map(|(param_id_str, param_ptr)| match param_ptr {
            ParamPtr::FloatParam(p) => (
                param_id_str.clone(),
                ParamValue::F32((*p).unmodulated_plain_value()),
            ),
            ParamPtr::IntParam(p) => (
                param_id_str.clone(),
                ParamValue::I32((*p).unmodulated_plain_value()),
            ),
            ParamPtr::BoolParam(p) => (
                param_id_str.clone(),
                ParamValue::Bool((*p).unmodulated_plain_value()),
            ),
            ParamPtr::EnumParam(p) => (
                // Enums are either serialized based on the active variant's index (which may not be
                // the same as the discriminator), or a custom set stable string ID. The latter
                // allows the variants to be reordered.
                param_id_str.clone(),
                match (*p).unmodulated_plain_id() {
                    Some(id) => ParamValue::String(id.to_owned()),
                    None => ParamValue::I32((*p).unmodulated_plain_value()),
                },
            ),
        })
        .collect();

    params.extend(plugin_params.serialize_parameter_overrides());

    // The plugin can also persist arbitrary fields alongside its parameters. This is useful for
    // storing things like sample data.
    let fields = plugin_params.serialize_fields();

    PluginState {
        version: String::from(P::VERSION),
        params,
        fields,
    }
}

/// Serialize a plugin's state to a vector containing JSON data. This can (and should) be shared
/// across plugin formats. If the `zstd` feature is enabled, then the state will be compressed using
/// Zstandard.
pub(crate) unsafe fn serialize_json<'a, P: Plugin>(
    plugin_params: Arc<dyn Params>,
    params_iter: impl IntoIterator<Item = (&'a String, ParamPtr)>,
) -> Result<Vec<u8>> {
    let plugin_state = serialize_object::<P>(plugin_params, params_iter);
    let json = serde_json::to_vec(&plugin_state).context("Could not format as JSON")?;

    #[cfg(feature = "zstd")]
    {
        let compressed = zstd::encode_all(json.as_slice(), zstd::DEFAULT_COMPRESSION_LEVEL)
            .context("Could not compress state")?;

        let state_bytes = json.len();
        let compressed_state_bytes = compressed.len();
        let compression_ratio = compressed_state_bytes as f32 / state_bytes as f32 * 100.0;
        nih_trace!(
            "Compressed {state_bytes} bytes of state to {compressed_state_bytes} bytes \
             ({compression_ratio:.1}% compression ratio)"
        );

        Ok(compressed)
    }
    #[cfg(not(feature = "zstd"))]
    {
        Ok(json)
    }
}

/// Deserialize a plugin's state from a [`PluginState`] object. This is used to allow the plugin to
/// do its own internal preset management. Returns `false` and logs an error if the state could not
/// be deserialized.
///
/// This uses a parameter getter function to avoid having to rebuild the parameter map, which may
/// include expensive user written code. See [`make_params_getter()`].
///
/// Make sure to reinitialize plugin after deserializing the state so it can react to the new
/// parameter values. The smoothers have already been reset by this function.
///
/// The [`Plugin`] argument is used to call [`Plugin::filter_state()`] just before loading the
/// state.
pub(crate) unsafe fn deserialize_object<P: Plugin>(
    state: &mut PluginState,
    plugin_params: Arc<dyn Params>,
    params_getter: impl Fn(&str) -> Option<ParamPtr>,
    current_buffer_config: Option<&BufferConfig>,
    is_active: bool,
    is_audio_thread: bool,
) -> bool {
    // Audio-thread restores must not reach allocating migration, validation,
    // or field work. Refuse before `filter_state` runs so the live
    // parameters and DSP state stay untouched; the host retries on a
    // control thread. Control-thread restores always proceed.
    if is_audio_thread && !P::state_restore_allows_audio_thread(state) {
        return false;
    }

    // This lets the plugin perform migrations on old state if needed
    P::filter_state(state);

    // Plugins that persist control-thread resources must be able to reject an
    // unsupported restore before any host-visible parameter is mutated. This
    // also lets them refuse work that would otherwise be deferred to the end
    // of an audio callback while the plugin is active.
    if !plugin_params.validate_state(
        state,
        is_active,
        is_audio_thread,
        current_buffer_config.map(|config| config.sample_rate),
    ) {
        return false;
    }

    let apply_parameters_now = !plugin_params.defer_state_parameter_values();
    if apply_parameters_now {
        if let Some(config) = current_buffer_config {
            // Preflight every parameter before changing any value or persisted field. A
            // restore resets smoothers rather than starting a timed ramp, so their
            // validated step counts do not need to be retained or allocated here.
            for param_id in state.params.keys() {
                if let Some(param) = params_getter(param_id) {
                    if param.prepare_smoother(config.sample_rate).is_none() {
                        return false;
                    }
                }
            }
        }
    }

    if apply_parameters_now {
        for (param_id_str, param_value) in &state.params {
            let param_ptr = match params_getter(param_id_str.as_str()) {
                Some(ptr) => ptr,
                None => {
                    nih_debug_assert_failure!("Unknown parameter: {}", param_id_str);
                    continue;
                }
            };

            match (param_ptr, param_value) {
                (ParamPtr::FloatParam(p), ParamValue::F32(v)) => {
                    (*p).set_plain_value(*v);
                }
                (ParamPtr::IntParam(p), ParamValue::I32(v)) => {
                    (*p).set_plain_value(*v);
                }
                (ParamPtr::BoolParam(p), ParamValue::Bool(v)) => {
                    (*p).set_plain_value(*v);
                }
                // Enums are either serialized based on the active variant's index (which may not be the
                // same as the discriminator), or a custom set stable string ID. The latter allows the
                // variants to be reordered.
                (ParamPtr::EnumParam(p), ParamValue::I32(variant_idx)) => {
                    (*p).set_plain_value(*variant_idx);
                }
                (ParamPtr::EnumParam(p), ParamValue::String(id)) => {
                    let deserialized_enum = (*p).set_from_id(id);
                    nih_debug_assert!(
                        deserialized_enum,
                        "Unknown ID {:?} for enum parameter \"{}\"",
                        id,
                        param_id_str,
                    );
                }
                (param_ptr, param_value) => {
                    nih_debug_assert_failure!(
                        "Invalid serialized value {:?} for parameter \"{}\" ({:?})",
                        param_value,
                        param_id_str,
                        param_ptr,
                    );
                }
            }

            // Make sure everything starts out in sync
            if current_buffer_config.is_some() {
                param_ptr.update_smoother_prepared(1, true);
            }
        }
    }

    // The plugin can also persist arbitrary fields alongside its parameters. This is useful for
    // storing things like sample data.
    plugin_params.deserialize_fields(&state.fields);

    true
}

/// Deserialize a plugin's state from a vector containing (compressed) JSON data. Doesn't load the
/// plugin state since doing so should be accompanied by calls to `Plugin::init()` and
/// `Plugin::reset()`, and this way all of that behavior can be encapsulated so it can be reused in
/// multiple places. The returned state object can be passed to [`deserialize_object()`].
pub(crate) unsafe fn deserialize_json(state: &[u8]) -> Option<PluginState> {
    // Host-provided state may be malformed; reject it without a debug-build panic.
    #[cfg(feature = "zstd")]
    let result: Option<PluginState> = match zstd::decode_all(state) {
        Ok(decompressed) => match serde_json::from_slice(decompressed.as_slice()) {
            Ok(s) => {
                let state_bytes = decompressed.len();
                let compressed_state_bytes = state.len();
                let compression_ratio = compressed_state_bytes as f32 / state_bytes as f32 * 100.0;
                nih_trace!(
                    "Inflated {compressed_state_bytes} bytes of state to {state_bytes} bytes \
                     ({compression_ratio:.1}% compression ratio)"
                );

                Some(s)
            }
            Err(err) => {
                nih_warn!("Error while deserializing state: {}", err);
                None
            }
        },
        // Uncompressed state files can still be loaded after enabling this feature to prevent
        // breaking existing plugin instances
        Err(zstd_err) => match serde_json::from_slice(state) {
            Ok(s) => {
                nih_trace!("Older uncompressed state found");
                Some(s)
            }
            Err(json_err) => {
                nih_warn!(
                    "Error while deserializing state as either compressed or uncompressed state: \
                     {}, {}",
                    zstd_err,
                    json_err
                );
                None
            }
        },
    };

    #[cfg(not(feature = "zstd"))]
    let result: Option<PluginState> = match serde_json::from_slice(state) {
        Ok(s) => Some(s),
        Err(err) => {
            nih_warn!("Error while deserializing state: {}", err);
            None
        }
    };

    result
}

#[cfg(test)]
mod admission_tests {
    //! Audio-thread state-restore admission: the pre-`filter_state` gate.
    //!
    //! These tests use dependency-free dummy plugins, so they build wherever
    //! the crate builds. The heap-scope test overrides the global allocator
    //! with a process-wide counter and must run single-threaded
    //! (`cargo test -p nih_plug -- --test-threads=1`); the behavioral tests
    //! are race-free under parallel execution. The heap test covers the
    //! `deserialize_object` gate only; the full audio handoff includes a
    //! `try_recv` that may allocate (documented `FIXME`), and control-thread
    //! JSON parsing may also allocate.

    use super::{deserialize_json, deserialize_object, ParamValue, PluginState};
    use crate::params::smoothing::SmoothingStyle;
    use crate::prelude::{
        AudioIOLayout, AuxiliaryBuffers, BoolParam, Buffer, FloatParam, FloatRange, IntParam,
        IntRange, Param, ParamPtr, Params, Plugin, ProcessContext, ProcessMode, ProcessStatus,
    };
    #[cfg(not(feature = "assert_process_allocs"))]
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::collections::BTreeMap;
    #[cfg(not(feature = "assert_process_allocs"))]
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct DummyParams;

    // SAFETY: no parameter pointers are exposed.
    unsafe impl Params for DummyParams {
        fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
            Vec::new()
        }
    }

    fn dummy_params() -> Arc<dyn Params> {
        Arc::new(DummyParams)
    }

    fn empty_state() -> PluginState {
        PluginState {
            version: String::from("test"),
            params: BTreeMap::new(),
            fields: BTreeMap::new(),
        }
    }

    /// Populated parameters with allocating probes. Tracks whether validation
    /// and field restore ran; both allocate when they run.
    struct PopulatedParams {
        float: FloatParam,
        int: IntParam,
        flag: BoolParam,
        validate_calls: AtomicUsize,
        deserialize_calls: AtomicUsize,
    }

    impl PopulatedParams {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                float: FloatParam::new(
                    "Float",
                    0.5f32,
                    FloatRange::Linear {
                        min: 0.0f32,
                        max: 1.0f32,
                    },
                )
                .with_smoother(SmoothingStyle::Linear(1_000.0)),
                int: IntParam::new("Int", 1, IntRange::Linear { min: 0, max: 3 }),
                flag: BoolParam::new("Flag", true),
                validate_calls: AtomicUsize::new(0),
                deserialize_calls: AtomicUsize::new(0),
            })
        }
    }

    // SAFETY: pointers borrow fields of the Arc-kept object, which outlives the test calls.
    unsafe impl Params for PopulatedParams {
        fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
            vec![
                (
                    String::from("float"),
                    ParamPtr::FloatParam(&self.float as *const _),
                    String::from("Group"),
                ),
                (
                    String::from("int"),
                    ParamPtr::IntParam(&self.int as *const _),
                    String::from("Group"),
                ),
                (
                    String::from("flag"),
                    ParamPtr::BoolParam(&self.flag as *const _),
                    String::from("Group"),
                ),
            ]
        }

        fn validate_state(
            &self,
            _state: &PluginState,
            _is_active: bool,
            _is_audio_thread: bool,
            _sample_rate: Option<f64>,
        ) -> bool {
            self.validate_calls.fetch_add(1, Ordering::SeqCst);
            // Allocating probe: must not run on audio refusal.
            let probe = std::hint::black_box(Vec::from([1, 2, 3]));
            let _ = probe.len();
            true
        }

        fn deserialize_fields(&self, _serialized: &BTreeMap<String, String>) {
            self.deserialize_calls.fetch_add(1, Ordering::SeqCst);
            // Allocating probe.
            let probe = "deserialize".to_string();
            let _ = probe.len();
        }
    }

    fn populated_state() -> PluginState {
        let mut params = BTreeMap::new();
        params.insert(String::from("float"), ParamValue::F32(0.9));
        params.insert(String::from("int"), ParamValue::I32(2));
        params.insert(String::from("flag"), ParamValue::Bool(false));
        let mut fields = BTreeMap::new();
        fields.insert(String::from("preset"), String::from("kept"));
        PluginState {
            version: String::from("test"),
            params,
            fields,
        }
    }

    fn live_config(sample_rate: f64) -> super::BufferConfig {
        super::BufferConfig {
            sample_rate,
            min_buffer_size: None,
            max_buffer_size: 512,
            process_mode: ProcessMode::Realtime,
        }
    }

    #[test]
    fn live_audio_restore_keeps_existing_valid_preset_behavior() {
        let params = PopulatedParams::new();
        let mut state = populated_state();
        let map = params.param_map();
        let getter = |id: &str| map.iter().find(|(key, _, _)| key == id).map(|(_, p, _)| *p);
        let config = live_config(48_000.0);
        // SAFETY: the Arc owns all pointers in `map` for this call.
        let restored = unsafe {
            deserialize_object::<CompatPlugin>(
                &mut state,
                params.clone(),
                getter,
                Some(&config),
                true,
                true,
            )
        };
        assert!(restored);
        assert_eq!(params.float.value(), 0.9);
        assert_eq!(params.int.value(), 2);
        assert!(!params.flag.value());
        assert_eq!(params.float.smoothed.steps_left(), 0);
        assert_eq!(params.float.smoothed.previous_value(), 0.9);
        assert_eq!(params.deserialize_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn invalid_live_horizon_refuses_before_any_active_preset_write() {
        let params = PopulatedParams::new();
        let mut state = populated_state();
        let map = params.param_map();
        let getter = |id: &str| map.iter().find(|(key, _, _)| key == id).map(|(_, p, _)| *p);
        let config = live_config(2_147_483_648.0);
        // SAFETY: the Arc owns all pointers in `map` for this call.
        let restored = unsafe {
            deserialize_object::<CompatPlugin>(
                &mut state,
                params.clone(),
                getter,
                Some(&config),
                true,
                true,
            )
        };
        assert!(!restored);
        assert_eq!(params.float.value(), 0.5);
        assert_eq!(params.int.value(), 1);
        assert!(params.flag.value());
        assert_eq!(params.float.smoothed.steps_left(), 0);
        assert_eq!(params.deserialize_calls.load(Ordering::SeqCst), 0);
    }

    /// A plugin that keeps the default admission hook and records migration
    /// by annotating the state object. An empty parameter map keeps the
    /// parameter loop (which reports unknown IDs) out of the picture.
    #[derive(Default)]
    struct CompatPlugin;

    impl Plugin for CompatPlugin {
        const NAME: &'static str = "compat admission probe";
        const VENDOR: &'static str = "test";
        const URL: &'static str = "https://example.invalid";
        const EMAIL: &'static str = "test@example.invalid";
        const VERSION: &'static str = "0.0.0";
        const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];

        type SysExMessage = ();
        type BackgroundTask = ();

        fn params(&self) -> Arc<dyn Params> {
            dummy_params()
        }

        fn process(
            &mut self,
            _buffer: &mut Buffer,
            _aux: &mut AuxiliaryBuffers,
            _context: &mut impl ProcessContext<Self>,
        ) -> ProcessStatus {
            ProcessStatus::Normal
        }

        fn filter_state(state: &mut PluginState) {
            state
                .fields
                .insert(String::from("probe_migrated"), String::from("1"));
        }
    }

    /// A plugin that refuses every audio-thread restore. Its `filter_state`
    /// annotates the state so a test can prove the gate ran first.
    #[derive(Default)]
    struct RefusingPlugin;

    impl Plugin for RefusingPlugin {
        const NAME: &'static str = "refusing admission probe";
        const VENDOR: &'static str = "test";
        const URL: &'static str = "https://example.invalid";
        const EMAIL: &'static str = "test@example.invalid";
        const VERSION: &'static str = "0.0.0";
        const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];

        type SysExMessage = ();
        type BackgroundTask = ();

        fn params(&self) -> Arc<dyn Params> {
            dummy_params()
        }

        fn process(
            &mut self,
            _buffer: &mut Buffer,
            _aux: &mut AuxiliaryBuffers,
            _context: &mut impl ProcessContext<Self>,
        ) -> ProcessStatus {
            ProcessStatus::Normal
        }

        fn filter_state(state: &mut PluginState) {
            state
                .fields
                .insert(String::from("probe_migrated"), String::from("1"));
        }

        fn state_restore_allows_audio_thread(state: &PluginState) -> bool {
            let _ = state;
            false
        }
    }

    #[test]
    fn default_hook_preserves_audio_thread_restore() {
        let mut state = empty_state();
        // SAFETY: the dummy exposes no parameters, so the getter is sound.
        let restored = unsafe {
            deserialize_object::<CompatPlugin>(
                &mut state,
                dummy_params(),
                |_| None,
                None,
                false,
                true,
            )
        };
        assert!(restored);
        assert_eq!(
            state.fields.get("probe_migrated").map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn refusing_hook_rejects_before_filter_state_without_mutation() {
        let mut state = empty_state();
        state
            .fields
            .insert(String::from("preset"), String::from("kept"));
        let fields_before = state.fields.clone();
        // SAFETY: the dummy exposes no parameters, so the getter is sound.
        let restored = unsafe {
            deserialize_object::<RefusingPlugin>(
                &mut state,
                dummy_params(),
                |_| None,
                None,
                false,
                true,
            )
        };
        assert!(!restored);
        assert!(
            !state.fields.contains_key("probe_migrated"),
            "filter_state must not run after an audio-thread refusal"
        );
        assert_eq!(state.fields, fields_before);
        assert!(state.params.is_empty());
    }

    #[test]
    fn refusing_hook_allows_control_thread_retry() {
        let mut state = empty_state();
        // SAFETY: the dummy exposes no parameters, so the getter is sound.
        let restored = unsafe {
            deserialize_object::<RefusingPlugin>(
                &mut state,
                dummy_params(),
                |_| None,
                None,
                false,
                false,
            )
        };
        assert!(restored);
        assert_eq!(
            state.fields.get("probe_migrated").map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn refusing_hook_leaves_populated_params_and_fields_untouched() {
        let params = PopulatedParams::new();
        let mut state = populated_state();
        let params_before = state.params.clone();
        let fields_before = state.fields.clone();
        let map = params.param_map();
        let getter = |id: &str| {
            map.iter()
                .find(|(key, _, _)| key == id)
                .map(|(_, ptr, _)| *ptr)
        };
        // SAFETY: params Arc outlives the call; getter returns its live pointers.
        let restored = unsafe {
            deserialize_object::<RefusingPlugin>(
                &mut state,
                params.clone(),
                getter,
                None,
                false,
                true,
            )
        };
        assert!(!restored);
        // No filter_state, validation, param write, or field restore ran.
        assert_eq!(
            serde_json::to_value(&state.params).unwrap(),
            serde_json::to_value(&params_before).unwrap(),
        );
        assert_eq!(state.fields, fields_before);
        assert!(!state.fields.contains_key("probe_migrated"));
        assert_eq!(params.validate_calls.load(Ordering::SeqCst), 0);
        assert_eq!(params.deserialize_calls.load(Ordering::SeqCst), 0);
        // Live values untouched: initial 0.5/1/true, not state 0.9/2/false.
        assert_eq!(params.float.unmodulated_plain_value(), 0.5f32);
        assert_eq!(params.int.unmodulated_plain_value(), 1);
        assert!(params.flag.unmodulated_plain_value());
    }

    #[test]
    fn control_retry_applies_populated_state_after_audio_refusal() {
        let params = PopulatedParams::new();
        let mut state = populated_state();
        let map = params.param_map();
        // Audio refusal first; ownership is retained for retry.
        {
            let getter = |id: &str| {
                map.iter()
                    .find(|(key, _, _)| key == id)
                    .map(|(_, ptr, _)| *ptr)
            };
            // SAFETY: params Arc outlives the call.
            let refused = unsafe {
                deserialize_object::<RefusingPlugin>(
                    &mut state,
                    params.clone(),
                    getter,
                    None,
                    false,
                    true,
                )
            };
            assert!(!refused);
        }
        // Control retry applies the same retained object.
        let getter = |id: &str| {
            map.iter()
                .find(|(key, _, _)| key == id)
                .map(|(_, ptr, _)| *ptr)
        };
        // SAFETY: params Arc outlives the call.
        let restored = unsafe {
            deserialize_object::<RefusingPlugin>(
                &mut state,
                params.clone(),
                getter,
                None,
                false,
                false,
            )
        };
        assert!(restored);
        assert_eq!(
            state.fields.get("probe_migrated").map(String::as_str),
            Some("1")
        );
        assert_eq!(params.validate_calls.load(Ordering::SeqCst), 1);
        assert_eq!(params.deserialize_calls.load(Ordering::SeqCst), 1);
        assert_eq!(params.float.unmodulated_plain_value(), 0.9f32);
        assert_eq!(params.int.unmodulated_plain_value(), 2);
        assert!(!params.flag.unmodulated_plain_value());
    }

    #[test]
    fn malformed_raw_state_is_rejected_before_restore() {
        // SAFETY: pure parsing, no plugin callbacks involved.
        assert!(unsafe { deserialize_json(b"\x00\x01not json{{") }.is_none());
        assert!(unsafe { deserialize_json(b"null") }.is_none());
        let encoded = serde_json::to_vec(&empty_state()).expect("state serializes");
        // SAFETY: pure parsing, no plugin callbacks involved.
        let parsed = unsafe { deserialize_json(&encoded) };
        assert!(parsed.is_some());
    }

    // The crate already installs `assert_no_alloc::AllocDisabler` as the
    // global allocator when `assert_process_allocs` is enabled, so this
    // counting allocator (and its test) only exists without that feature.
    #[cfg(not(feature = "assert_process_allocs"))]
    struct CountingAllocator;

    #[cfg(not(feature = "assert_process_allocs"))]
    static COUNTING: AtomicBool = AtomicBool::new(false);
    #[cfg(not(feature = "assert_process_allocs"))]
    static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    #[cfg(not(feature = "assert_process_allocs"))]
    static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

    // SAFETY: forwards to the system allocator; the counters are lock-free.
    #[cfg(not(feature = "assert_process_allocs"))]
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if COUNTING.load(Ordering::SeqCst) {
                ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
            }
            // SAFETY: layout comes from the caller, as required.
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            if COUNTING.load(Ordering::SeqCst) {
                DEALLOCATIONS.fetch_add(1, Ordering::SeqCst);
            }
            // SAFETY: ptr and layout come from the caller, as required.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    #[cfg(not(feature = "assert_process_allocs"))]
    #[global_allocator]
    static GLOBAL: CountingAllocator = CountingAllocator;

    #[cfg(not(feature = "assert_process_allocs"))]
    #[test]
    fn audio_thread_refusal_performs_no_heap_work() {
        // Process-wide counters: this test is exact only when the test
        // binary runs single-threaded (`--test-threads=1`). A populated
        // state with allocating Params probes is the meaningful input: any
        // validation, param write, or field restore would allocate and fail.
        let mut state = populated_state();
        let params = PopulatedParams::new();
        let params_for_call: Arc<dyn crate::prelude::Params> = params.clone();
        let map = params.param_map();
        let getter = |id: &str| {
            map.iter()
                .find(|(key, _, _)| key == id)
                .map(|(_, ptr, _)| *ptr)
        };
        COUNTING.store(true, Ordering::SeqCst);
        let allocations_before = ALLOCATIONS.load(Ordering::SeqCst);
        let deallocations_before = DEALLOCATIONS.load(Ordering::SeqCst);
        // SAFETY: params Arc outlives the call; getter returns its live pointers.
        let restored = unsafe {
            deserialize_object::<RefusingPlugin>(
                &mut state,
                params_for_call,
                getter,
                None,
                false,
                true,
            )
        };
        let allocations = ALLOCATIONS.load(Ordering::SeqCst) - allocations_before;
        let deallocations = DEALLOCATIONS.load(Ordering::SeqCst) - deallocations_before;
        COUNTING.store(false, Ordering::SeqCst);
        assert!(!restored);
        assert_eq!(
            (allocations, deallocations),
            (0, 0),
            "audio-thread refusal must not touch the heap"
        );
        assert_eq!(params.validate_calls.load(Ordering::SeqCst), 0);
        assert_eq!(params.deserialize_calls.load(Ordering::SeqCst), 0);
    }
}
