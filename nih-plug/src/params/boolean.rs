//! Simple boolean parameters.

use atomic_float::AtomicF32;
use std::fmt::{Debug, Display};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::internals::ParamPtr;
use super::{Param, ParamFlags, ParamMut};

/// A simple boolean parameter.
pub struct BoolParam {
    /// The field's current value, after monophonic modulation has been applied.
    value: AtomicBool,
    /// The field's current value normalized to the `[0, 1]` range.
    normalized_value: AtomicF32,
    /// The field's value before any monophonic automation coming from the host has been applied.
    /// This will always be the same as `value` for VST3 plugins.
    unmodulated_value: AtomicBool,
    /// The field's value normalized to the `[0, 1]` range before any monophonic automation coming
    /// from the host has been applied. This will always be the same as `value` for VST3 plugins.
    unmodulated_normalized_value: AtomicF32,
    /// A value in `[-1, 1]` indicating the amount of modulation applied to
    /// `unmodulated_normalized_`. This needs to be stored separately since the normalized values are
    /// clamped, and this value persists after new automation events.
    modulation_offset: AtomicF32,
    /// The field's default value.
    default: bool,

    /// Flags to control the parameter's behavior. See [`ParamFlags`].
    flags: ParamFlags,
    /// Optional callback for listening to value changes. The argument passed to this function is
    /// the parameter's new value. This should not do anything expensive as it may be called
    /// multiple times in rapid succession, and it can be run from both the GUI and the audio
    /// thread.
    value_changed: Option<Arc<dyn Fn(bool) + Send + Sync>>,
    /// Opt-in notification of every base/effective update, including echoed values.
    coupled_value_updated: Option<Arc<dyn Fn(bool, bool) + Send + Sync>>,

    /// The parameter's human readable display name.
    name: String,
    /// If this parameter has been marked as polyphonically modulatable, then this will be a unique
    /// integer identifying the parameter. Because this value is determined by the plugin itself,
    /// the plugin can easily map
    /// [`NoteEvent::PolyModulation`][crate::prelude::NoteEvent::PolyModulation] events to the
    /// correct parameter by pattern matching on a constant.
    poly_modulation_id: Option<u32>,
    /// Optional custom conversion function from a boolean value to a string.
    value_to_string: Option<Arc<dyn Fn(bool) -> String + Send + Sync>>,
    /// Optional custom conversion function from a string to a boolean value. If the string cannot
    /// be parsed, then this should return a `None`. If this happens while the parameter is being
    /// updated then the update will be canceled.
    string_to_value: Option<Arc<dyn Fn(&str) -> Option<bool> + Send + Sync>>,
}

impl Display for BoolParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.value(), &self.value_to_string) {
            (v, Some(func)) => write!(f, "{}", func(v)),
            (true, None) => write!(f, "On"),
            (false, None) => write!(f, "Off"),
        }
    }
}

impl Debug for BoolParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // This uses the above `Display` instance to show the value
        if self.value.load(Ordering::Relaxed) != self.unmodulated_value.load(Ordering::Relaxed) {
            write!(f, "{}: {} (modulated)", self.name, self)
        } else {
            write!(f, "{}: {}", self.name, self)
        }
    }
}

// `Params` can not be implemented outside of NIH-plug itself because `ParamPtr` is also closed
impl super::Sealed for BoolParam {}

impl Param for BoolParam {
    type Plain = bool;

    fn name(&self) -> &str {
        &self.name
    }

