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
    #[inline]
    pub fn get(&self, frame: u32, channel: u16) -> Sample {
        self.data[frame as usize * self.channels as usize + channel as usize]
    }

    /// Set a single sample at (frame, channel).
    #[inline]
    pub fn set(&mut self, frame: u32, channel: u16, value: Sample) {
        self.data[frame as usize * self.channels as usize + channel as usize] = value;
    }

    /// Access the raw interleaved data (zero-copy: returns a slice into the internal buffer).
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

    #[test]
    fn test_from_interleaved_odd_sample_count_truncates_frames() {
        // 7 samples with 2 channels: 7/2 = 3 frames (integer division), last sample is orphaned
        let buf = AudioBuffer::from_interleaved(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7], 2);
        assert_eq!(buf.frames(), 3);
        assert_eq!(buf.channels(), 2);
        // The raw data still contains all 7 samples
        assert_eq!(buf.sample_count(), 7);
        // But frame-based access only reaches frames 0..3
        assert_eq!(buf.get(0, 0), 0.1);
        assert_eq!(buf.get(2, 1), 0.6);
    }

    #[test]
    fn test_from_interleaved_single_channel_odd() {
        // 5 samples, 3 channels: 5/3 = 1 frame
        let buf = AudioBuffer::from_interleaved(vec![0.1, 0.2, 0.3, 0.4, 0.5], 3);
        assert_eq!(buf.frames(), 1);
        assert_eq!(buf.channels(), 3);
        assert_eq!(buf.get(0, 0), 0.1);
        assert_eq!(buf.get(0, 1), 0.2);
        assert_eq!(buf.get(0, 2), 0.3);
    }

    #[test]
    fn test_as_interleaved_returns_raw_data() {
        let data = vec![0.1, 0.2, 0.3, 0.4];
        let buf = AudioBuffer::from_interleaved(data.clone(), 2);
        assert_eq!(buf.as_interleaved(), &data[..]);
    }

    #[test]
    fn test_as_interleaved_mut_allows_modification() {
        let mut buf = AudioBuffer::new(2, 2);
        let raw = buf.as_interleaved_mut();
        raw[0] = 0.5;
        raw[1] = -0.5;
        raw[2] = 0.25;
        raw[3] = -0.25;
        assert_eq!(buf.get(0, 0), 0.5);
        assert_eq!(buf.get(0, 1), -0.5);
        assert_eq!(buf.get(1, 0), 0.25);
        assert_eq!(buf.get(1, 1), -0.25);
    }

    #[test]
    fn test_apply_gain_zero() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0, -1.0, 0.5, -0.5], 2);
        buf.apply_gain(0.0);
        assert!(buf.as_interleaved().iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_apply_gain_negative() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0, 0.5], 1);
        buf.apply_gain(-1.0);
        assert_eq!(buf.as_interleaved(), &[-1.0, -0.5]);
    }

    #[test]
    fn test_mix_from_different_lengths() {
        // When buffers have different data lengths, mix_from uses the smaller
        let mut a = AudioBuffer::from_interleaved(vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5], 2);
        let b = AudioBuffer::from_interleaved(vec![0.3, 0.3], 2);
        a.mix_from(&b);
        // Only first 2 samples should be mixed
        assert!((a.as_interleaved()[0] - 0.8).abs() < 1e-6);
        assert!((a.as_interleaved()[1] - 0.8).abs() < 1e-6);
        // Remaining should be unchanged
        assert_eq!(a.as_interleaved()[2], 0.5);
        assert_eq!(a.as_interleaved()[5], 0.5);
    }

    #[test]
    fn test_clear_zeroes_all_samples() {
        let mut buf = AudioBuffer::from_interleaved(vec![0.7, -0.3, 0.5, -0.1], 2);
        buf.clear();
        assert!(buf.as_interleaved().iter().all(|&s| s == 0.0));
        assert_eq!(buf.frames(), 2);
        assert_eq!(buf.channels(), 2);
    }

    #[test]
    fn test_empty_buffer() {
        let buf = AudioBuffer::new(2, 0);
        assert_eq!(buf.frames(), 0);
        assert_eq!(buf.channels(), 2);
        assert_eq!(buf.sample_count(), 0);
        assert!(buf.as_interleaved().is_empty());
    }

    #[test]
    fn test_single_sample_buffer() {
        let mut buf = AudioBuffer::new(1, 1);
        assert_eq!(buf.frames(), 1);
        assert_eq!(buf.sample_count(), 1);
        buf.set(0, 0, 0.42);
        assert_eq!(buf.get(0, 0), 0.42);
    }

    #[test]
    fn test_large_buffer() {
        let frames = 192000; // 4 seconds at 48kHz
        let buf = AudioBuffer::new(2, frames);
        assert_eq!(buf.frames(), frames);
        assert_eq!(buf.sample_count(), frames as usize * 2);
        assert!(buf.as_interleaved().iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_from_interleaved_empty_vec() {
        let buf = AudioBuffer::from_interleaved(vec![], 2);
        assert_eq!(buf.frames(), 0);
        assert_eq!(buf.sample_count(), 0);
    }

    #[test]
    fn test_apply_gain_on_empty_buffer() {
        let mut buf = AudioBuffer::new(2, 0);
        buf.apply_gain(2.0); // should not panic
        assert_eq!(buf.sample_count(), 0);
    }

    #[test]
    fn test_clear_on_empty_buffer() {
        let mut buf = AudioBuffer::new(1, 0);
        buf.clear(); // should not panic
    }

    #[test]
    fn test_mix_from_empty_buffers() {
        let mut a = AudioBuffer::new(2, 0);
        let b = AudioBuffer::new(2, 0);
        a.mix_from(&b); // should not panic
    }

    #[test]
    fn test_as_interleaved_is_zero_copy_pointer_identity() {
        // Verify as_interleaved() returns a direct reference to internal data (zero-copy).
        // Calling it twice should return pointers to the same memory.
        let buf = AudioBuffer::from_interleaved(vec![0.1, 0.2, 0.3, 0.4], 2);
        let slice1 = buf.as_interleaved();
        let slice2 = buf.as_interleaved();
        assert_eq!(
            slice1.as_ptr(),
            slice2.as_ptr(),
            "as_interleaved() should return the same pointer (zero-copy)"
        );
        assert_eq!(slice1.len(), 4);
    }

    #[test]
    fn test_as_interleaved_mut_is_zero_copy_pointer_identity() {
        // Verify as_interleaved_mut() also returns a direct reference (zero-copy).
        let mut buf = AudioBuffer::from_interleaved(vec![0.1, 0.2, 0.3, 0.4], 2);
        let ptr = buf.as_interleaved_mut().as_ptr();
        let ptr2 = buf.as_interleaved().as_ptr();
        assert_eq!(
            ptr, ptr2,
            "Mutable and immutable interleaved access should point to the same data"
        );
    }
}
