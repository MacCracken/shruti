pub mod cpal_backend;

pub use cpal_backend::CpalBackend;

use shruti_dsp::AudioFormat;

/// Callback type for output streams (receives mutable buffer to fill).
pub type OutputCallback = Box<dyn FnMut(&mut [f32]) + Send + 'static>;
/// Callback type for input streams (receives recorded buffer).
pub type InputCallback = Box<dyn FnMut(&[f32]) + Send + 'static>;

/// Information about an audio device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub is_input: bool,
    pub is_output: bool,
}

/// Trait for platform audio host abstraction.
pub trait AudioHost {
    fn output_devices(&self) -> Vec<DeviceInfo>;
    fn input_devices(&self) -> Vec<DeviceInfo>;

    fn open_output_stream(
        &self,
        device: Option<&str>,
        format: AudioFormat,
        callback: OutputCallback,
    ) -> Result<Box<dyn AudioStream>, Box<dyn std::error::Error>>;

    fn open_input_stream(
        &self,
        device: Option<&str>,
        format: AudioFormat,
        callback: InputCallback,
    ) -> Result<Box<dyn AudioStream>, Box<dyn std::error::Error>>;
}

/// Handle to a running audio stream.
pub trait AudioStream: Send {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn stop(&self) -> Result<(), Box<dyn std::error::Error>>;
}
