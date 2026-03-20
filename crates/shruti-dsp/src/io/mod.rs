pub mod reader;
pub mod writer;

#[cfg(feature = "tarang")]
pub mod loudness;
#[cfg(feature = "tarang")]
pub mod mix;
#[cfg(feature = "tarang")]
pub mod resample;

pub use reader::{SUPPORTED_EXTENSIONS, read_audio_file};
pub use writer::{
    BitDepth, ExportConfig, ExportFormat, is_native_export, write_audio_file, write_wav_file,
};

#[cfg(feature = "tarang")]
pub use loudness::{LoudnessMetrics, apply_gain, measure_loudness, normalize_loudness};
#[cfg(feature = "tarang")]
pub use mix::{ChannelLayout, mix_channels};
#[cfg(feature = "tarang")]
pub use reader::{StreamingError, StreamingReader};
#[cfg(feature = "tarang")]
pub use resample::{resample, resample_sinc};
