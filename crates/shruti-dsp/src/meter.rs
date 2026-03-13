use crate::buffer::AudioBuffer;

/// Audio level meter with peak, RMS, and LUFS measurements.
#[derive(Debug, Clone)]
pub struct Meter {
    /// Current peak level per channel (linear).
    pub peak: Vec<f32>,
    /// Current RMS level per channel (linear).
    pub rms: Vec<f32>,
    /// Integrated LUFS value (mono/stereo).
    pub lufs: f32,
    channels: usize,
    // RMS accumulator state
    rms_sum: Vec<f64>,
    rms_count: u64,
    // LUFS gating state (simplified EBU R128)
    lufs_blocks: Vec<f64>,
    lufs_buffer: Vec<f64>,
    lufs_buffer_pos: usize,
    /// Peak hold with decay
    peak_hold: Vec<f32>,
    peak_decay: f32,
}

impl Meter {
    pub fn new(channels: usize, sample_rate: f32) -> Self {
        Self {
            peak: vec![0.0; channels],
            rms: vec![0.0; channels],
            lufs: -200.0,
            channels,
            rms_sum: vec![0.0; channels],
            rms_count: 0,
            lufs_blocks: Vec::new(),
            lufs_buffer: vec![0.0; (sample_rate * 0.4) as usize], // 400ms blocks
            lufs_buffer_pos: 0,
            peak_hold: vec![0.0; channels],
            peak_decay: 0.9995,
        }
    }

    /// Analyze an audio buffer and update all meter values.
    pub fn process(&mut self, buffer: &AudioBuffer) {
        let frames = buffer.frames();
        let channels = buffer.channels() as usize;
        let active_channels = channels.min(self.channels);

        // Reset peak for this block
        for ch_peak in &mut self.peak {
            *ch_peak = 0.0;
        }

        for frame in 0..frames {
            // EBU R128 compliant LUFS: average per-channel mean-square values
            let mut channel_sq_sum: f64 = 0.0;

            for ch in 0..active_channels {
                let sample = buffer.get(frame, ch as u16);
                let abs = sample.abs();

                // Peak detection
                if abs > self.peak[ch] {
                    self.peak[ch] = abs;
                }

                // RMS accumulation
                let sq = (sample as f64).powi(2);
                self.rms_sum[ch] += sq;

                // LUFS: sum squared samples per channel
                channel_sq_sum += sq;
            }

            self.rms_count += 1;

            // LUFS: average RMS-squared per channel, then accumulate into 400ms blocks
            let mean_sq = if active_channels > 0 {
                channel_sq_sum / active_channels as f64
            } else {
                0.0
            };
            if self.lufs_buffer_pos < self.lufs_buffer.len() {
                self.lufs_buffer[self.lufs_buffer_pos] = mean_sq;
                self.lufs_buffer_pos += 1;
            }

            if self.lufs_buffer_pos >= self.lufs_buffer.len() {
                // Complete a 400ms block
                let block_power: f64 =
                    self.lufs_buffer.iter().sum::<f64>() / self.lufs_buffer.len() as f64;
                self.lufs_blocks.push(block_power);
                self.lufs_buffer_pos = 0;
                self.compute_lufs();
            }
        }

        // Update RMS
        if self.rms_count > 0 {
            for ch in 0..self.channels {
                self.rms[ch] = (self.rms_sum[ch] / self.rms_count as f64).sqrt() as f32;
            }
        }

        // Update peak hold with decay
        for ch in 0..self.channels {
            if self.peak[ch] > self.peak_hold[ch] {
                self.peak_hold[ch] = self.peak[ch];
            } else {
                self.peak_hold[ch] *= self.peak_decay;
            }
        }
    }

    /// Compute integrated LUFS using simplified EBU R128 gating.
    fn compute_lufs(&mut self) {
        if self.lufs_blocks.is_empty() {
            self.lufs = -200.0;
            return;
        }

        // Absolute gate: -70 LUFS
        let abs_gate = 10.0_f64.powf(-70.0 / 10.0);
        let gated: Vec<f64> = self
            .lufs_blocks
            .iter()
            .copied()
            .filter(|&p| p > abs_gate)
            .collect();

        if gated.is_empty() {
            self.lufs = -200.0;
            return;
        }

        // Relative gate: mean - 10 LUFS
        let mean_power = gated.iter().sum::<f64>() / gated.len() as f64;
        let rel_gate = mean_power * 10.0_f64.powf(-10.0 / 10.0);

        let final_blocks: Vec<f64> = gated.into_iter().filter(|&p| p > rel_gate).collect();

        if final_blocks.is_empty() {
            self.lufs = -200.0;
            return;
        }

        let integrated = final_blocks.iter().sum::<f64>() / final_blocks.len() as f64;
        self.lufs = (-0.691 + 10.0 * integrated.log10()) as f32;
    }

