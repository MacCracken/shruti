//! Fine-grained music tokenizer for MIDI ↔ token conversion.
//!
//! Encodes MIDI events into a token sequence suitable for transformer-based
//! music LLMs, and decodes generated tokens back to MIDI events.
//!
//! Token vocabulary:
//! - Note tokens: 128 pitches (0–127)
//! - Velocity tokens: 32 bins (quantized from 0–127)
//! - Duration tokens: 64 bins (quantized time steps)
//! - Time-shift tokens: 100 bins (10ms resolution, up to 1s)
//! - Special tokens: BOS, EOS, PAD, BAR, INSTRUMENT

use serde::{Deserialize, Serialize};
use shruti_session::midi::NoteEvent;
use shruti_session::types::FramePos;

/// Number of velocity quantization bins.
const VELOCITY_BINS: u8 = 32;

/// Number of duration quantization bins.
const DURATION_BINS: u8 = 64;

/// Number of time-shift bins (10ms each, up to 1s).
const TIME_SHIFT_BINS: u8 = 100;

/// A single token in the music vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MidiToken {
    /// Note-on for a MIDI pitch (0–127).
    NoteOn(u8),
    /// Note-off for a MIDI pitch (0–127).
    NoteOff(u8),
    /// Velocity bin (0–31, quantized from 0–127).
    Velocity(u8),
    /// Duration bin (0–63, quantized time steps).
    Duration(u8),
    /// Time shift in 10ms increments (0–99).
    TimeShift(u8),
    /// Bar line marker.
    Bar,
    /// Instrument identifier (General MIDI program number).
    Instrument(u8),
    /// Beginning of sequence.
    Bos,
    /// End of sequence.
    Eos,
    /// Padding token.
    Pad,
}

impl MidiToken {
    /// Convert a token to its integer ID for model input.
    pub fn to_id(self) -> u32 {
        match self {
            MidiToken::NoteOn(n) => n as u32,          // 0–127
            MidiToken::NoteOff(n) => 128 + n as u32,   // 128–255
            MidiToken::Velocity(v) => 256 + v as u32,  // 256–287
            MidiToken::Duration(d) => 288 + d as u32,  // 288–351
            MidiToken::TimeShift(t) => 352 + t as u32, // 352–451
            MidiToken::Bar => 452,
            MidiToken::Instrument(i) => 453 + i as u32, // 453–580
            MidiToken::Bos => 581,
            MidiToken::Eos => 582,
            MidiToken::Pad => 583,
        }
    }

    /// Convert an integer ID back to a token.
    pub fn from_id(id: u32) -> Option<Self> {
        match id {
            0..=127 => Some(MidiToken::NoteOn(id as u8)),
            128..=255 => Some(MidiToken::NoteOff((id - 128) as u8)),
            256..=287 => Some(MidiToken::Velocity((id - 256) as u8)),
            288..=351 => Some(MidiToken::Duration((id - 288) as u8)),
            352..=451 => Some(MidiToken::TimeShift((id - 352) as u8)),
            452 => Some(MidiToken::Bar),
            453..=580 => Some(MidiToken::Instrument((id - 453) as u8)),
            581 => Some(MidiToken::Bos),
            582 => Some(MidiToken::Eos),
            583 => Some(MidiToken::Pad),
            _ => None,
        }
    }

    /// Total vocabulary size.
    pub const VOCAB_SIZE: u32 = 584;
}

/// Tokenizer for converting between MIDI events and token sequences.
#[derive(Debug, Clone)]
pub struct MidiTokenizer {
    /// Sample rate for frame-to-time conversion.
    sample_rate: u32,
    /// Tempo in BPM for bar line detection.
    bpm: f64,
    /// Time signature numerator.
    time_sig_num: u8,
}

impl MidiTokenizer {
    pub fn new(sample_rate: u32, bpm: f64, time_sig_num: u8) -> Self {
        Self {
            sample_rate,
            bpm,
            time_sig_num,
        }
    }

    /// Quantize a velocity (0–127) to a velocity bin (0–31).
    fn quantize_velocity(velocity: u8) -> u8 {
        (velocity as u16 * VELOCITY_BINS as u16 / 128) as u8
    }

    /// Dequantize a velocity bin (0–31) back to velocity (0–127).
    fn dequantize_velocity(bin: u8) -> u8 {
        let bin = bin.min(VELOCITY_BINS - 1);
        (bin as u16 * 127 / (VELOCITY_BINS as u16 - 1)) as u8
    }

