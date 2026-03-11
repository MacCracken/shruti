use rtrb::{Consumer, Producer, RingBuffer};
use shruti_dsp::io::write_wav_file;
use shruti_dsp::{AudioBuffer, AudioFormat};

use std::path::Path;
use std::thread;

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
    pub fn finish(
        mut self,
        path: &Path,
        format: &AudioFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Drop the producer to signal the consumer thread to finish
        drop(std::mem::replace(&mut self.producer, RingBuffer::new(1).0));

        let handle = self.accumulator_handle.take().unwrap();
        let samples = handle.join().map_err(|_| "recording thread panicked")?;

        let buffer = AudioBuffer::from_interleaved(samples, format.channels);
        write_wav_file(path, &buffer, format)?;

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
