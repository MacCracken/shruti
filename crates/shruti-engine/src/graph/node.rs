use shruti_dsp::{AudioBuffer, Sample};
use std::sync::atomic::{AtomicU32, Ordering};

/// Unique identifier for a node in the audio graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

static NEXT_NODE_ID: AtomicU32 = AtomicU32::new(1);

impl NodeId {
    pub fn next() -> Self {
        Self(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Trait for audio processing nodes.
pub trait AudioNode: Send {
    fn name(&self) -> &str;
    fn num_inputs(&self) -> usize;
    fn num_outputs(&self) -> usize;

    /// Process one buffer cycle.
    /// `inputs` contains mixed input buffers (one per input port).
    /// Write output into `output`.
    fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer);

    /// Returns true when the node has finished producing output (e.g. file ended).
    fn is_finished(&self) -> bool {
        false
    }
}

/// Plays back a pre-loaded audio buffer.
pub struct FilePlayerNode {
    buffer: AudioBuffer,
    position: usize,
    looping: bool,
}

impl FilePlayerNode {
    pub fn new(buffer: AudioBuffer, looping: bool) -> Self {
        Self {
            buffer,
            position: 0,
            looping,
        }
    }

    pub fn reset(&mut self) {
        self.position = 0;
    }
}

impl AudioNode for FilePlayerNode {
    fn name(&self) -> &str {
        "file_player"
    }

    fn num_inputs(&self) -> usize {
        0
    }

    fn num_outputs(&self) -> usize {
        1
    }

    fn process(&mut self, _inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
        let out_frames = output.frames() as usize;
        let src_frames = self.buffer.frames() as usize;
        let channels = output.channels().min(self.buffer.channels()) as usize;

        for frame in 0..out_frames {
            if self.position >= src_frames {
                if self.looping {
                    self.position = 0;
                } else {
                    // Fill remaining with silence
                    for ch in 0..channels {
                        output.set(frame as u32, ch as u16, 0.0);
                    }
                    continue;
                }
            }

            for ch in 0..channels {
                let sample = self.buffer.get(self.position as u32, ch as u16);
                output.set(frame as u32, ch as u16, sample);
            }
            self.position += 1;
        }
    }

    fn is_finished(&self) -> bool {
        !self.looping && self.position >= self.buffer.frames() as usize
    }
}

/// Simple gain node for volume control.
pub struct GainNode {
    pub gain: Sample,
}

impl GainNode {
    pub fn new(gain: Sample) -> Self {
        Self { gain }
    }
}

impl AudioNode for GainNode {
    fn name(&self) -> &str {
        "gain"
    }

    fn num_inputs(&self) -> usize {
        1
    }

    fn num_outputs(&self) -> usize {
        1
    }

    fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
        if let Some(input) = inputs.first() {
            output
                .as_interleaved_mut()
                .copy_from_slice(input.as_interleaved());
            output.apply_gain(self.gain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_player_basic() {
        let src = AudioBuffer::from_interleaved(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], 2);
        let mut player = FilePlayerNode::new(src, false);
        let mut out = AudioBuffer::new(2, 3);

        player.process(&[], &mut out);

        assert_eq!(out.get(0, 0), 0.1);
        assert_eq!(out.get(0, 1), 0.2);
        assert_eq!(out.get(2, 0), 0.5);
        assert!(player.is_finished());
    }

    #[test]
    fn test_file_player_looping() {
        let src = AudioBuffer::from_interleaved(vec![1.0, -1.0], 2);
        let mut player = FilePlayerNode::new(src, true);
        let mut out = AudioBuffer::new(2, 3);

        player.process(&[], &mut out);

        assert_eq!(out.get(0, 0), 1.0);
        assert_eq!(out.get(1, 0), 1.0);
        assert_eq!(out.get(2, 0), 1.0);
        assert!(!player.is_finished());
    }

    #[test]
    fn test_gain_node() {
        let input = AudioBuffer::from_interleaved(vec![1.0, -1.0, 0.5, -0.5], 2);
        let mut gain = GainNode::new(0.5);
        let mut out = AudioBuffer::new(2, 2);

        gain.process(&[&input], &mut out);

        assert_eq!(out.get(0, 0), 0.5);
        assert_eq!(out.get(0, 1), -0.5);
    }
}