    fn unit(&self) -> &'static str {
        ""
    }

    fn poly_modulation_id(&self) -> Option<u32> {
        self.poly_modulation_id
    }

    #[inline]
    fn modulated_plain_value(&self) -> Self::Plain {
        self.value.load(Ordering::Relaxed)
    }

    #[inline]
    fn modulated_normalized_value(&self) -> f32 {
        self.normalized_value.load(Ordering::Relaxed)
    }

    #[inline]
    fn unmodulated_plain_value(&self) -> Self::Plain {
        self.unmodulated_value.load(Ordering::Relaxed)
    }

    #[inline]
    fn unmodulated_normalized_value(&self) -> f32 {
        self.unmodulated_normalized_value.load(Ordering::Relaxed)
    }

    #[inline]
    fn default_plain_value(&self) -> Self::Plain {
        self.default
    }

    fn step_count(&self) -> Option<usize> {
        Some(1)
    }

    fn previous_step(&self, _from: Self::Plain, _finer: bool) -> Self::Plain {
        false
    }

    fn next_step(&self, _from: Self::Plain, _finer: bool) -> Self::Plain {
        true
    }

    fn normalized_value_to_string(&self, normalized: f32, _include_unit: bool) -> String {
        let value = self.preview_plain(normalized);
        match (value, &self.value_to_string) {
            (v, Some(f)) => f(v),
            (true, None) => String::from("On"),
            (false, None) => String::from("Off"),
        }
    }

    fn string_to_normalized_value(&self, string: &str) -> Option<f32> {
        let string = string.trim();
        let value = match &self.string_to_value {
            Some(f) => f(string),
            None => Some(string.eq_ignore_ascii_case("true") || string.eq_ignore_ascii_case("on")),
        }?;

        Some(self.preview_normalized(value))
    }

    #[inline]
    fn preview_normalized(&self, plain: Self::Plain) -> f32 {
        if plain {
            1.0
        } else {
            0.0
        }
    }

    #[inline]
    fn preview_plain(&self, normalized: f32) -> Self::Plain {
        normalized > 0.5
    }

    fn flags(&self) -> ParamFlags {
        self.flags
    }

    fn as_ptr(&self) -> ParamPtr {
        ParamPtr::BoolParam(self as *const BoolParam as *mut BoolParam)
    }
}

impl ParamMut for BoolParam {
    fn set_plain_value(&self, plain: Self::Plain) -> bool {
        let unmodulated_value = plain;
        let unmodulated_normalized_value = self.preview_normalized(plain);

        let modulation_offset = self.modulation_offset.load(Ordering::Relaxed);
        let (value, normalized_value) = if modulation_offset == 0.0 {
            (unmodulated_value, unmodulated_normalized_value)
        } else {
            let normalized_value =
                (unmodulated_normalized_value + modulation_offset).clamp(0.0, 1.0);

            (self.preview_plain(normalized_value), normalized_value)
        };

        // REAPER spams automation events with the same value. This prevents callbacks from firing
        // multiple times. This can be problematic when they're used to trigger expensive
        // computations when a parameter changes.
        let old_value = self.value.swap(value, Ordering::Relaxed);
        if let Some(coupled_callback) = &self.coupled_value_updated {
            // The opt-in coupled-control contract publishes incoming base state
            // even when modulation leaves the effective boolean unchanged.
            let old_normalized = self
                .normalized_value
                .swap(normalized_value, Ordering::Relaxed);
            let old_unmodulated = self
                .unmodulated_value
                .swap(unmodulated_value, Ordering::Relaxed);
            let old_unmodulated_normalized = self
                .unmodulated_normalized_value
                .swap(unmodulated_normalized_value, Ordering::Relaxed);
            if value != old_value {
                if let Some(callback) = &self.value_changed {
                    callback(value);
                }
            }
            coupled_callback(unmodulated_value, value);
            // A changed base value must also notify the host/editor when the
            // effective value is held unchanged by modulation. Same-value echoes
            // still invoke the coupled callback, but do not claim a value change.
            return value != old_value
                || old_unmodulated != unmodulated_value
                || old_normalized != normalized_value
                || old_unmodulated_normalized != unmodulated_normalized_value;
        }
        if value != old_value {
            self.normalized_value
                .store(normalized_value, Ordering::Relaxed);
            self.unmodulated_value
                .store(unmodulated_value, Ordering::Relaxed);
            self.unmodulated_normalized_value
                .store(unmodulated_normalized_value, Ordering::Relaxed);
            if let Some(f) = &self.value_changed {
                f(value);
            }

            true
        } else {
            false
        }
    }

    fn set_normalized_value(&self, normalized: f32) -> bool {
        // NOTE: The double conversion here is to make sure the state is reproducible. State is
        //       saved and restored using plain values, and the new normalized value will be
        //       different from `normalized`. This is not necessary for the modulation as these
        //       values are never shown to the host.
        self.set_plain_value(self.preview_plain(normalized))
    }

    fn modulate_value(&self, modulation_offset: f32) -> bool {
        self.modulation_offset
            .store(modulation_offset, Ordering::Relaxed);

        // TODO: This renormalizes this value, which is not necessary
        self.set_plain_value(self.unmodulated_plain_value())
    }

