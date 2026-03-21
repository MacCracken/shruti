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
        let channels = channels.max(1);
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
        debug_assert!(
            (frame as usize * self.channels as usize + channel as usize) < self.data.len(),
            "AudioBuffer::get out of bounds: frame={frame}, channel={channel}, frames={}, channels={}",
            self.frames,
            self.channels
        );
        self.data[frame as usize * self.channels as usize + channel as usize]
    }

    /// Set a single sample at (frame, channel).
    #[inline]
    pub fn set(&mut self, frame: u32, channel: u16, value: Sample) {
        debug_assert!(
            (frame as usize * self.channels as usize + channel as usize) < self.data.len(),
            "AudioBuffer::set out of bounds: frame={frame}, channel={channel}, frames={}, channels={}",
            self.frames,
            self.channels
        );
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

// ── dhvani buffer utilities ──────────────────────────────────────

pub use dhvani::buffer::resample::ResampleQuality;

/// Re-export raw format conversion functions (operate on slices, not AudioBuffer).
pub use dhvani::buffer::convert::{f32_to_i16, i16_to_f32};

/// Helper: convert shruti AudioBuffer to dhvani AudioBuffer.
fn to_dhvani(buf: &AudioBuffer, sample_rate: u32) -> Option<dhvani::buffer::AudioBuffer> {
    dhvani::buffer::AudioBuffer::from_interleaved(
        buf.as_interleaved().to_vec(),
        buf.channels() as u32,
        sample_rate,
    )
    .ok()
}

/// Helper: convert dhvani AudioBuffer back to shruti AudioBuffer.
fn from_dhvani(dbuf: &dhvani::buffer::AudioBuffer) -> AudioBuffer {
    AudioBuffer::from_interleaved(dbuf.samples.clone(), dbuf.channels as u16)
}

/// Resample using linear interpolation (fast, lower quality).
pub fn resample_linear(
    buf: &AudioBuffer,
    sample_rate: u32,
    target_rate: u32,
) -> Option<AudioBuffer> {
    if sample_rate == target_rate {
        return Some(buf.clone());
    }
    let dbuf = to_dhvani(buf, sample_rate)?;
    dhvani::buffer::resample_linear(&dbuf, target_rate)
        .ok()
        .map(|r| from_dhvani(&r))
}

/// Resample using windowed sinc interpolation (high quality).
pub fn resample_sinc(
    buf: &AudioBuffer,
    sample_rate: u32,
    target_rate: u32,
    quality: ResampleQuality,
) -> Option<AudioBuffer> {
    if sample_rate == target_rate {
        return Some(buf.clone());
    }
    let dbuf = to_dhvani(buf, sample_rate)?;
    dhvani::buffer::resample::resample_sinc(&dbuf, target_rate, quality)
        .ok()
        .map(|r| from_dhvani(&r))
}

/// Convert a mono buffer to stereo (duplicate channels).
pub fn mono_to_stereo(buf: &AudioBuffer, sample_rate: u32) -> Option<AudioBuffer> {
    let dbuf = to_dhvani(buf, sample_rate)?;
    dhvani::buffer::convert::mono_to_stereo(&dbuf)
        .ok()
        .map(|r| from_dhvani(&r))
}

/// Mix multiple buffers together (additive sum).
pub fn mix_buffers(buffers: &[&AudioBuffer], sample_rate: u32) -> Option<AudioBuffer> {
    let dhvani_bufs: Vec<dhvani::buffer::AudioBuffer> = buffers
        .iter()
        .filter_map(|b| to_dhvani(b, sample_rate))
        .collect();
    let refs: Vec<&dhvani::buffer::AudioBuffer> = dhvani_bufs.iter().collect();
    dhvani::buffer::mix(&refs).ok().map(|r| from_dhvani(&r))
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

    // ── dhvani buffer utility tests ──────────────────────────────

    #[test]
    fn resample_linear_changes_length() {
        let samples: Vec<f32> = (0..200).map(|i| (i as f32 * 0.01).sin()).collect();
        let buf = AudioBuffer::from_interleaved(samples, 2);
        let result = resample_linear(&buf, 44100, 48000).unwrap();
        assert!(result.frames() > buf.frames());
        assert_eq!(result.channels(), 2);
    }

    #[test]
    fn resample_linear_same_rate_noop() {
        let buf = AudioBuffer::from_interleaved(vec![0.1, -0.1, 0.2, -0.2], 2);
        let result = resample_linear(&buf, 44100, 44100).unwrap();
        assert_eq!(result.frames(), buf.frames());
    }

    #[test]
    fn resample_sinc_changes_length() {
        let samples: Vec<f32> = (0..200).map(|i| (i as f32 * 0.01).sin()).collect();
        let buf = AudioBuffer::from_interleaved(samples, 2);
        let result = resample_sinc(&buf, 44100, 48000, ResampleQuality::Good).unwrap();
        assert!(result.frames() > buf.frames());
    }

    #[test]
    fn i16_f32_roundtrip() {
        let original: Vec<i16> = vec![16384, -16384, 0, 32767];
        let floats = i16_to_f32(&original);
        let back = f32_to_i16(&floats);
        for (o, b) in original.iter().zip(back.iter()) {
            assert!((*o as i32 - *b as i32).abs() <= 1, "{o} != {b}");
        }
    }

    #[test]
    fn mono_to_stereo_doubles_channels() {
        let buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 1);
        let stereo = mono_to_stereo(&buf, 48000).unwrap();
        assert_eq!(stereo.channels(), 2);
        assert_eq!(stereo.frames(), buf.frames());
    }

    #[test]
    fn mix_buffers_sums() {
        let a = AudioBuffer::from_interleaved(vec![0.3, 0.3, 0.3, 0.3], 2);
        let b = AudioBuffer::from_interleaved(vec![0.2, 0.2, 0.2, 0.2], 2);
        let mixed = mix_buffers(&[&a, &b], 48000).unwrap();
        assert_eq!(mixed.channels(), 2);
        for s in mixed.as_interleaved() {
            assert!((s - 0.5).abs() < 0.01, "expected 0.5, got {s}");
        }
    }
}
