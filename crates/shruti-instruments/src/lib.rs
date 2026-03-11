//! Built-in virtual instruments for Shruti.

pub mod drum_machine;
pub mod envelope;
pub mod filter;
pub mod instrument;
pub mod lfo;
pub mod oscillator;
pub mod preset;
pub mod routing;
pub mod sampler;
pub mod step_sequencer;
pub mod synth;
pub mod voice;

pub use drum_machine::{DrumMachine, DrumPad, PlayMode, NUM_PADS};
pub use envelope::{AdsrParams, Envelope, EnvelopeState};
pub use filter::{Filter, FilterMode};
pub use instrument::{InstrumentInfo, InstrumentNode, InstrumentParam};
pub use lfo::{Lfo, LfoShape};
pub use oscillator::{Oscillator, Waveform};
pub use preset::{InstrumentPreset, PresetParam};
pub use routing::{MidiRoute, VelocityCurve};
pub use sampler::{LoopMode, SampleZone, Sampler};
pub use step_sequencer::{PadSequence, Step, StepSequencer};
pub use synth::SubtractiveSynth;
pub use voice::{Voice, VoiceManager, VoiceState, VoiceStealMode};
