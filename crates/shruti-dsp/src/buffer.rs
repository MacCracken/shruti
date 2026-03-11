use crate::format::Sample;

/// Interleaved audio buffer with channel-based access.
///
/// Stores samples interleaved (L R L R ...) for cache locality,
/// but provides per-channel slice access for processing.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    data: Vec<Sample>,
    channels: u16,
    frames: u32,
}

impl AudioBuffer {
    /// Create a new zero-filled buffer.
    pub fn new(channels: u16, frames: u32) -> Self {
        Self {
            data: vec![0.0; channels as usize * frames as usize],
            channels,
            frames,
        }
    }

    /// Create a buffer from existing interleaved sample data.
    pub fn from_interleaved(data: Vec<Sample>, channels: u16) -> Self {
        let frames = data.len() as u32 / channels as u32;
        Self {
            data,
            channels,
            frames,
        }
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn frames(&self) -> u32 {
        self.frames
    }

    pub fn sample_count(&self) -> usize {
        self.data.len()
    }

    /// Get a single sample at (frame, channel).
    pub fn get(&self, frame: u32, channel: u16) -> Sample {
        self.data[frame as usize * self.channels as usize + channel as usize]
    }

    /// Set a single sample at (frame, channel).
    pub fn set(&mut self, frame: u32, channel: u16, value: Sample) {
        self.data[frame as usize * self.channels as usize + channel as usize] = value;
    }

    /// Access the raw interleaved data.
    pub fn as_interleaved(&self) -> &[Sample] {
        &self.data
    }

    /// Access the raw interleaved data mutably.
    pub fn as_interleaved_mut(&mut self) -> &mut [Sample] {
        &mut self.data
    }

    /// Copy samples for a single channel into the provided slice.
    pub fn read_channel(&self, channel: u16, out: &mut [Sample]) {
        let ch = channel as usize;
        let stride = self.channels as usize;
        for (i, sample) in out.iter_mut().enumerate() {
            *sample = self.data[i * stride + ch];
        }
    }

    /// Write samples for a single channel from the provided slice.
    pub fn write_channel(&mut self, channel: u16, src: &[Sample]) {
        let ch = channel as usize;
        let stride = self.channels as usize;
        for (i, &sample) in src.iter().enumerate() {
            self.data[i * stride + ch] = sample;
        }
    }

    /// Fill the entire buffer with silence.
    pub fn clear(&mut self) {
        self.data.fill(0.0);
    }

    /// Mix (add) another buffer's contents into this one.
    pub fn mix_from(&mut self, other: &AudioBuffer) {
        assert_eq!(self.channels, other.channels);
        let len = self.data.len().min(other.data.len());
        for i in 0..len {
            self.data[i] += other.data[i];
        }
    }

    /// Apply gain to the entire buffer.
    pub fn apply_gain(&mut self, gain: Sample) {
        for sample in &mut self.data {
            *sample *= gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_silent() {
        let buf = AudioBuffer::new(2, 128);
        assert!(buf.as_interleaved().iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_get_set() {
        let mut buf = AudioBuffer::new(2, 4);
        buf.set(1, 0, 0.5);
        buf.set(1, 1, -0.5);
        assert_eq!(buf.get(1, 0), 0.5);
        assert_eq!(buf.get(1, 1), -0.5);
        assert_eq!(buf.get(0, 0), 0.0);
    }

    #[test]
    fn test_channel_read_write() {
        let mut buf = AudioBuffer::new(2, 4);
        let left = [0.1, 0.2, 0.3, 0.4];
        buf.write_channel(0, &left);

        let mut out = [0.0; 4];
        buf.read_channel(0, &mut out);
        assert_eq!(out, left);

        // Right channel should still be silent
        buf.read_channel(1, &mut out);
        assert_eq!(out, [0.0; 4]);
    }

    #[test]
    fn test_mix_from() {
        let mut a = AudioBuffer::from_interleaved(vec![0.5, 0.5, 0.5, 0.5], 2);
        let b = AudioBuffer::from_interleaved(vec![0.3, 0.3, 0.3, 0.3], 2);
        a.mix_from(&b);
        for &s in a.as_interleaved() {
            assert!((s - 0.8).abs() < 1e-6);
        }
    }

    #[test]
    fn test_apply_gain() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0, -1.0, 0.5, -0.5], 2);
        buf.apply_gain(0.5);
        assert_eq!(buf.as_interleaved(), &[0.5, -0.5, 0.25, -0.25]);
    }
}