    fn update_smoother(&self, _sample_rate: f64, _init: bool) {
        // Can't really smooth a binary parameter now can you
    }

    fn prepare_smoother(&self, sample_rate: f64) -> Option<i32> {
        (sample_rate.is_finite() && sample_rate > 0.0).then_some(1)
    }

    fn update_smoother_prepared(&self, _steps: i32, _reset: bool) {}
}

impl BoolParam {
    /// Build a new [`BoolParam`]. Use the other associated functions to modify the behavior of the
    /// parameter.
    pub fn new(name: impl Into<String>, default: bool) -> Self {
        Self {
            value: AtomicBool::new(default),
            normalized_value: AtomicF32::new(if default { 1.0 } else { 0.0 }),
            unmodulated_value: AtomicBool::new(default),
            unmodulated_normalized_value: AtomicF32::new(if default { 1.0 } else { 0.0 }),
            modulation_offset: AtomicF32::new(0.0),
            default,

            flags: ParamFlags::default(),
            value_changed: None,
            coupled_value_updated: None,

            name: name.into(),
            poly_modulation_id: None,
            value_to_string: None,
            string_to_value: None,
        }
    }

    /// The field's current plain value, after monophonic modulation has been applied. Equivalent to
    /// calling `param.plain_value()`.
    #[inline]
    pub fn value(&self) -> bool {
        self.modulated_plain_value()
    }

    /// Set a parameter value while committing a successfully prepared plugin
    /// state. This is for control-thread initialization only, never automation.
    #[doc(hidden)]
    pub fn set_plain_value_for_initialization(&self, value: bool) -> bool {
        <Self as ParamMut>::set_plain_value(self, value)
    }

    /// Set a parameter while committing initialized state.
    #[doc(hidden)]
    pub fn set_plain_value_and_reset_smoother_for_initialization(
        &self,
        value: bool,
        sample_rate: f64,
    ) -> bool {
        let changed = <Self as ParamMut>::set_plain_value(self, value);
        <Self as ParamMut>::update_smoother(self, sample_rate, true);
        changed
    }

    /// Disable a coupled control and clear its existing modulation.
    ///
    /// This opt-in operation sets the plain, unmodulated, effective, and normalized
    /// values to false and resets the modulation offset to zero. All stored values
    /// are updated before the existing value-change callback is invoked. The
    /// callback runs only when the effective boolean changes, as with normal
    /// parameter updates. Parameter flags and subsequent modulation remain intact.
    ///
    /// Coupled-control owners must serialize this operation with host parameter
    /// updates, just as they serialize ordinary parameter setters. This operation
    /// does not make multiple parameter writes an atomic transaction.
    ///
    /// Returns whether any stored value or modulation offset changed. There are
    /// no allocations or locks; the owner-provided callback must also be suitable
    /// for the calling thread.
    #[doc(hidden)]
    pub fn disable_for_coupled_control(&self) -> bool {
        let old_offset = self.modulation_offset.swap(0.0, Ordering::Relaxed);
        let old_unmodulated = self.unmodulated_value.swap(false, Ordering::Relaxed);
        let old_unmodulated_normalized = self
            .unmodulated_normalized_value
            .swap(0.0, Ordering::Relaxed);
        let old_normalized = self.normalized_value.swap(0.0, Ordering::Relaxed);
        let old_value = self.value.swap(false, Ordering::Relaxed);

        if old_value {
            if let Some(callback) = &self.value_changed {
                callback(false);
            }
        }

        if let Some(callback) = &self.coupled_value_updated {
            callback(false, false);
        }

        old_value
            || old_unmodulated
            || old_offset != 0.0
            || old_normalized != 0.0
            || old_unmodulated_normalized != 0.0
    }

    /// Enable polyphonic modulation for this parameter. The ID is used to uniquely identify this
    /// parameter in [`NoteEvent::PolyModulation`][crate::prelude::NoteEvent::PolyModulation]
    /// events, and must thus be unique between _all_ polyphonically modulatable parameters. See the
    /// event's documentation on how to use polyphonic modulation. Also consider configuring the
    /// [`ClapPlugin::CLAP_POLY_MODULATION_CONFIG`][crate::prelude::ClapPlugin::CLAP_POLY_MODULATION_CONFIG]
    /// constant when enabling this.
    ///
    /// # Important
    ///
    /// After enabling polyphonic modulation, the plugin **must** start sending
    /// [`NoteEvent::VoiceTerminated`][crate::prelude::NoteEvent::VoiceTerminated] events to the
    /// host when a voice has fully ended. This allows the host to reuse its modulation resources.
    pub fn with_poly_modulation_id(mut self, id: u32) -> Self {
        self.poly_modulation_id = Some(id);
        self
    }

