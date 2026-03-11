/// Audio sample type (32-bit float, range -1.0 to 1.0).
pub type Sample = f32;

/// Describes the format of an audio stream or buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: u32,
}

impl AudioFormat {
    pub fn new(sample_rate: u32, channels: u16, buffer_size: u32) -> Self {
        Self {
            sample_rate,
            channels,
            buffer_size,
        }
    }

    /// Duration of one buffer in seconds.
    pub fn buffer_duration_secs(&self) -> f64 {
        self.buffer_size as f64 / self.sample_rate as f64
    }

    /// Duration of one buffer in milliseconds.
    pub fn buffer_duration_ms(&self) -> f64 {
        self.buffer_duration_secs() * 1000.0
    }
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            buffer_size: 256,
        }
    }
}
