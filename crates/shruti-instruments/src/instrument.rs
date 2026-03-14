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
