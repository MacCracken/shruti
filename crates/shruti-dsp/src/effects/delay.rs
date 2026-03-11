use crate::buffer::AudioBuffer;
use crate::format::Sample;

/// Stereo delay effect with feedback and dry/wet mix.
#[derive(Debug, Clone)]
pub struct Delay {
    /// Delay time in seconds.
    pub time: f32,
    /// Feedback amount (0.0 to <1.0).
    pub feedback: f32,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    pub mix: f32,
    sample_rate: f32,
    // Per-channel circular buffers
    buffers: Vec<Vec<Sample>>,
    write_pos: usize,
}

impl Delay {
    pub fn new(sample_rate: f32) -> Self {
        let max_delay_samples = (sample_rate * 5.0) as usize; // Up to 5 seconds
        Self {
            time: 0.25,
            feedback: 0.4,
            mix: 0.3,
            sample_rate,
            buffers: vec![vec![0.0; max_delay_samples]; 2],
            write_pos: 0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let max_delay_samples = (sample_rate * 5.0) as usize;
        self.buffers = vec![vec![0.0; max_delay_samples]; 2];
        self.write_pos = 0;
    }

    fn delay_samples(&self) -> usize {
        (self.time * self.sample_rate) as usize
    }

    /// Process an audio buffer in place.
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        let channels = buffer.channels() as usize;
        let frames = buffer.frames();
        let delay_samples = self.delay_samples();
        let buf_len = self.buffers[0].len();

        if delay_samples == 0 || buf_len == 0 {
            return;
        }

        for frame in 0..frames {
            for ch in 0..channels.min(2) {
                let input = buffer.get(frame, ch as u16);

                // Read from delay line
                let read_pos = (self.write_pos + buf_len - delay_samples) % buf_len;
                let delayed = self.buffers[ch][read_pos];

                // Write input + feedback to delay line
                self.buffers[ch][self.write_pos] = input + delayed * self.feedback;

                // Mix
                let output = input * (1.0 - self.mix) + delayed * self.mix;
                buffer.set(frame, ch as u16, output);
            }
            self.write_pos = (self.write_pos + 1) % buf_len;
        }
    }

    /// Reset all delay buffers.
    pub fn reset(&mut self) {
        for buf in &mut self.buffers {
            buf.fill(0.0);
        }
        self.write_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_produces_echo() {
        let mut delay = Delay::new(48000.0);
        delay.time = 0.01; // 10ms = 480 samples
        delay.feedback = 0.0;
        delay.mix = 1.0;

        // Impulse at frame 0
        let frames = 960;
        let mut data = vec![0.0_f32; frames];
        data[0] = 1.0;

        let mut buf = AudioBuffer::from_interleaved(data, 1);
        delay.process(&mut buf);

        // The impulse should appear at frame 480 (10ms later)
        assert!(
            buf.get(480, 0).abs() > 0.5,
            "Echo should appear at delay time"
        );
        // Original frame should be near 0 (fully wet, no dry)
        assert!(
            buf.get(0, 0).abs() < 0.01,
            "Dry signal suppressed at mix=1.0"
        );
    }

    #[test]
    fn test_delay_dry_passthrough() {
        let mut delay = Delay::new(48000.0);
        delay.mix = 0.0;

        let mut buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 2);
        delay.process(&mut buf);

        assert_eq!(buf.get(0, 0), 0.5);
        assert_eq!(buf.get(0, 1), -0.5);
    }

    #[test]
    fn test_feedback_produces_repeating_echoes() {
        let mut delay = Delay::new(48000.0);
        delay.time = 0.01; // 480 samples
        delay.feedback = 0.5;
        delay.mix = 1.0; // fully wet so we only see delayed signal

        let frames = 2400; // enough for multiple echoes
        let mut data = vec![0.0_f32; frames];
        data[0] = 1.0; // impulse
        let mut buf = AudioBuffer::from_interleaved(data, 1);
        delay.process(&mut buf);

        // First echo at sample 480
        let echo1 = buf.get(480, 0).abs();
        // Second echo at sample 960 (feedback * first echo)
        let echo2 = buf.get(960, 0).abs();
        // Third echo at sample 1440
        let echo3 = buf.get(1440, 0).abs();

        assert!(echo1 > 0.5, "First echo should be strong: {echo1}");
        assert!(
            echo2 > 0.1,
            "Second echo should exist with feedback: {echo2}"
        );
        assert!(echo3 > 0.01, "Third echo should exist: {echo3}");
        // Each successive echo should be quieter
        assert!(echo1 > echo2, "Echoes should decay: {echo1} > {echo2}");
        assert!(echo2 > echo3, "Echoes should decay: {echo2} > {echo3}");
    }

    #[test]
    fn test_delay_mix_parameter() {
        let mut delay_dry = Delay::new(48000.0);
        delay_dry.time = 0.01;
        delay_dry.feedback = 0.0;
        delay_dry.mix = 0.0;

        let mut delay_wet = Delay::new(48000.0);
        delay_wet.time = 0.01;
        delay_wet.feedback = 0.0;
        delay_wet.mix = 1.0;

        let frames = 960;
        let mut data_dry = vec![0.0_f32; frames];
        data_dry[0] = 1.0;
        let data_wet = data_dry.clone();

        let mut buf_dry = AudioBuffer::from_interleaved(data_dry, 1);
        let mut buf_wet = AudioBuffer::from_interleaved(data_wet, 1);

        delay_dry.process(&mut buf_dry);
        delay_wet.process(&mut buf_wet);

        // mix=0: frame 0 should keep dry signal
        assert!(
            (buf_dry.get(0, 0) - 1.0).abs() < 0.01,
            "mix=0 should pass dry: {}",
            buf_dry.get(0, 0)
        );
        // mix=1: frame 0 should have no dry signal
        assert!(
            buf_wet.get(0, 0).abs() < 0.01,
            "mix=1 should suppress dry at frame 0: {}",
            buf_wet.get(0, 0)
        );
        // mix=1: echo should appear at delay time
        assert!(
            buf_wet.get(480, 0).abs() > 0.5,
            "mix=1 should have echo at 480: {}",
            buf_wet.get(480, 0)
        );
    }
}
