//! DSP primitives and audio effects.

#![deny(unsafe_code)]

pub mod buffer;
pub mod format;
pub mod io;

pub use buffer::AudioBuffer;
pub use format::{AudioFormat, Sample};
