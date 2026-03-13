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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_construction() {
        let fmt = AudioFormat::new(44100, 2, 512);
        assert_eq!(fmt.sample_rate, 44100);
        assert_eq!(fmt.channels, 2);
        assert_eq!(fmt.buffer_size, 512);
    }

    #[test]
    fn test_audio_format_default() {
        let fmt = AudioFormat::default();
        assert_eq!(fmt.sample_rate, 48000);
        assert_eq!(fmt.channels, 2);
        assert_eq!(fmt.buffer_size, 256);
    }

    #[test]
    fn test_buffer_duration_secs() {
        let fmt = AudioFormat::new(48000, 2, 480);
        let dur = fmt.buffer_duration_secs();
        assert!((dur - 0.01).abs() < 1e-9, "480/48000 = 0.01s, got {dur}");
    }

    #[test]
    fn test_buffer_duration_ms() {
        let fmt = AudioFormat::new(48000, 2, 480);
        let dur_ms = fmt.buffer_duration_ms();
        assert!(
            (dur_ms - 10.0).abs() < 1e-6,
            "480/48000 = 10ms, got {dur_ms}"
        );
    }

    #[test]
    fn test_audio_format_equality() {
        let a = AudioFormat::new(48000, 2, 256);
        let b = AudioFormat::new(48000, 2, 256);
        let c = AudioFormat::new(44100, 2, 256);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_buffer_duration_secs_various_rates() {
        // 44100 Hz, 441 samples = 0.01s
        let fmt = AudioFormat::new(44100, 2, 441);
        assert!((fmt.buffer_duration_secs() - 0.01).abs() < 1e-9);

        // 96000 Hz, 960 samples = 0.01s
        let fmt2 = AudioFormat::new(96000, 1, 960);
        assert!((fmt2.buffer_duration_secs() - 0.01).abs() < 1e-9);
    }

    #[test]
    fn test_buffer_duration_ms_various() {
        let fmt = AudioFormat::new(44100, 2, 4410);
        assert!((fmt.buffer_duration_ms() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_buffer_duration_zero_buffer_size() {
        let fmt = AudioFormat::new(48000, 2, 0);
        assert_eq!(fmt.buffer_duration_secs(), 0.0);
        assert_eq!(fmt.buffer_duration_ms(), 0.0);
    }

    #[test]
    fn test_format_clone_and_copy() {
        let fmt = AudioFormat::new(48000, 2, 256);
        let fmt2 = fmt; // Copy
        let fmt3 = fmt;
        assert_eq!(fmt, fmt2);
        assert_eq!(fmt, fmt3);
    }

    #[test]
    fn test_format_debug() {
        let fmt = AudioFormat::new(48000, 2, 256);
        let debug_str = format!("{:?}", fmt);
        assert!(debug_str.contains("48000"));
        assert!(debug_str.contains("256"));
    }

    #[test]
    fn test_format_different_channels() {
        let mono = AudioFormat::new(48000, 1, 256);
        let stereo = AudioFormat::new(48000, 2, 256);
        assert_ne!(mono, stereo);
    }

    #[test]
    fn test_format_different_buffer_sizes() {
        let a = AudioFormat::new(48000, 2, 128);
        let b = AudioFormat::new(48000, 2, 512);
        assert_ne!(a, b);
    }
}
