//! Built-in virtual instruments for Shruti.

pub mod drum_kit;
pub mod drum_machine;
pub mod effect_chain;
pub mod envelope;
pub mod error;
pub mod filter;
pub mod instrument;
pub mod lfo;
pub mod mod_matrix;
pub mod oscillator;
pub mod preset;
pub mod routing;
pub mod sampler;
pub mod sf2;
pub mod sfz;
pub mod step_sequencer;
pub mod synth;
pub mod voice;

pub use drum_kit::{DrumKit, DrumKitPad};
pub use drum_machine::{
    DrumMachine, DrumMachineParam, DrumPad, LayerSelection, NUM_PADS, PadEffects, PlayMode,
    SampleLayer,
};
pub use effect_chain::{EffectChain, InstrumentEffect, InstrumentEffectType};
pub use envelope::{AdsrParams, Envelope, EnvelopeState};
pub use error::InstrumentError;
pub use filter::{Filter, FilterMode};
pub use instrument::{InstrumentInfo, InstrumentNode, InstrumentParam, ParamIndex};
pub use lfo::{Lfo, LfoShape};
pub use mod_matrix::{
    ModDestination, ModMatrix, ModOutput, ModRouting, ModSource, ModSourceValues,
};
pub use oscillator::{Oscillator, Waveform};
pub use preset::{InstrumentPreset, PresetParam};
pub use routing::{MidiRoute, VelocityCurve};
pub use sampler::{LoopMode, SampleZone, Sampler, SamplerParam, SlicePoint};
pub use sf2::{Sf2Preset, parse_sf2};
pub use sfz::{SfzRegion, parse_sfz};
pub use step_sequencer::{
    PadSequence, Pattern, PatternBank, PatternChain, PatternId, Step, StepSequencer,
};
pub use synth::{SubtractiveSynth, SynthParam};
pub use voice::{Voice, VoiceManager, VoiceState, VoiceStealMode};
