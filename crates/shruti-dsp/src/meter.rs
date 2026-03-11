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

        // Reset peak for this block
        for ch_peak in &mut self.peak {
            *ch_peak = 0.0;
        }

        for frame in 0..frames {
            let mut mono_sum: f64 = 0.0;

            for ch in 0..channels.min(self.channels) {
                let sample = buffer.get(frame, ch as u16);
                let abs = sample.abs();

                // Peak detection
                if abs > self.peak[ch] {
                    self.peak[ch] = abs;
                }

                // RMS accumulation
                self.rms_sum[ch] += (sample as f64).powi(2);

                mono_sum += sample as f64;
            }

            self.rms_count += 1;

            // LUFS: accumulate mono sum into 400ms blocks
            let mono_avg = mono_sum / channels.max(1) as f64;
            if self.lufs_buffer_pos < self.lufs_buffer.len() {
                self.lufs_buffer[self.lufs_buffer_pos] = mono_avg * mono_avg;
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
}
