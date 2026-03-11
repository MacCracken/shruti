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

        if src_frames == 0 {
            // Empty buffer — fill with silence regardless of looping mode.
            for frame in 0..out_frames {
                for ch in 0..channels {
                    output.set(frame as u32, ch as u16, 0.0);
                }
            }
            return;
        }

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

    #[test]
    fn test_file_player_empty_buffer_no_loop() {
        let src = AudioBuffer::from_interleaved(vec![], 2);
        let mut player = FilePlayerNode::new(src, false);
        let mut out = AudioBuffer::from_interleaved(vec![1.0; 4], 2);

        player.process(&[], &mut out);

        // Output should remain as-is since channels min(2,2)=2 but frames=0
        // Actually: src_frames==0 branch fills silence for out_frames
        // However channels = min(out.channels, self.buffer.channels) and
        // self.buffer has 0 frames but channels is 2 (from_interleaved with empty vec and ch=2 => frames=0)
        // So silence loop runs for 0..out_frames, 0..channels
        for &s in out.as_interleaved() {
            assert_eq!(s, 0.0);
        }
        // Empty non-looping: position(0) >= frames(0), so is_finished
        assert!(player.is_finished());
    }

    #[test]
    fn test_file_player_empty_buffer_looping() {
        let src = AudioBuffer::from_interleaved(vec![], 2);
        let mut player = FilePlayerNode::new(src, true);
        let mut out = AudioBuffer::from_interleaved(vec![1.0; 4], 2);

        player.process(&[], &mut out);

        for &s in out.as_interleaved() {
            assert_eq!(s, 0.0);
        }
        // Looping never reports finished
        assert!(!player.is_finished());
    }

    #[test]
    fn test_file_player_reset() {
        let src = AudioBuffer::from_interleaved(vec![0.5, -0.5], 1);
        let mut player = FilePlayerNode::new(src, false);
        let mut out = AudioBuffer::new(1, 2);

        player.process(&[], &mut out);
        assert_eq!(out.get(0, 0), 0.5);
        assert_eq!(out.get(1, 0), -0.5);
        assert!(player.is_finished());

        // Reset and play again
        player.reset();
        assert!(!player.is_finished());

        let mut out2 = AudioBuffer::new(1, 2);
        player.process(&[], &mut out2);
        assert_eq!(out2.get(0, 0), 0.5);
        assert_eq!(out2.get(1, 0), -0.5);
        assert!(player.is_finished());
    }

    #[test]
    fn test_file_player_non_looping_fills_silence_past_end() {
        // Source has 1 frame, output requests 3
        let src = AudioBuffer::from_interleaved(vec![0.9, -0.9], 2);
        let mut player = FilePlayerNode::new(src, false);
        let mut out = AudioBuffer::new(2, 3);

        player.process(&[], &mut out);

        assert_eq!(out.get(0, 0), 0.9);
        assert_eq!(out.get(0, 1), -0.9);
        // Frames past the source should be silence
        assert_eq!(out.get(1, 0), 0.0);
        assert_eq!(out.get(1, 1), 0.0);
        assert_eq!(out.get(2, 0), 0.0);
        assert_eq!(out.get(2, 1), 0.0);
        assert!(player.is_finished());
    }

    #[test]
    fn test_file_player_is_finished_states() {
        let src = AudioBuffer::from_interleaved(vec![1.0, 2.0], 1);
        let mut player = FilePlayerNode::new(src, false);

        // Not finished before processing
        assert!(!player.is_finished());

        let mut out = AudioBuffer::new(1, 1);
        player.process(&[], &mut out);
        // Consumed 1 of 2 frames — not finished
        assert!(!player.is_finished());

        player.process(&[], &mut out);
        // Consumed 2 of 2 — finished
        assert!(player.is_finished());
    }

    #[test]
    fn test_file_player_looping_never_finished() {
        let src = AudioBuffer::from_interleaved(vec![1.0], 1);
        let mut player = FilePlayerNode::new(src, true);

        // Process many cycles
        let mut out = AudioBuffer::new(1, 100);
        player.process(&[], &mut out);
        assert!(!player.is_finished());
    }

    #[test]
    fn test_gain_node_empty_inputs() {
        let mut gain = GainNode::new(2.0);
        let mut out = AudioBuffer::from_interleaved(vec![1.0, 1.0], 2);

        // Process with no inputs — output should remain unchanged (no copy happens)
        gain.process(&[], &mut out);
        assert_eq!(out.get(0, 0), 1.0);
        assert_eq!(out.get(0, 1), 1.0);
    }

    #[test]
    fn test_gain_node_zero_gain() {
        let input = AudioBuffer::from_interleaved(vec![1.0, -1.0, 0.5, -0.5], 2);
        let mut gain = GainNode::new(0.0);
        let mut out = AudioBuffer::new(2, 2);

        gain.process(&[&input], &mut out);

        for &s in out.as_interleaved() {
            assert_eq!(s, 0.0);
        }
    }

    #[test]
    fn test_gain_node_unity_gain() {
        let input = AudioBuffer::from_interleaved(vec![0.3, -0.7, 0.5, -0.5], 2);
        let mut gain = GainNode::new(1.0);
        let mut out = AudioBuffer::new(2, 2);

        gain.process(&[&input], &mut out);

        assert_eq!(out.as_interleaved(), input.as_interleaved());
    }

    #[test]
    fn test_gain_node_negative_gain() {
        let input = AudioBuffer::from_interleaved(vec![1.0, -1.0], 2);
        let mut gain = GainNode::new(-1.0);
        let mut out = AudioBuffer::new(2, 1);

        gain.process(&[&input], &mut out);

        assert_eq!(out.get(0, 0), -1.0);
        assert_eq!(out.get(0, 1), 1.0);
    }

    #[test]
    fn test_gain_node_name_and_ports() {
        let gain = GainNode::new(1.0);
        assert_eq!(gain.name(), "gain");
        assert_eq!(gain.num_inputs(), 1);
        assert_eq!(gain.num_outputs(), 1);
        // GainNode uses default is_finished which returns false
        assert!(!gain.is_finished());
    }

    #[test]
    fn test_file_player_name_and_ports() {
        let src = AudioBuffer::new(1, 0);
        let player = FilePlayerNode::new(src, false);
        assert_eq!(player.name(), "file_player");
        assert_eq!(player.num_inputs(), 0);
        assert_eq!(player.num_outputs(), 1);
    }

    #[test]
    fn test_node_id_uniqueness() {
        let id1 = NodeId::next();
        let id2 = NodeId::next();
        let id3 = NodeId::next();
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }
}
