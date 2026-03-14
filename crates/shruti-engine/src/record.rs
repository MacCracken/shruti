use rtrb::{Consumer, Producer, RingBuffer};
use shruti_dsp::io::write_wav_file;
use shruti_dsp::{AudioBuffer, AudioFormat};

use std::path::Path;
use std::thread;

use crate::error::EngineError;

/// Manages recording from the RT thread to disk.
///
/// Uses a lock-free ring buffer: the RT callback pushes samples
/// into the producer, and a background thread drains the consumer
/// and writes to a WAV file when recording stops.
pub struct RecordManager {
    producer: Producer<f32>,
    accumulator_handle: Option<thread::JoinHandle<Vec<f32>>>,
}

impl RecordManager {
    /// Create a new RecordManager with the given ring buffer capacity (in samples).
    pub fn new(capacity: usize) -> Self {
        let (producer, consumer) = RingBuffer::new(capacity);

        let handle = thread::spawn(move || accumulate_samples(consumer));

        Self {
            producer,
            accumulator_handle: Some(handle),
        }
    }

    /// Push interleaved samples from the RT callback. Non-blocking.
    /// Drops samples if the ring buffer is full (preferable to blocking the RT thread).
    pub fn push_samples(&mut self, data: &[f32]) {
        for &sample in data {
            let _ = self.producer.push(sample);
        }
    }

    /// Stop recording and write the accumulated audio to a WAV file.
    pub fn finish(mut self, path: &Path, format: &AudioFormat) -> Result<(), EngineError> {
        // Drop the producer to signal the consumer thread to finish
        drop(std::mem::replace(&mut self.producer, RingBuffer::new(1).0));

        let handle = self.accumulator_handle.take().unwrap();
        let samples = handle
            .join()
            .map_err(|_| EngineError::Recording("recording thread panicked".into()))?;

        let buffer = AudioBuffer::from_interleaved(samples, format.channels);
        write_wav_file(path, &buffer, format).map_err(|e| EngineError::Recording(e.to_string()))?;

        Ok(())
    }
}

fn accumulate_samples(mut consumer: Consumer<f32>) -> Vec<f32> {
    let mut samples = Vec::new();

    loop {
        match consumer.pop() {
            Ok(sample) => samples.push(sample),
            Err(_) => {
                // Buffer empty — if the producer is gone, we're done
                if consumer.is_abandoned() {
                    // Drain any remaining
                    while let Ok(sample) = consumer.pop() {
                        samples.push(sample);
                    }
                    break;
                }
                thread::yield_now();
            }
        }
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_manager_creation() {
        // Should not panic with various capacities
        let _rm = RecordManager::new(1024);
        let _rm = RecordManager::new(1);
    }

    #[test]
    fn test_push_samples() {
        let mut rm = RecordManager::new(1024);
        let data = vec![0.1, 0.2, 0.3, 0.4];
        rm.push_samples(&data);
        // Push more
        rm.push_samples(&[0.5, 0.6]);
        // No panic is the success condition; actual data verified via finish()
    }

    #[test]
    fn test_push_samples_empty() {
        let mut rm = RecordManager::new(1024);
        rm.push_samples(&[]);
        // No panic
    }

    #[test]
    fn test_accumulate_samples_basic() {
        let (mut producer, consumer) = rtrb::RingBuffer::new(64);

        let handle = thread::spawn(move || accumulate_samples(consumer));

        // Push some data then drop the producer to signal completion
        for &s in &[1.0f32, 2.0, 3.0, 4.0] {
            let _ = producer.push(s);
        }
        drop(producer);

        let result = handle.join().unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_accumulate_samples_empty() {
        let (producer, consumer) = rtrb::RingBuffer::new(64);
        let handle = thread::spawn(move || accumulate_samples(consumer));
        drop(producer);

        let result = handle.join().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_finish_writes_wav() {
        let mut rm = RecordManager::new(1024);
        let samples = vec![0.1f32, -0.1, 0.2, -0.2, 0.3, -0.3, 0.4, -0.4];
        rm.push_samples(&samples);

        let dir = std::env::temp_dir();
        let path = dir.join("shruti_test_record.wav");

        let format = AudioFormat {
            sample_rate: 44100,
            channels: 2,
            buffer_size: 256,
        };

        rm.finish(&path, &format).unwrap();

        // Verify file was created
        assert!(path.exists());
        // Clean up
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_finish_empty_recording() {
        let rm = RecordManager::new(1024);

        let dir = std::env::temp_dir();
        let path = dir.join("shruti_test_empty_record.wav");

        let format = AudioFormat {
            sample_rate: 44100,
            channels: 1,
            buffer_size: 256,
        };

        rm.finish(&path, &format).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_push_samples_overflow_does_not_panic() {
        // Ring buffer of capacity 4, push 10 samples
        let mut rm = RecordManager::new(4);
        let data = vec![1.0; 10];
        rm.push_samples(&data);
        // Should not panic — overflow samples are silently dropped
    }

    #[test]
    fn test_push_overflow_then_finish() {
        // Overflow the buffer, then finish — should produce a valid WAV
        // (with only the samples that fit).
        let mut rm = RecordManager::new(8);
        let data: Vec<f32> = (0..20).map(|i| i as f32 * 0.05).collect();
        rm.push_samples(&data);

        let dir = std::env::temp_dir();
        let path = dir.join("shruti_test_overflow_record.wav");

        let format = AudioFormat {
            sample_rate: 44100,
            channels: 1,
            buffer_size: 256,
        };

        rm.finish(&path, &format).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_accumulate_samples_large_batch() {
        let (mut producer, consumer) = rtrb::RingBuffer::new(4096);
        let handle = thread::spawn(move || accumulate_samples(consumer));

        let samples: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
        for &s in &samples {
            let _ = producer.push(s);
        }
        drop(producer);

        let result = handle.join().unwrap();
        assert_eq!(result.len(), 1000);
        assert!((result[0] - 0.0).abs() < 1e-7);
        assert!((result[999] - 0.999).abs() < 1e-7);
    }

    #[test]
    fn test_finish_stereo_recording() {
        let mut rm = RecordManager::new(2048);
        // Push stereo interleaved: L R L R ...
        let samples: Vec<f32> = (0..200)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        rm.push_samples(&samples);

        let dir = std::env::temp_dir();
        let path = dir.join("shruti_test_stereo_record.wav");

        let format = AudioFormat {
            sample_rate: 48000,
            channels: 2,
            buffer_size: 512,
        };

        rm.finish(&path, &format).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }
}