    /// Quantize a duration in frames to a duration bin (0–63).
    fn quantize_duration(&self, frames: u64) -> u8 {
        // Each bin = 50ms worth of frames
        let ms = frames as f64 / self.sample_rate as f64 * 1000.0;
        let bin = (ms / 50.0).round() as u8;
        bin.min(DURATION_BINS - 1)
    }

    /// Dequantize a duration bin back to frames.
    fn dequantize_duration(&self, bin: u8) -> u64 {
        let ms = bin as f64 * 50.0;
        (ms / 1000.0 * self.sample_rate as f64).round() as u64
    }

    /// Quantize a time gap in frames to time-shift bins (10ms each).
    fn quantize_time_shift(&self, frames: u64) -> Vec<u8> {
        let ms = frames as f64 / self.sample_rate as f64 * 1000.0;
        let total_bins = (ms / 10.0).round() as u64;

        if total_bins == 0 {
            return Vec::new();
        }

        // Emit multiple time-shift tokens if gap > 1 second
        let mut shifts = Vec::new();
        let mut remaining = total_bins;
        while remaining > 0 {
            let bin = remaining.min(TIME_SHIFT_BINS as u64 - 1);
            shifts.push(bin as u8);
            remaining = remaining.saturating_sub(TIME_SHIFT_BINS as u64);
        }
        shifts
    }

    /// Encode a sequence of MIDI note events into tokens.
    ///
    /// Events should be sorted by position. The output includes BOS/EOS
    /// markers, time shifts between events, and velocity/duration tokens
    /// before each note-on.
    ///
    /// Note: encoding involves quantization of velocity (32 bins), duration (64 bins),
    /// and timing (10ms resolution). Exact round-tripping is not guaranteed.
    pub fn encode(&self, events: &[NoteEvent]) -> Vec<MidiToken> {
        let mut tokens = vec![MidiToken::Bos];
        let mut last_pos: u64 = 0;

        for event in events {
            let pos = event.position.0;

            // Emit time shift tokens for the gap
            if pos > last_pos {
                let shifts = self.quantize_time_shift(pos - last_pos);
                for s in shifts {
                    tokens.push(MidiToken::TimeShift(s));
                }
            }

            // Velocity before note-on
            tokens.push(MidiToken::Velocity(Self::quantize_velocity(event.velocity)));

            // Duration before note-on
            tokens.push(MidiToken::Duration(
                self.quantize_duration(event.duration.0),
            ));

            // Note-on
            tokens.push(MidiToken::NoteOn(event.note));

            last_pos = pos;
        }

        tokens.push(MidiToken::Eos);
        tokens
    }

