use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use shruti_dsp::AudioFormat;

use super::{AudioHost, AudioStream, DeviceInfo};

/// Audio backend powered by cpal.
/// Supports ALSA/PipeWire (Linux), CoreAudio (macOS), WASAPI (Windows).
pub struct CpalBackend {
    host: cpal::Host,
}

impl CpalBackend {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    fn find_output_device(&self, name: Option<&str>) -> Option<cpal::Device> {
        match name {
            Some(name) => self
                .host
                .output_devices()
                .ok()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false)),
            None => self.host.default_output_device(),
        }
    }

    fn find_input_device(&self, name: Option<&str>) -> Option<cpal::Device> {
        match name {
            Some(name) => self
                .host
                .input_devices()
                .ok()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false)),
            None => self.host.default_input_device(),
        }
    }
}

impl Default for CpalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioHost for CpalBackend {
    fn output_devices(&self) -> Vec<DeviceInfo> {
        let default_name = self
            .host
            .default_output_device()
            .and_then(|d| d.name().ok());

        self.host
            .output_devices()
            .map(|devices| {
                devices
                    .filter_map(|d| {
                        let name = d.name().ok()?;
                        Some(DeviceInfo {
                            is_default: default_name.as_deref() == Some(&name),
                            name,
                            is_input: false,
                            is_output: true,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn input_devices(&self) -> Vec<DeviceInfo> {
        let default_name = self
            .host
            .default_input_device()
            .and_then(|d| d.name().ok());

        self.host
            .input_devices()
            .map(|devices| {
                devices
                    .filter_map(|d| {
                        let name = d.name().ok()?;
                        Some(DeviceInfo {
                            is_default: default_name.as_deref() == Some(&name),
                            name,
                            is_input: true,
                            is_output: false,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn open_output_stream(
        &self,
        device: Option<&str>,
        format: AudioFormat,
        mut callback: Box<dyn FnMut(&mut [f32]) + Send + 'static>,
    ) -> Result<Box<dyn AudioStream>, Box<dyn std::error::Error>> {
        let dev = self
            .find_output_device(device)
            .ok_or("output device not found")?;

        let config = StreamConfig {
            channels: format.channels,
            sample_rate: cpal::SampleRate(format.sample_rate),
            buffer_size: if format.buffer_size > 0 {
                cpal::BufferSize::Fixed(format.buffer_size)
            } else {
                cpal::BufferSize::Default
            },
        };

        let stream = dev.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                callback(data);
            },
            |err| eprintln!("audio output error: {err}"),
            None,
        )?;

        Ok(Box::new(CpalStream { stream }))
    }

    fn open_input_stream(
        &self,
        device: Option<&str>,
        format: AudioFormat,
        mut callback: Box<dyn FnMut(&[f32]) + Send + 'static>,
    ) -> Result<Box<dyn AudioStream>, Box<dyn std::error::Error>> {
        let dev = self
            .find_input_device(device)
            .ok_or("input device not found")?;

        let config = StreamConfig {
            channels: format.channels,
            sample_rate: cpal::SampleRate(format.sample_rate),
            buffer_size: if format.buffer_size > 0 {
                cpal::BufferSize::Fixed(format.buffer_size)
            } else {
                cpal::BufferSize::Default
            },
        };

        let stream = dev.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                callback(data);
            },
            |err| eprintln!("audio input error: {err}"),
            None,
        )?;

        Ok(Box::new(CpalStream { stream }))
    }
}

struct CpalStream {
    stream: cpal::Stream,
}

// Safety: cpal::Stream is Send on all platforms we target.
unsafe impl Send for CpalStream {}

impl AudioStream for CpalStream {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.play()?;
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.pause()?;
        Ok(())
    }
}
