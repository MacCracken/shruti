//! DSP primitives and audio effects.

#![deny(unsafe_code)]

pub mod analysis;
pub mod buffer;
pub mod clock;
pub mod constants;
pub mod effects;
pub mod error;
pub mod format;
pub mod graph;
pub mod io;
pub mod meter;
pub mod midi;

pub use analysis::dynamics::{DynamicsAnalysis, analyze_dynamics};
pub use analysis::spectral::{SpectralAnalysis, analyze_spectrum};
pub use analysis::{Chromagram, R128Loudness, Spectrogram};
pub use buffer::AudioBuffer;
pub use clock::AudioClock;
pub use error::AudioError;
pub use format::{AudioFormat, Sample};
