use serde::{Deserialize, Serialize};
use shruti_dsp::AudioBuffer;
use shruti_session::midi::{ControlChange, NoteEvent};

/// Information about an instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentInfo {
    pub name: String,
    pub category: String,
    pub author: String,
    pub description: String,
}

/// A named parameter of an instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentParam {
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub value: f32,
    pub unit: String,
}

impl InstrumentParam {
    pub fn new(name: &str, min: f32, max: f32, default: f32, unit: &str) -> Self {
        Self {
            name: name.to_string(),
            min,
            max,
            default,
            value: default,
            unit: unit.to_string(),
        }
    }

    /// Set value, clamped to [min, max].
    pub fn set(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
    }

    /// Get normalized value (0.0 to 1.0).
    pub fn normalized(&self) -> f32 {
        if (self.max - self.min).abs() < f32::EPSILON {
            return 0.0;
        }
        (self.value - self.min) / (self.max - self.min)
    }

    /// Set from normalized value (0.0 to 1.0).
    pub fn set_normalized(&mut self, n: f32) {
        self.value =
            (self.min + n.clamp(0.0, 1.0) * (self.max - self.min)).clamp(self.min, self.max);
    }
}

/// Trait for type-safe parameter indices.
///
/// Each instrument defines an enum whose variants map 1-to-1 to the entries in
/// `InstrumentNode::params()`. Implementing this trait lets callers use the enum
/// with the typed `get_param` / `set_param` helpers instead of raw `usize` offsets.
pub trait ParamIndex: Copy {
    /// The `usize` offset into the params slice.
    fn index(self) -> usize;
    /// Total number of parameters the instrument exposes.
    fn count() -> usize;
}

/// Trait for all virtual instruments.
pub trait InstrumentNode: Send {
    /// Get instrument info.
    fn info(&self) -> &InstrumentInfo;

    /// Set the sample rate. Called when the audio engine starts or sample rate changes.
    fn set_sample_rate(&mut self, sample_rate: f32);

    /// Process a block of audio. The instrument reads MIDI events and writes audio output.
    /// `note_events` are sorted by position within this block.
    /// `output` is the buffer to write into (instrument should ADD to existing content, not overwrite).
    fn process(
        &mut self,
        note_events: &[NoteEvent],
        control_changes: &[ControlChange],
        output: &mut AudioBuffer,
    );

    /// Handle a single note-on event (for real-time MIDI input).
    fn note_on(&mut self, note: u8, velocity: u8, channel: u8);

    /// Handle a single note-off event.
    fn note_off(&mut self, note: u8, channel: u8);

    /// Get all parameters.
    fn params(&self) -> &[InstrumentParam];

    /// Get a mutable reference to all parameters.
    fn params_mut(&mut self) -> &mut [InstrumentParam];

    /// Reset all internal state (voices, oscillators, etc.).
    fn reset(&mut self);

    /// Number of currently active voices.
    fn active_voices(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_new_defaults_value_to_default() {
        let p = InstrumentParam::new("Volume", 0.0, 1.0, 0.75, "dB");
        assert_eq!(p.name, "Volume");
        assert_eq!(p.min, 0.0);
        assert_eq!(p.max, 1.0);
        assert_eq!(p.default, 0.75);
        assert_eq!(p.value, 0.75);
        assert_eq!(p.unit, "dB");
    }

    #[test]
    fn param_set_clamps_to_range() {
        let mut p = InstrumentParam::new("Gain", 0.0, 1.0, 0.5, "");
        p.set(2.0);
        assert_eq!(p.value, 1.0);
        p.set(-1.0);
        assert_eq!(p.value, 0.0);
        p.set(0.5);
        assert_eq!(p.value, 0.5);
    }

    #[test]
    fn param_normalized_returns_correct_range() {
        let p = InstrumentParam::new("Cutoff", 20.0, 20000.0, 1000.0, "Hz");
        let n = p.normalized();
        let expected = (1000.0 - 20.0) / (20000.0 - 20.0);
        assert!((n - expected).abs() < 1e-6);
    }

    #[test]
    fn param_normalized_at_min_is_zero() {
        let mut p = InstrumentParam::new("Test", 0.0, 100.0, 50.0, "");
        p.set(0.0);
        assert!((p.normalized() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn param_normalized_at_max_is_one() {
        let mut p = InstrumentParam::new("Test", 0.0, 100.0, 50.0, "");
        p.set(100.0);
        assert!((p.normalized() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn param_normalized_same_min_max_returns_zero() {
        let p = InstrumentParam::new("Fixed", 5.0, 5.0, 5.0, "");
        assert_eq!(p.normalized(), 0.0);
    }

    #[test]
    fn param_set_normalized_maps_correctly() {
        let mut p = InstrumentParam::new("Freq", 20.0, 20000.0, 1000.0, "Hz");
        p.set_normalized(0.0);
        assert!((p.value - 20.0).abs() < 1e-3);
        p.set_normalized(1.0);
        assert!((p.value - 20000.0).abs() < 1e-3);
        p.set_normalized(0.5);
        let expected = 20.0 + 0.5 * (20000.0 - 20.0);
        assert!((p.value - expected).abs() < 1e-3);
    }

    #[test]
    fn param_set_normalized_clamps_input() {
        let mut p = InstrumentParam::new("Test", 0.0, 10.0, 5.0, "");
        p.set_normalized(2.0);
        assert_eq!(p.value, 10.0);
        p.set_normalized(-1.0);
        assert_eq!(p.value, 0.0);
    }

    #[test]
    fn param_roundtrip_normalized() {
        let mut p = InstrumentParam::new("Test", 0.0, 100.0, 50.0, "");
        p.set(73.0);
        let n = p.normalized();
        p.set(0.0);
        p.set_normalized(n);
        assert!((p.value - 73.0).abs() < 0.01);
    }
}