    /// Run a callback whenever this parameter's value changes. The argument passed to this function
    /// is the parameter's new value. This should not do anything expensive as it may be called
    /// multiple times in rapid succession, and it can be run from both the GUI and the audio
    /// thread.
    pub fn with_callback(mut self, callback: Arc<dyn Fn(bool) + Send + Sync>) -> Self {
        self.value_changed = Some(callback);
        self
    }

    /// Observe every base and effective update for an opt-in coupled control.
    ///
    /// The callback receives the unmodulated base value followed by the effective
    /// value. Unlike [`Self::with_callback`], it also observes unchanged effective
    /// values and same-value echoes. Opting in publishes incoming base and
    /// normalized values even when modulation keeps the effective boolean fixed.
    /// Setters report changes to any of these stored values so wrappers can notify
    /// their host/editor. Unchanged echoes invoke this callback but return false.
    ///
    /// Parameter flags, ordinary change callbacks, and future modulation remain
    /// unchanged. The callback can run on the audio thread and must not allocate
    /// or block. Coupled owners must ignore all-false updates to avoid recursion.
    #[doc(hidden)]
    pub fn with_coupled_update_callback(
        mut self,
        callback: Arc<dyn Fn(bool, bool) + Send + Sync>,
    ) -> Self {
        self.coupled_value_updated = Some(callback);
        self
    }

    /// Use a custom conversion function to convert the boolean value to a string.
    pub fn with_value_to_string(
        mut self,
        callback: Arc<dyn Fn(bool) -> String + Send + Sync>,
    ) -> Self {
        self.value_to_string = Some(callback);
        self
    }

    /// Use a custom conversion function to convert from a string to a boolean value. If the string
    /// cannot be parsed, then this should return a `None`. If this happens while the parameter is
    /// being updated then the update will be canceled.
    pub fn with_string_to_value(
        mut self,
        callback: Arc<dyn Fn(&str) -> Option<bool> + Send + Sync>,
    ) -> Self {
        self.string_to_value = Some(callback);
        self
    }

    /// Mark this parameter as a bypass parameter. Plugin hosts can integrate this parameter into
    /// their UI. Only a single [`BoolParam`] can be a bypass parameter, and NIH-plug will add one
    /// if you don't create one yourself. You will need to implement this yourself if your plugin
    /// introduces latency.
    pub fn make_bypass(mut self) -> Self {
        self.flags.insert(ParamFlags::BYPASS);
        self
    }

    /// Mark the parameter as non-automatable. This means that the parameter cannot be changed from
    /// an automation lane. The parameter can however still be manually changed by the user from
    /// either the plugin's own GUI or from the host's generic UI.
    pub fn non_automatable(mut self) -> Self {
        self.flags.insert(ParamFlags::NON_AUTOMATABLE);
        self
    }

    /// Mark this parameter as requiring a plugin restart after its value changes.
    ///
    /// This is intended for structural values that can be changed by the user but cannot be
    /// applied safely to an already prepared DSP instance. The parameter should also be marked
    /// [`non_automatable()`][Self::non_automatable].
    pub fn requires_restart(mut self) -> Self {
        self.flags.insert(ParamFlags::REQUIRES_RESTART);
        self
    }

    /// Hide the parameter in the host's generic UI for this plugin. This also implies
    /// `NON_AUTOMATABLE`. Setting this does not prevent you from changing the parameter in the
    /// plugin's editor GUI.
    pub fn hide(mut self) -> Self {
        self.flags.insert(ParamFlags::HIDDEN);
        self
    }

    /// Don't show this parameter when generating a generic UI for the plugin using one of
    /// NIH-plug's generic UI widgets.
    pub fn hide_in_generic_ui(mut self) -> Self {
        self.flags.insert(ParamFlags::HIDE_IN_GENERIC_UI);
        self
    }
}

