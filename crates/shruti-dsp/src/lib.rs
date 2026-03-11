//! DSP primitives and audio effects.

#![deny(unsafe_code)]

pub mod buffer;
pub mod effects;
pub mod error;
pub mod format;
pub mod io;
pub mod meter;

pub use buffer::AudioBuffer;
pub use error::AudioError;
pub use format::{AudioFormat, Sample};