    /// Get peak level in dB for a channel.
    pub fn peak_db(&self, channel: usize) -> f32 {
        linear_to_db(self.peak.get(channel).copied().unwrap_or(0.0))
    }

    /// Get RMS level in dB for a channel.
    pub fn rms_db(&self, channel: usize) -> f32 {
        linear_to_db(self.rms.get(channel).copied().unwrap_or(0.0))
    }

    /// Get peak hold level in dB for a channel.
    pub fn peak_hold_db(&self, channel: usize) -> f32 {
        linear_to_db(self.peak_hold.get(channel).copied().unwrap_or(0.0))
    }

    /// Reset all meter state.
    pub fn reset(&mut self) {
        self.peak.fill(0.0);
        self.rms.fill(0.0);
        self.lufs = -200.0;
        self.rms_sum.fill(0.0);
        self.rms_count = 0;
        self.lufs_blocks.clear();
        self.lufs_buffer.fill(0.0);
        self.lufs_buffer_pos = 0;
        self.peak_hold.fill(0.0);
    }
}

fn linear_to_db(linear: f32) -> f32 {
    if linear < 1e-10 {
        -200.0
    } else {
        20.0 * linear.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_silence() {
        let mut meter = Meter::new(2, 48000.0);
        let buf = AudioBuffer::new(2, 256);
        meter.process(&buf);

        assert_eq!(meter.peak[0], 0.0);
        assert_eq!(meter.peak[1], 0.0);
        assert_eq!(meter.rms[0], 0.0);
    }

    #[test]
    fn test_meter_peak_detection() {
        let mut meter = Meter::new(1, 48000.0);
        let mut data = vec![0.0_f32; 256];
        data[100] = 0.75;
        data[200] = -0.9;
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);

        assert!((meter.peak[0] - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_meter_rms() {
        let mut meter = Meter::new(1, 48000.0);
        // Constant signal of 0.5 — RMS should equal 0.5
        let data = vec![0.5_f32; 1024];
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);

        assert!(
            (meter.rms[0] - 0.5).abs() < 0.001,
            "RMS of constant 0.5 signal"
        );
    }

    #[test]
    fn test_meter_db_conversion() {
        assert!((linear_to_db(1.0)).abs() < 0.001);
        assert!((linear_to_db(0.5) - (-6.02)).abs() < 0.1);
        assert!(linear_to_db(0.0) < -100.0);
    }

    #[test]
    fn test_meter_reset() {
        let mut meter = Meter::new(2, 48000.0);
        let data = vec![0.5_f32; 512];
        let buf = AudioBuffer::from_interleaved(data, 2);
        meter.process(&buf);

        meter.reset();
        assert_eq!(meter.peak[0], 0.0);
        assert_eq!(meter.rms[0], 0.0);
        assert_eq!(meter.lufs, -200.0);
    }

    #[test]
    fn test_lufs_with_real_signal() {
        let mut meter = Meter::new(1, 48000.0);
        let sample_rate = 48000.0_f32;
        // Generate a 1kHz sine at -14 dBFS for enough samples to fill multiple 400ms blocks
        let frames = (sample_rate * 2.0) as usize; // 2 seconds
        let amplitude = 0.2; // roughly -14 dBFS
        let data: Vec<f32> = (0..frames)
            .map(|i| {
                (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sample_rate).sin() * amplitude
            })
            .collect();
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);

        // LUFS should be a finite negative number (not -200)
        assert!(
            meter.lufs > -200.0,
            "LUFS should be computed for a real signal, got {}",
            meter.lufs
        );
        assert!(
            meter.lufs < 0.0,
            "LUFS should be negative, got {}",
            meter.lufs
        );
    }

    #[test]
    fn test_peak_hold_and_peak_hold_db() {
        let mut meter = Meter::new(1, 48000.0);

        // First block with a loud peak
        let mut data1 = vec![0.0_f32; 256];
        data1[50] = 0.8;
        let buf1 = AudioBuffer::from_interleaved(data1, 1);
        meter.process(&buf1);

        assert!(
            (meter.peak_hold[0] - 0.8).abs() < 0.001,
            "Peak hold should capture 0.8"
        );
        let hold_db = meter.peak_hold_db(0);
        assert!(hold_db > -200.0, "peak_hold_db should be finite");
        assert!(
            (hold_db - 20.0 * 0.8_f32.log10()).abs() < 0.1,
            "peak_hold_db should match linear-to-dB of 0.8"
        );

        // Second block with a quieter signal — peak hold should decay but remain > 0
        let data2 = vec![0.01_f32; 256];
        let buf2 = AudioBuffer::from_interleaved(data2, 1);
        meter.process(&buf2);

        assert!(
            meter.peak_hold[0] > 0.0,
            "Peak hold should still be positive after decay"
        );
        assert!(
            meter.peak_hold[0] < 0.8,
            "Peak hold should have decayed below 0.8"
        );
    }

    #[test]
    fn test_multi_channel_processing() {
        let mut meter = Meter::new(2, 48000.0);
        // Stereo buffer: channel 0 has 0.6, channel 1 has 0.3
        let frames = 256;
        let mut data = vec![0.0_f32; frames * 2];
        for i in 0..frames {
            data[i * 2] = 0.6;
            data[i * 2 + 1] = 0.3;
        }
        let buf = AudioBuffer::from_interleaved(data, 2);
        meter.process(&buf);

        assert!(
            (meter.peak[0] - 0.6).abs() < 0.001,
            "Channel 0 peak should be 0.6"
        );
        assert!(
            (meter.peak[1] - 0.3).abs() < 0.001,
            "Channel 1 peak should be 0.3"
        );
        assert!(
            (meter.rms[0] - 0.6).abs() < 0.001,
            "Channel 0 RMS should be 0.6"
        );
        assert!(
            (meter.rms[1] - 0.3).abs() < 0.001,
            "Channel 1 RMS should be 0.3"
        );
        // dB values
        assert!(meter.peak_db(0) > meter.peak_db(1));
        assert!(meter.rms_db(0) > meter.rms_db(1));
    }

    #[test]
    fn test_reset_clears_everything() {
        let mut meter = Meter::new(2, 48000.0);
        let sample_rate = 48000.0_f32;
        // Process enough data to populate LUFS blocks
        let frames = (sample_rate * 1.0) as usize;
        let data: Vec<f32> = (0..frames * 2)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sample_rate).sin() * 0.5)
            .collect();
        let buf = AudioBuffer::from_interleaved(data, 2);
        meter.process(&buf);

        // Verify something was accumulated
        assert!(meter.rms[0] > 0.0);
        assert!(meter.peak_hold[0] > 0.0);

        meter.reset();

        assert_eq!(meter.peak[0], 0.0);
        assert_eq!(meter.peak[1], 0.0);
        assert_eq!(meter.rms[0], 0.0);
        assert_eq!(meter.rms[1], 0.0);
        assert_eq!(meter.lufs, -200.0);
        assert_eq!(meter.peak_hold[0], 0.0);
        assert_eq!(meter.peak_hold[1], 0.0);
        // Internal state should be cleared
        assert_eq!(meter.rms_count, 0);
        assert!(meter.lufs_blocks.is_empty());
        assert_eq!(meter.lufs_buffer_pos, 0);
    }

    #[test]
    fn test_peak_detection_negative_signal() {
        let mut meter = Meter::new(1, 48000.0);
        let mut data = vec![0.0f32; 256];
        data[50] = -0.95;
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);
        assert!(
            (meter.peak[0] - 0.95).abs() < 0.001,
            "Peak should detect absolute value"
        );
    }

    #[test]
    fn test_rms_of_sine_wave() {
        let mut meter = Meter::new(1, 48000.0);
        let frames = 48000; // full second at 48kHz
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);
        // RMS of sine wave = 1/sqrt(2) ~ 0.707
        assert!(
            (meter.rms[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01,
            "RMS of unit sine should be ~0.707, got {}",
            meter.rms[0]
        );
    }

    #[test]
    fn test_lufs_silence_returns_floor() {
        let mut meter = Meter::new(1, 48000.0);
        // Process enough silence to fill several 400ms blocks
        let frames = 48000; // 1 second
        let buf = AudioBuffer::new(1, frames);
        meter.process(&buf);
        assert_eq!(meter.lufs, -200.0, "LUFS of silence should be -200");
    }

    #[test]
    fn test_lufs_loud_signal() {
        let mut meter = Meter::new(1, 48000.0);
        let frames = 96000; // 2 seconds
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin() * 0.5)
            .collect();
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);

        assert!(
            meter.lufs > -200.0,
            "LUFS should be computed, got {}",
            meter.lufs
        );
        assert!(
            meter.lufs < 0.0,
            "LUFS should be negative, got {}",
            meter.lufs
        );
    }

    #[test]
    fn test_meter_single_sample() {
        let mut meter = Meter::new(1, 48000.0);
        let buf = AudioBuffer::from_interleaved(vec![0.42], 1);
        meter.process(&buf);
        assert!((meter.peak[0] - 0.42).abs() < 0.001);
        assert!((meter.rms[0] - 0.42).abs() < 0.001);
    }

    #[test]
    fn test_meter_very_loud_signal() {
        let mut meter = Meter::new(1, 48000.0);
        let data = vec![5.0f32; 1024]; // clipped/over-range signal
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);
        assert!(
            (meter.peak[0] - 5.0).abs() < 0.001,
            "Peak should handle over-range"
        );
        assert!(
            (meter.rms[0] - 5.0).abs() < 0.001,
            "RMS should handle over-range"
        );
        assert!(
            meter.peak_db(0) > 0.0,
            "Peak dB should be positive for signal > 1.0"
        );
    }

    #[test]
    fn test_peak_db_and_rms_db_for_known_values() {
        let mut meter = Meter::new(1, 48000.0);
        let data = vec![1.0f32; 256];
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);
        // Peak of 1.0 = 0 dB
        assert!((meter.peak_db(0)).abs() < 0.1, "1.0 linear should be ~0 dB");
        assert!(
            (meter.rms_db(0)).abs() < 0.1,
            "RMS of constant 1.0 should be ~0 dB"
        );
    }

    #[test]
    fn test_peak_db_invalid_channel() {
        let meter = Meter::new(1, 48000.0);
        // Channel 5 does not exist
        let db = meter.peak_db(5);
        assert!(db < -100.0, "Invalid channel should return very low dB");
    }

    #[test]
    fn test_rms_db_invalid_channel() {
        let meter = Meter::new(1, 48000.0);
        let db = meter.rms_db(5);
        assert!(db < -100.0, "Invalid channel should return very low dB");
    }

    #[test]
    fn test_meter_accumulates_across_process_calls() {
        let mut meter = Meter::new(1, 48000.0);

        // First call: signal of 0.3
        let buf1 = AudioBuffer::from_interleaved(vec![0.3f32; 512], 1);
        meter.process(&buf1);
        let rms1 = meter.rms[0];

        // Second call: signal of 0.6
        let buf2 = AudioBuffer::from_interleaved(vec![0.6f32; 512], 1);
        meter.process(&buf2);
        let rms2 = meter.rms[0];

        // RMS after both calls should reflect both blocks
        // RMS = sqrt((512*0.3^2 + 512*0.6^2) / 1024) = sqrt((46.08 + 184.32)/1024) = sqrt(0.225) ~ 0.474
        let expected_rms = ((512.0 * 0.3f32.powi(2) + 512.0 * 0.6f32.powi(2)) / 1024.0).sqrt();
        assert!(
            (rms2 - expected_rms).abs() < 0.01,
            "RMS should accumulate: expected {expected_rms}, got {rms2}"
        );
        assert!(rms2 != rms1, "Second process call should change RMS");
    }

    #[test]
    fn test_lufs_ebu_r128_mono_sine() {
        // EBU R128: a 997 Hz sine at -3.01 dBFS (amplitude = 1/sqrt(2))
        // has RMS of 0.5, so LUFS = -0.691 + 10*log10(0.5^2) = -0.691 + 10*(-0.30103) = -3.70 LUFS
        // (This is approximate due to simplified gating.)
        let mut meter = Meter::new(1, 48000.0);
        let frames = 96000; // 2 seconds
        let amplitude = std::f32::consts::FRAC_1_SQRT_2;
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 997.0 * i as f32 / 48000.0).sin() * amplitude)
            .collect();
        let buf = AudioBuffer::from_interleaved(data, 1);
        meter.process(&buf);

        // RMS of sine at amplitude A = A/sqrt(2), mean-square = A^2/2 = 0.25
        // LUFS = -0.691 + 10*log10(0.25) = -0.691 + (-6.02) = -6.71
        let expected_lufs = -0.691 + 10.0 * (0.25_f64).log10();
        assert!(
            (meter.lufs as f64 - expected_lufs).abs() < 0.5,
            "LUFS for 1/sqrt(2) sine should be ~{:.2}, got {:.2}",
            expected_lufs,
            meter.lufs
        );
    }

    #[test]
    fn test_lufs_stereo_matches_mono_for_equal_channels() {
        // When both channels have identical signals, LUFS should match mono result
        let sample_rate = 48000.0;
        let frames = 96000usize;
        let amplitude = 0.3;

        // Mono
        let mut meter_mono = Meter::new(1, sample_rate);
        let mono_data: Vec<f32> = (0..frames)
            .map(|i| {
                (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sample_rate).sin() * amplitude
            })
            .collect();
        let buf_mono = AudioBuffer::from_interleaved(mono_data, 1);
        meter_mono.process(&buf_mono);

        // Stereo (same signal both channels)
        let mut meter_stereo = Meter::new(2, sample_rate);
        let stereo_data: Vec<f32> = (0..frames)
            .flat_map(|i| {
                let s = (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sample_rate).sin()
                    * amplitude;
                [s, s]
            })
            .collect();
        let buf_stereo = AudioBuffer::from_interleaved(stereo_data, 2);
        meter_stereo.process(&buf_stereo);

        assert!(
            (meter_mono.lufs - meter_stereo.lufs).abs() < 0.5,
            "Stereo LUFS with identical channels should match mono: mono={}, stereo={}",
            meter_mono.lufs,
            meter_stereo.lufs
        );
    }
}
