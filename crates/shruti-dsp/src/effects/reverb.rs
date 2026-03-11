use crate::buffer::AudioBuffer;

/// Schroeder-style reverb with comb and allpass filters.
#[derive(Debug, Clone)]
pub struct Reverb {
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    pub mix: f32,
    /// Room size / decay (0.0 to 1.0).
    pub room_size: f32,
    /// High frequency damping (0.0 to 1.0).
    pub damping: f32,
    comb_filters: Vec<CombFilter>,
    allpass_filters: Vec<AllpassFilter>,
    sample_rate: f32,
}

#[derive(Debug, Clone)]
struct CombFilter {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    damp1: f32,
    damp2: f32,
    filter_store: f32,
}

impl CombFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            index: 0,
            feedback: 0.7,
            damp1: 0.5,
            damp2: 0.5,
            filter_store: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];
        self.filter_store = output * self.damp2 + self.filter_store * self.damp1;
        self.buffer[self.index] = input + self.filter_store * self.feedback;
        self.index = (self.index + 1) % self.buffer.len();
        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.filter_store = 0.0;
    }
}

#[derive(Debug, Clone)]
struct AllpassFilter {
    buffer: Vec<f32>,
    index: usize,
}

impl AllpassFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            index: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.index];
        let output = -input + buffered;
        self.buffer[self.index] = input + buffered * 0.5;
        self.index = (self.index + 1) % self.buffer.len();
        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
    }
}

