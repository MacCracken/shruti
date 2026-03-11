use std::collections::BTreeSet;

use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use shruti_dsp::AudioFormat;

use super::{AudioHost, AudioStream, DeviceInfo, InputCallback, OutputCallback};

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
                        let (max_channels, supported_sample_rates) =
                            match d.supported_output_configs() {
                                Ok(configs) => {
                                    let mut max_ch: u16 = 0;
                                    let mut rates = BTreeSet::new();
                                    for cfg in configs {
                                        let ch = cfg.channels();
                                        if ch > max_ch {
                                            max_ch = ch;
                                        }
                                        rates.insert(cfg.min_sample_rate().0);
                                        rates.insert(cfg.max_sample_rate().0);
                                    }
                                    if max_ch == 0 {
                                        (2, vec![44100, 48000, 96000])
                                    } else {
                                        (max_ch, rates.into_iter().collect())
                                    }
                                }
                                Err(_) => (2, vec![44100, 48000, 96000]),
                            };
                        Some(DeviceInfo {
                            is_default: default_name.as_deref() == Some(&name),
                            name,
                            is_input: false,
                            is_output: true,
                            max_channels,
                            supported_sample_rates,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn input_devices(&self) -> Vec<DeviceInfo> {
        let default_name = self.host.default_input_device().and_then(|d| d.name().ok());

        self.host
            .input_devices()
            .map(|devices| {
                devices
                    .filter_map(|d| {
                        let name = d.name().ok()?;
                        let (max_channels, supported_sample_rates) =
                            match d.supported_input_configs() {
                                Ok(configs) => {
                                    let mut max_ch: u16 = 0;
                                    let mut rates = BTreeSet::new();
                                    for cfg in configs {
                                        let ch = cfg.channels();
                                        if ch > max_ch {
                                            max_ch = ch;
                                        }
                                        rates.insert(cfg.min_sample_rate().0);
                                        rates.insert(cfg.max_sample_rate().0);
                                    }
                                    if max_ch == 0 {
                                        (2, vec![44100, 48000, 96000])
                                    } else {
                                        (max_ch, rates.into_iter().collect())
                                    }
                                }
                                Err(_) => (2, vec![44100, 48000, 96000]),
                            };
                        Some(DeviceInfo {
                            is_default: default_name.as_deref() == Some(&name),
                            name,
                            is_input: true,
                            is_output: false,
                            max_channels,
                            supported_sample_rates,
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
        mut callback: OutputCallback,
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
        mut callback: InputCallback,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::AudioHost;

    #[test]
    fn all_devices_merges_input_output() {
        let backend = CpalBackend::new();
        let all = backend.all_devices();
        // In CI there may be no devices, just verify no panic
        for device in &all {
            assert!(!device.name.is_empty());
        }
    }

    #[test]
    fn output_devices_have_channels() {
        let backend = CpalBackend::new();
        for device in backend.output_devices() {
            // max_channels should be at least 1 (or default 2)
            assert!(device.max_channels >= 1);
        }
    }
}