#[cfg(test)]
mod coupled_control_tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn disabling_clears_effective_and_unmodulated_values_with_either_offset() {
        for offset in [-1.0, -0.75, 0.0, 0.75, 1.0] {
            let parameter = BoolParam::new("Coupled", true);
            let flags = parameter.flags();
            parameter.modulate_value(offset);
            assert!(parameter.disable_for_coupled_control());
            assert!(!parameter.value());
            assert!(!parameter.unmodulated_plain_value());
            assert_eq!(parameter.modulated_normalized_value(), 0.0);
            assert_eq!(parameter.unmodulated_normalized_value(), 0.0);
            assert_eq!(parameter.modulation_offset.load(Ordering::Relaxed), 0.0);
            assert_eq!(parameter.flags(), flags);
            assert!(!parameter.disable_for_coupled_control());
            // Clearing the old offset does not disable future modulation.
            assert!(parameter.modulate_value(0.75));
            assert!(parameter.value());
            assert!(!parameter.unmodulated_plain_value());
        }
    }

    #[test]
    fn disabling_publishes_every_field_before_the_effective_change_callback() {
        let parameter = Arc::new_cyclic(|weak: &std::sync::Weak<BoolParam>| {
            let weak = weak.clone();
            BoolParam::new("Coupled", true).with_callback(Arc::new(move |value| {
                let parameter = weak.upgrade().unwrap();
                assert!(!value);
                assert!(!parameter.value());
                assert!(!parameter.unmodulated_plain_value());
                assert_eq!(parameter.modulated_normalized_value(), 0.0);
                assert_eq!(parameter.unmodulated_normalized_value(), 0.0);
                assert_eq!(parameter.modulation_offset.load(Ordering::Relaxed), 0.0);
            }))
        });
        assert!(parameter.disable_for_coupled_control());
    }

    #[test]
    fn already_ineffective_peer_still_clears_plain_value_without_false_callback() {
        let callbacks = Arc::new(AtomicUsize::new(0));
        let observed = callbacks.clone();
        let parameter = BoolParam::new("Coupled", true).with_callback(Arc::new(move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
        }));
        assert!(parameter.modulate_value(-1.0));
        assert!(!parameter.value());
        assert!(parameter.unmodulated_plain_value());
        let before = callbacks.load(Ordering::Relaxed);
        assert!(parameter.disable_for_coupled_control());
        assert_eq!(callbacks.load(Ordering::Relaxed), before);
        assert!(!parameter.unmodulated_plain_value());
        assert_eq!(parameter.modulation_offset.load(Ordering::Relaxed), 0.0);
    }
    #[test]
    fn opted_in_plain_updates_publish_base_under_negative_modulation_and_echo() {
        let callbacks = Arc::new(AtomicUsize::new(0));
        let observed = callbacks.clone();
        let parameter = BoolParam::new("Coupled", false).with_coupled_update_callback(Arc::new(
            move |base, effective| {
                if base {
                    assert!(!effective);
                }
                observed.fetch_add(1, Ordering::Relaxed);
            },
        ));
        parameter.modulate_value(-1.0);
        assert!(parameter.set_plain_value(true));
        assert!(parameter.unmodulated_plain_value());
        assert_eq!(parameter.unmodulated_normalized_value(), 1.0);
        assert!(!parameter.value());
        assert_eq!(parameter.modulated_normalized_value(), 0.0);
        let before = callbacks.load(Ordering::Relaxed);
        assert!(!parameter.set_plain_value(true));
        assert_eq!(callbacks.load(Ordering::Relaxed), before + 1);
    }

    #[test]
    fn ordinary_callbacks_keep_legacy_effective_change_and_echo_behavior() {
        let callbacks = Arc::new(AtomicUsize::new(0));
        let observed = callbacks.clone();
        let parameter = BoolParam::new("Ordinary", false).with_callback(Arc::new(move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
        }));
        parameter.modulate_value(-1.0);
        assert!(!parameter.set_plain_value(true));
        assert!(!parameter.unmodulated_plain_value());
        assert_eq!(callbacks.load(Ordering::Relaxed), 0);
        parameter.modulate_value(0.0);
        assert!(parameter.set_plain_value(true));
        assert!(!parameter.set_plain_value(true));
        assert_eq!(callbacks.load(Ordering::Relaxed), 1);
    }
}