// Freeverb-style delay lengths (tuned for 44100 Hz, scaled for actual sample rate)
const COMB_LENGTHS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_LENGTHS: [usize; 4] = [556, 441, 341, 225];

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate / 44100.0;

        let comb_filters = COMB_LENGTHS
            .iter()
            .map(|&len| CombFilter::new((((len as f32) * scale) as usize).max(1)))
            .collect();

        let allpass_filters = ALLPASS_LENGTHS
            .iter()
            .map(|&len| AllpassFilter::new((((len as f32) * scale) as usize).max(1)))
            .collect();

        let mut reverb = Self {
            mix: 0.3,
            room_size: 0.7,
            damping: 0.5,
            comb_filters,
            allpass_filters,
            sample_rate,
        };
        reverb.update_parameters();
        reverb
    }

    /// Recompute internal parameters from public settings.
    pub fn update_parameters(&mut self) {
        let feedback = self.room_size * 0.28 + 0.7;
        let damp1 = self.damping * 0.4;
        let damp2 = 1.0 - damp1;

        for comb in &mut self.comb_filters {
            comb.feedback = feedback;
            comb.damp1 = damp1;
            comb.damp2 = damp2;
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let scale = sample_rate / 44100.0;

        self.comb_filters = COMB_LENGTHS
            .iter()
            .map(|&len| CombFilter::new(((len as f32) * scale) as usize))
            .collect();

        self.allpass_filters = ALLPASS_LENGTHS
            .iter()
            .map(|&len| AllpassFilter::new(((len as f32) * scale) as usize))
            .collect();

        self.update_parameters();
    }

    /// Process an audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        let channels = buffer.channels() as usize;
        let frames = buffer.frames();

        for frame in 0..frames {
            // Sum input to mono for reverb processing
            let mut input: f32 = 0.0;
            for ch in 0..channels {
                input += buffer.get(frame, ch as u16);
            }
            input /= channels as f32;

            // Parallel comb filters
            let mut wet: f32 = 0.0;
            for comb in &mut self.comb_filters {
                wet += comb.process(input);
            }

            // Series allpass filters
            for allpass in &mut self.allpass_filters {
                wet = allpass.process(wet);
            }

            // Mix dry/wet and write back
            for ch in 0..channels {
                let dry = buffer.get(frame, ch as u16);
                let output = dry * (1.0 - self.mix) + wet * self.mix;
                buffer.set(frame, ch as u16, output);
            }
        }
    }

    /// Reset all internal state.
    pub fn reset(&mut self) {
        for comb in &mut self.comb_filters {
            comb.reset();
        }
        for allpass in &mut self.allpass_filters {
            allpass.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverb_adds_tail() {
        let mut reverb = Reverb::new(48000.0);
        reverb.mix = 0.5;

        // Create an impulse
        let frames = 4800;
        let mut data = vec![0.0_f32; frames * 2];
        data[0] = 1.0; // Left impulse
        data[1] = 1.0; // Right impulse

        let mut buf = AudioBuffer::from_interleaved(data, 2);
        reverb.process(&mut buf);

        // Check that there's energy after the impulse (reverb tail)
        let tail_energy: f32 = (480..frames).map(|i| buf.get(i as u32, 0).powi(2)).sum();

        assert!(tail_energy > 0.001, "Reverb should produce a tail");
    }

    #[test]
    fn test_reverb_dry_only() {
        let mut reverb = Reverb::new(48000.0);
        reverb.mix = 0.0;

        let mut buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 2);
        reverb.process(&mut buf);

        assert_eq!(buf.get(0, 0), 0.5);
        assert_eq!(buf.get(0, 1), -0.5);
    }

    #[test]
    fn test_reverb_fully_wet() {
        let mut reverb = Reverb::new(48000.0);
        reverb.mix = 1.0;
        reverb.update_parameters();

        // Create an impulse
        let frames = 4800;
        let mut data = vec![0.0_f32; frames * 2];
        data[0] = 1.0;
        data[1] = 1.0;
        let mut buf = AudioBuffer::from_interleaved(data, 2);
        reverb.process(&mut buf);

        // At mix=1.0, frame 0 should be mostly wet (the dry component is 0)
        // The wet signal at frame 0 should come from the comb/allpass filters
        // which initially output 0 for the first frame (buffers are empty).
        // But there should be significant energy in the tail.
        let tail_energy: f32 = (480..frames).map(|i| buf.get(i as u32, 0).powi(2)).sum();
        assert!(
            tail_energy > 0.001,
            "Fully wet reverb should have a tail, energy={}",
            tail_energy
        );
    }

    #[test]
    fn test_room_size_changes_decay() {
        // Larger room_size should produce more reverb energy (longer decay)
        let frames = 9600;

        // Small room
        let mut reverb_small = Reverb::new(48000.0);
        reverb_small.mix = 1.0;
        reverb_small.room_size = 0.1;
        reverb_small.update_parameters();

        let mut data_small = vec![0.0_f32; frames];
        data_small[0] = 1.0;
        let mut buf_small = AudioBuffer::from_interleaved(data_small, 1);
        reverb_small.process(&mut buf_small);

        let energy_small: f32 = (2400..frames)
            .map(|i| buf_small.get(i as u32, 0).powi(2))
            .sum();

        // Large room
        let mut reverb_large = Reverb::new(48000.0);
        reverb_large.mix = 1.0;
        reverb_large.room_size = 1.0;
        reverb_large.update_parameters();

        let mut data_large = vec![0.0_f32; frames];
        data_large[0] = 1.0;
        let mut buf_large = AudioBuffer::from_interleaved(data_large, 1);
        reverb_large.process(&mut buf_large);

        let energy_large: f32 = (2400..frames)
            .map(|i| buf_large.get(i as u32, 0).powi(2))
            .sum();

        assert!(
            energy_large > energy_small,
            "Larger room should have more late tail energy: large={energy_large}, small={energy_small}"
        );
    }

    #[test]
    fn test_damping_effect() {
        // Higher damping should reduce high-frequency content in the reverb tail
        let frames = 9600;

        // Low damping
        let mut reverb_low = Reverb::new(48000.0);
        reverb_low.mix = 1.0;
        reverb_low.room_size = 0.8;
        reverb_low.damping = 0.0;
        reverb_low.update_parameters();

        let mut data_low = vec![0.0_f32; frames];
        data_low[0] = 1.0;
        let mut buf_low = AudioBuffer::from_interleaved(data_low, 1);
        reverb_low.process(&mut buf_low);

        // High damping
        let mut reverb_high = Reverb::new(48000.0);
        reverb_high.mix = 1.0;
        reverb_high.room_size = 0.8;
        reverb_high.damping = 1.0;
        reverb_high.update_parameters();

        let mut data_high = vec![0.0_f32; frames];
        data_high[0] = 1.0;
        let mut buf_high = AudioBuffer::from_interleaved(data_high, 1);
        reverb_high.process(&mut buf_high);

        // With high damping, successive samples should be smoother (lower variance).
        // Compute a rough measure: sum of absolute differences between consecutive samples
        // in the late tail.
        let roughness = |buf: &AudioBuffer, start: usize, end: usize| -> f32 {
            (start + 1..end)
                .map(|i| (buf.get(i as u32, 0) - buf.get((i - 1) as u32, 0)).abs())
                .sum::<f32>()
        };
        let rough_low = roughness(&buf_low, 2400, frames);
        let rough_high = roughness(&buf_high, 2400, frames);

        assert!(
            rough_high < rough_low,
            "High damping should produce smoother tail: rough_low={rough_low}, rough_high={rough_high}"
        );
    }

    #[test]
    fn test_reverb_new_at_44100() {
        let reverb = Reverb::new(44100.0);
        assert_eq!(reverb.sample_rate, 44100.0);
        assert_eq!(reverb.comb_filters.len(), 8);
        assert_eq!(reverb.allpass_filters.len(), 4);
    }

    #[test]
    fn test_reverb_new_at_96000() {
        let reverb = Reverb::new(96000.0);
        assert_eq!(reverb.sample_rate, 96000.0);
        // Buffer sizes should be scaled up for higher sample rate
        assert!(reverb.comb_filters[0].buffer.len() > COMB_LENGTHS[0]);
    }

    #[test]
    fn test_reverb_new_at_very_low_sample_rate() {
        // Very low sample rate: filter sizes are clamped to at least 1
        let reverb = Reverb::new(100.0);
        assert_eq!(reverb.comb_filters.len(), 8);
        for comb in &reverb.comb_filters {
            assert!(comb.buffer.len() >= 1, "Comb buffer should be at least 1");
        }
        for ap in &reverb.allpass_filters {
            assert!(ap.buffer.len() >= 1, "Allpass buffer should be at least 1");
        }
    }

    #[test]
    fn test_reverb_process_silence_stays_silent() {
        let mut reverb = Reverb::new(48000.0);
        reverb.mix = 0.5;
        let mut buf = AudioBuffer::new(2, 256);
        reverb.process(&mut buf);
        // Silence in = silence out
        for i in 0..256 {
            assert_eq!(buf.get(i, 0), 0.0);
            assert_eq!(buf.get(i, 1), 0.0);
        }
    }

    #[test]
    fn test_reverb_process_mono_buffer() {
        let mut reverb = Reverb::new(48000.0);
        reverb.mix = 0.5;
        let frames = 2400;
        let mut data = vec![0.0f32; frames];
        data[0] = 1.0;
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        reverb.process(&mut buf);
        // Should not panic and should produce a tail
        let tail_energy: f32 = (240..frames).map(|i| buf.get(i as u32, 0).powi(2)).sum();
        assert!(tail_energy > 0.0001, "Mono reverb should produce a tail");
    }

    #[test]
    fn test_reverb_reset_clears_state() {
        let mut reverb = Reverb::new(48000.0);
        reverb.mix = 1.0;

        // Feed an impulse
        let frames = 2400;
        let mut data = vec![0.0f32; frames];
        data[0] = 1.0;
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        reverb.process(&mut buf);

        // Reset
        reverb.reset();

        // Process silence: should produce silence (no leftover tail)
        let mut buf2 = AudioBuffer::new(1, 2400);
        reverb.process(&mut buf2);
        let energy: f32 = (0..2400u32).map(|i| buf2.get(i, 0).powi(2)).sum();
        assert!(
            energy < 1e-10,
            "After reset, processing silence should produce silence, got energy={energy}"
        );
    }

    #[test]
    fn test_reverb_mix_zero_preserves_dry() {
        let mut reverb = Reverb::new(48000.0);
        reverb.mix = 0.0;
        let data = vec![0.25, -0.25, 0.5, -0.5];
        let mut buf = AudioBuffer::from_interleaved(data.clone(), 2);
        reverb.process(&mut buf);
        for (i, &expected) in data.iter().enumerate() {
            assert!(
                (buf.as_interleaved()[i] - expected).abs() < 1e-6,
                "mix=0 should preserve dry signal at index {i}"
            );
        }
    }

    #[test]
    fn test_reverb_steady_signal() {
        let mut reverb = Reverb::new(48000.0);
        reverb.mix = 0.3;
        // Constant signal
        let data = vec![0.5f32; 4800];
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        reverb.process(&mut buf);
        // Output should not contain NaN or infinity
        for i in 0..4800u32 {
            let s = buf.get(i, 0);
            assert!(s.is_finite(), "Output should be finite at frame {i}");
        }
    }

    #[test]
    fn test_set_sample_rate_reinitializes() {
        let mut reverb = Reverb::new(48000.0);
        let old_comb_len = reverb.comb_filters[0].buffer.len();
        reverb.set_sample_rate(96000.0);
        let new_comb_len = reverb.comb_filters[0].buffer.len();
        assert!(
            new_comb_len > old_comb_len,
            "Doubling sample rate should increase comb buffer size"
        );
    }
}
