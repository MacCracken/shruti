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
    pub max_channels: u16,
    pub supported_sample_rates: Vec<u32>,
}

/// Trait for platform audio host abstraction.
pub trait AudioHost {
    fn output_devices(&self) -> Vec<DeviceInfo>;
    fn input_devices(&self) -> Vec<DeviceInfo>;

    /// Return all devices, merging input and output entries that share a name.
    fn all_devices(&self) -> Vec<DeviceInfo> {
        let mut devices = self.output_devices();
        for input in self.input_devices() {
            if let Some(existing) = devices.iter_mut().find(|d| d.name == input.name) {
                existing.is_input = true;
            } else {
                devices.push(input);
            }
        }
        devices
    }

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
