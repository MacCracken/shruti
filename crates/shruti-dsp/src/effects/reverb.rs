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
            .map(|&len| CombFilter::new(((len as f32) * scale) as usize))
            .collect();

        let allpass_filters = ALLPASS_LENGTHS
            .iter()
            .map(|&len| AllpassFilter::new(((len as f32) * scale) as usize))
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
}
