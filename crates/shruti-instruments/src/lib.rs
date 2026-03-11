//! Built-in virtual instruments for Shruti.

pub mod envelope;
pub mod instrument;
pub mod oscillator;
pub mod synth;
pub mod voice;

pub use envelope::{AdsrParams, Envelope, EnvelopeState};
pub use instrument::{InstrumentInfo, InstrumentNode, InstrumentParam};
pub use oscillator::{Oscillator, Waveform};
pub use synth::SubtractiveSynth;
pub use voice::{Voice, VoiceManager, VoiceState, VoiceStealMode};
