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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let info = DeviceInfo {
            name: "Test Device".to_string(),
            is_default: true,
            is_input: false,
            is_output: true,
            max_channels: 2,
            supported_sample_rates: vec![44100, 48000, 96000],
        };
        assert_eq!(info.name, "Test Device");
        assert!(info.is_default);
        assert!(!info.is_input);
        assert!(info.is_output);
        assert_eq!(info.max_channels, 2);
        assert_eq!(info.supported_sample_rates, vec![44100, 48000, 96000]);
    }

    #[test]
    fn test_device_info_clone() {
        let info = DeviceInfo {
            name: "Clone Test".to_string(),
            is_default: false,
            is_input: true,
            is_output: true,
            max_channels: 8,
            supported_sample_rates: vec![48000],
        };
        let cloned = info.clone();
        assert_eq!(cloned.name, info.name);
        assert_eq!(cloned.is_default, info.is_default);
        assert_eq!(cloned.max_channels, info.max_channels);
        assert_eq!(cloned.supported_sample_rates, info.supported_sample_rates);
    }

    #[test]
    fn test_device_info_debug() {
        let info = DeviceInfo {
            name: "Debug Dev".to_string(),
            is_default: true,
            is_input: true,
            is_output: false,
            max_channels: 1,
            supported_sample_rates: vec![44100],
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("Debug Dev"));
        assert!(debug_str.contains("is_default: true"));
    }

    /// Mock AudioHost to test the default `all_devices` implementation.
    struct MockHost {
        outputs: Vec<DeviceInfo>,
        inputs: Vec<DeviceInfo>,
    }

    impl AudioHost for MockHost {
        fn output_devices(&self) -> Vec<DeviceInfo> {
            self.outputs.clone()
        }

        fn input_devices(&self) -> Vec<DeviceInfo> {
            self.inputs.clone()
        }

        fn open_output_stream(
            &self,
            _device: Option<&str>,
            _format: AudioFormat,
            _callback: OutputCallback,
        ) -> Result<Box<dyn AudioStream>, Box<dyn std::error::Error>> {
            Err("not implemented".into())
        }

        fn open_input_stream(
            &self,
            _device: Option<&str>,
            _format: AudioFormat,
            _callback: InputCallback,
        ) -> Result<Box<dyn AudioStream>, Box<dyn std::error::Error>> {
            Err("not implemented".into())
        }
    }

    #[test]
    fn test_all_devices_merges_by_name() {
        let host = MockHost {
            outputs: vec![DeviceInfo {
                name: "Shared Device".to_string(),
                is_default: true,
                is_input: false,
                is_output: true,
                max_channels: 2,
                supported_sample_rates: vec![48000],
            }],
            inputs: vec![DeviceInfo {
                name: "Shared Device".to_string(),
                is_default: false,
                is_input: true,
                is_output: false,
                max_channels: 2,
                supported_sample_rates: vec![48000],
            }],
        };

        let all = host.all_devices();
        assert_eq!(all.len(), 1);
        assert!(all[0].is_input);
        assert!(all[0].is_output);
        assert_eq!(all[0].name, "Shared Device");
    }

    #[test]
    fn test_all_devices_separate_devices() {
        let host = MockHost {
            outputs: vec![DeviceInfo {
                name: "Speaker".to_string(),
                is_default: true,
                is_input: false,
                is_output: true,
                max_channels: 2,
                supported_sample_rates: vec![48000],
            }],
            inputs: vec![DeviceInfo {
                name: "Microphone".to_string(),
                is_default: false,
                is_input: true,
                is_output: false,
                max_channels: 1,
                supported_sample_rates: vec![44100],
            }],
        };

        let all = host.all_devices();
        assert_eq!(all.len(), 2);
        let names: Vec<&str> = all.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Speaker"));
        assert!(names.contains(&"Microphone"));
    }

    #[test]
    fn test_all_devices_empty() {
        let host = MockHost {
            outputs: vec![],
            inputs: vec![],
        };
        let all = host.all_devices();
        assert!(all.is_empty());
    }

    #[test]
    fn test_all_devices_only_inputs() {
        let host = MockHost {
            outputs: vec![],
            inputs: vec![DeviceInfo {
                name: "Mic".to_string(),
                is_default: true,
                is_input: true,
                is_output: false,
                max_channels: 1,
                supported_sample_rates: vec![44100],
            }],
        };
        let all = host.all_devices();
        assert_eq!(all.len(), 1);
        assert!(all[0].is_input);
        assert!(!all[0].is_output);
    }
}