    /// Decode a token sequence back to MIDI note events.
    ///
    /// Reconstructs note events from the token stream, resolving
    /// velocity, duration, and time shifts.
    ///
    /// Note: decoded events reflect quantized values from the token stream;
    /// they will not exactly match original events due to binning.
    pub fn decode(&self, tokens: &[MidiToken]) -> Vec<NoteEvent> {
        let mut events = Vec::new();
        let mut current_pos: u64 = 0;
        let mut current_velocity: u8 = 100;
        let mut current_duration: u64 = self.dequantize_duration(4); // default ~200ms

        for token in tokens {
            match token {
                MidiToken::TimeShift(bins) => {
                    let frames =
                        (*bins as f64 * 10.0 / 1000.0 * self.sample_rate as f64).round() as u64;
                    current_pos += frames;
                }
                MidiToken::Velocity(bin) => {
                    current_velocity = Self::dequantize_velocity(*bin);
                }
                MidiToken::Duration(bin) => {
                    current_duration = self.dequantize_duration(*bin);
                }
                MidiToken::NoteOn(note) => {
                    events.push(NoteEvent {
                        position: FramePos(current_pos),
                        duration: FramePos(current_duration),
                        note: *note,
                        velocity: current_velocity,
                        channel: 0,
                    });
                }
                MidiToken::Bos | MidiToken::Eos | MidiToken::Pad => {}
                MidiToken::Bar => {}
                MidiToken::NoteOff(_) | MidiToken::Instrument(_) => {}
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_id_roundtrip() {
        let tokens = [
            MidiToken::NoteOn(60),
            MidiToken::NoteOff(60),
            MidiToken::Velocity(16),
            MidiToken::Duration(32),
            MidiToken::TimeShift(50),
            MidiToken::Bar,
            MidiToken::Instrument(0),
            MidiToken::Bos,
            MidiToken::Eos,
            MidiToken::Pad,
        ];
        for token in &tokens {
            let id = token.to_id();
            let back = MidiToken::from_id(id).unwrap();
            assert_eq!(*token, back, "roundtrip failed for {token:?} (id={id})");
        }
    }

    #[test]
    fn token_id_range() {
        // All note-on IDs should be 0–127
        assert_eq!(MidiToken::NoteOn(0).to_id(), 0);
        assert_eq!(MidiToken::NoteOn(127).to_id(), 127);
        // All note-off IDs should be 128–255
        assert_eq!(MidiToken::NoteOff(0).to_id(), 128);
        assert_eq!(MidiToken::NoteOff(127).to_id(), 255);
        // Vocab size
        assert_eq!(MidiToken::VOCAB_SIZE, 584);
    }

    #[test]
    fn invalid_token_id_returns_none() {
        assert!(MidiToken::from_id(584).is_none());
        assert!(MidiToken::from_id(10000).is_none());
    }

    #[test]
    fn velocity_quantize_roundtrip() {
        // Full velocity should map to highest bin and back
        let bin = MidiTokenizer::quantize_velocity(127);
        assert_eq!(bin, 31);
        let back = MidiTokenizer::dequantize_velocity(bin);
        assert_eq!(back, 127);
    }

    #[test]
    fn velocity_quantize_zero() {
        let bin = MidiTokenizer::quantize_velocity(0);
        assert_eq!(bin, 0);
        let back = MidiTokenizer::dequantize_velocity(bin);
        assert_eq!(back, 0);
    }

    #[test]
    fn encode_single_note() {
        let tokenizer = MidiTokenizer::new(44100, 120.0, 4);
        let events = vec![NoteEvent {
            position: FramePos(0),
            duration: FramePos(22050), // ~500ms
            note: 60,
            velocity: 100,
            channel: 0,
        }];

        let tokens = tokenizer.encode(&events);
        assert_eq!(tokens[0], MidiToken::Bos);
        assert!(matches!(tokens[1], MidiToken::Velocity(_)));
        assert!(matches!(tokens[2], MidiToken::Duration(_)));
        assert_eq!(tokens[3], MidiToken::NoteOn(60));
        assert_eq!(*tokens.last().unwrap(), MidiToken::Eos);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let tokenizer = MidiTokenizer::new(44100, 120.0, 4);
        let events = vec![
            NoteEvent {
                position: FramePos(0),
                duration: FramePos(22050),
                note: 60,
                velocity: 100,
                channel: 0,
            },
            NoteEvent {
                position: FramePos(22050),
                duration: FramePos(22050),
                note: 64,
                velocity: 80,
                channel: 0,
            },
        ];

        let tokens = tokenizer.encode(&events);
        let decoded = tokenizer.decode(&tokens);

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].note, 60);
        assert_eq!(decoded[1].note, 64);
        // Velocity should be approximately preserved (quantization loss)
        assert!((decoded[0].velocity as i16 - 100).abs() < 10);
        assert!((decoded[1].velocity as i16 - 80).abs() < 10);
    }

    #[test]
    fn encode_with_time_gap() {
        let tokenizer = MidiTokenizer::new(44100, 120.0, 4);
        let events = vec![
            NoteEvent {
                position: FramePos(0),
                duration: FramePos(4410),
                note: 60,
                velocity: 100,
                channel: 0,
            },
            NoteEvent {
                position: FramePos(44100), // 1 second gap
                duration: FramePos(4410),
                note: 64,
                velocity: 80,
                channel: 0,
            },
        ];

        let tokens = tokenizer.encode(&events);
        // Should have time-shift tokens between the two notes
        let time_shifts: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t, MidiToken::TimeShift(_)))
            .collect();
        assert!(!time_shifts.is_empty());
    }

    #[test]
    fn decode_empty_sequence() {
        let tokenizer = MidiTokenizer::new(44100, 120.0, 4);
        let tokens = vec![MidiToken::Bos, MidiToken::Eos];
        let events = tokenizer.decode(&tokens);
        assert!(events.is_empty());
    }

    #[test]
    fn token_serializes() {
        let token = MidiToken::NoteOn(60);
        let json = serde_json::to_string(&token).unwrap();
        let rt: MidiToken = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, token);
    }
}
