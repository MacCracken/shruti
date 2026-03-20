//! AI Player — an `InstrumentNode` that generates MIDI from model inference.
//!
//! The AI Player listens to session context (key, tempo, chord progression)
//! and generates MIDI note events in real-time using a music LLM. It implements
//! `InstrumentNode` so it integrates seamlessly with the existing track/mixing
//! pipeline.
//!
//! Generated MIDI is routed to a configurable instrument (synth, sampler, etc.)
//! for audio rendering, or output as raw MIDI events.

use serde::{Deserialize, Serialize};
use shruti_dsp::AudioBuffer;
use shruti_session::midi::{ControlChange, NoteEvent};
use shruti_session::types::FramePos;

use crate::model::{GenerationConfig, InferenceScheduler, StubRuntime};
use crate::tokenizer::MidiTokenizer;

use shruti_instruments::instrument::{InstrumentInfo, InstrumentNode, InstrumentParam, ParamIndex};

/// AI Player playback mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackMode {
    /// Generate freely within constraints.
    Improvisation,
    /// Follow and complement a lead track.
    Accompaniment,
    /// Listen and respond to phrases.
    CallAndResponse,
}

/// Configuration for an AI Player instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPlayerConfig {
    /// Playback mode.
    pub mode: PlaybackMode,
    /// Musical key (0=C, 1=C#, ..., 11=B).
    pub key: u8,
    /// Whether the scale is minor (true) or major (false).
    pub minor: bool,
    /// Creativity level (0.0 = conservative, 1.0 = experimental).
    pub creativity: f32,
    /// Tempo in BPM (used for tokenizer timing).
    pub bpm: f64,
}

impl Default for AiPlayerConfig {
    fn default() -> Self {
        Self {
            mode: PlaybackMode::Improvisation,
            key: 0,
            minor: false,
            creativity: 0.5,
            bpm: 120.0,
        }
    }
}

/// Type-safe parameter indices for [`AiPlayer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum AiPlayerParam {
    Volume = 0,
    Creativity = 1,
    Key = 2,
    Minor = 3,
    Mode = 4,
}

impl ParamIndex for AiPlayerParam {
    fn index(self) -> usize {
        self as usize
    }
    fn count() -> usize {
        AiPlayerParam::Mode as usize + 1
    }
}

impl From<AiPlayerParam> for usize {
    fn from(p: AiPlayerParam) -> usize {
        p as usize
    }
}

impl TryFrom<usize> for AiPlayerParam {
    type Error = ();
    fn try_from(v: usize) -> Result<Self, ()> {
        match v {
            0 => Ok(Self::Volume),
            1 => Ok(Self::Creativity),
            2 => Ok(Self::Key),
            3 => Ok(Self::Minor),
            4 => Ok(Self::Mode),
            _ => Err(()),
        }
    }
}

/// AI-powered virtual instrument that generates MIDI via music LLM inference.
///
/// Implements `InstrumentNode` for seamless integration with the DAW's
/// track and mixing pipeline. Uses an `InferenceScheduler` to generate
/// tokens ahead of playback, then decodes them to MIDI events.
pub struct AiPlayer {
    info: InstrumentInfo,
    params: Vec<InstrumentParam>,
    scheduler: InferenceScheduler,
    tokenizer: MidiTokenizer,
    sample_rate: f32,
    /// Pending generated events waiting to be rendered.
    pending_events: Vec<NoteEvent>,
    /// Current playback position within generated events.
    event_cursor: usize,
    /// Frame counter within the current generation window.
    frame_counter: u64,
    /// Whether the player is actively generating.
    generating: bool,
}

impl AiPlayer {
    /// Create a new AI Player with the stub runtime (for testing).
    pub fn new(sample_rate: f32) -> Self {
        Self::with_config(sample_rate, AiPlayerConfig::default())
    }

    /// Create a new AI Player with the given config.
    pub fn with_config(sample_rate: f32, config: AiPlayerConfig) -> Self {
        let info = InstrumentInfo {
            name: "AI Player".to_string(),
            category: "AI".to_string(),
            author: "Shruti".to_string(),
            description: "AI-powered instrument using music LLM inference".to_string(),
        };

        let params = vec![
            InstrumentParam::new("Volume", 0.0, 1.0, 0.8, ""),
            InstrumentParam::new("Creativity", 0.0, 1.0, config.creativity, ""),
            InstrumentParam::new("Key", 0.0, 11.0, config.key as f32, ""),
            InstrumentParam::new("Minor", 0.0, 1.0, if config.minor { 1.0 } else { 0.0 }, ""),
            InstrumentParam::new("Mode", 0.0, 2.0, config.mode as u8 as f32, ""),
        ];

        let gen_config = GenerationConfig {
            temperature: 0.5 + config.creativity * 0.8,
            ..Default::default()
        };

        let runtime = Box::new(StubRuntime::new());
        let scheduler = InferenceScheduler::new(runtime, gen_config, 64);
        let tokenizer = MidiTokenizer::new(sample_rate as u32, config.bpm, 4);

        Self {
            info,
            params,
            scheduler,
            tokenizer,
            sample_rate,
            pending_events: Vec::new(),
            event_cursor: 0,
            frame_counter: 0,
            generating: false,
        }
    }

    /// Start generating music. Call this when playback begins.
    pub fn start_generation(&mut self) {
        self.generating = true;
        self.frame_counter = 0;
        self.event_cursor = 0;
        self.scheduler.reset();
        self.generate_events();
    }

    /// Stop generating.
    pub fn stop_generation(&mut self) {
        self.generating = false;
        self.pending_events.clear();
        self.event_cursor = 0;
    }

    /// Generate a batch of events from the model.
    fn generate_events(&mut self) {
        self.scheduler.fill_buffer();

        // Collect tokens until we have enough for a phrase
        let mut tokens = Vec::new();
        while let Some(token) = self.scheduler.next_token() {
            tokens.push(token);
            // Stop after a reasonable phrase length
            if tokens.len() >= 32 {
                break;
            }
        }

        if !tokens.is_empty() {
            let mut events = self.tokenizer.decode(&tokens);
            // Offset events relative to current frame counter
            for event in &mut events {
                event.position = FramePos(event.position.0 + self.frame_counter);
            }
            self.pending_events.extend(events);
        }
    }

    /// Get events that should play during the current frame range.
    fn events_in_range(&self, start: u64, end: u64) -> Vec<NoteEvent> {
        self.pending_events
            .iter()
            .filter(|e| {
                let pos = e.position.0;
                pos >= start && pos < end
            })
            .cloned()
            .collect()
    }
}

impl InstrumentNode for AiPlayer {
    fn info(&self) -> &InstrumentInfo {
        &self.info
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.tokenizer = MidiTokenizer::new(sample_rate as u32, 120.0, 4);
    }

    fn process(
        &mut self,
        _note_events: &[NoteEvent],
        _control_changes: &[ControlChange],
        output: &mut AudioBuffer,
    ) {
        if !self.generating {
            return;
        }

        let frames = output.frames() as u64;
        let start = self.frame_counter;
        let end = start + frames;

        // Check if we need more events
        let max_pending = self
            .pending_events
            .last()
            .map(|e| e.position.0)
            .unwrap_or(0);
        if max_pending < end + self.sample_rate as u64 {
            self.generate_events();
        }

        // Collect events in this frame range
        let events = self.events_in_range(start, end);

        // Render events as simple sine tones (placeholder audio rendering)
        // In production, these events would be routed to a real instrument
        let volume = self.params[AiPlayerParam::Volume.index()].value;
        for event in &events {
            let event_start = event.position.0.saturating_sub(start) as usize;
            let event_duration = event.duration.0.min(frames - event_start as u64) as usize;
            let freq = 440.0 * 2.0f32.powf((event.note as f32 - 69.0) / 12.0);
            let vel_gain = event.velocity as f32 / 127.0;

            for frame in event_start..event_start + event_duration {
                if frame >= frames as usize {
                    break;
                }
                let t = frame as f32 / self.sample_rate;
                let sample = (t * freq * std::f32::consts::TAU).sin() * vel_gain * volume * 0.3;
                for ch in 0..output.channels() {
                    let current = output.get(frame as u32, ch);
                    output.set(frame as u32, ch, current + sample);
                }
            }
        }

        self.frame_counter = end;
    }

    fn note_on(&mut self, _note: u8, _velocity: u8, _channel: u8) {
        // AI Player generates its own notes — external note-on starts generation
        if !self.generating {
            self.start_generation();
        }
    }

    fn note_off(&mut self, _note: u8, _channel: u8) {
        // Ignore — AI Player manages its own note lifecycle
    }

    fn params(&self) -> &[InstrumentParam] {
        &self.params
    }

    fn params_mut(&mut self) -> &mut [InstrumentParam] {
        &mut self.params
    }

    fn reset(&mut self) {
        self.stop_generation();
        self.scheduler.reset();
    }

    fn active_voices(&self) -> usize {
        if self.generating {
            self.events_in_range(
                self.frame_counter,
                self.frame_counter + self.sample_rate as u64 / 100,
            )
            .len()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_player_creates() {
        let player = AiPlayer::new(44100.0);
        assert_eq!(player.info().name, "AI Player");
        assert_eq!(player.params().len(), 5);
    }

    #[test]
    fn ai_player_param_count() {
        assert_eq!(AiPlayerParam::count(), 5);
    }

    #[test]
    fn ai_player_generates_audio() {
        let mut player = AiPlayer::new(44100.0);
        player.start_generation();

        let mut buf = AudioBuffer::new(2, 4096);
        player.process(&[], &[], &mut buf);

        // Should produce some non-zero audio
        assert!(
            buf.as_interleaved().iter().any(|&s| s.abs() > 0.0001),
            "AI player should produce audio after starting generation"
        );
    }

    #[test]
    fn ai_player_silent_when_not_generating() {
        let mut player = AiPlayer::new(44100.0);
        let mut buf = AudioBuffer::new(2, 256);
        player.process(&[], &[], &mut buf);

        assert!(
            buf.as_interleaved().iter().all(|&s| s.abs() < 0.0001),
            "AI player should be silent when not generating"
        );
    }

    #[test]
    fn ai_player_note_on_starts_generation() {
        let mut player = AiPlayer::new(44100.0);
        assert!(!player.generating);
        player.note_on(60, 100, 0);
        assert!(player.generating);
    }

    #[test]
    fn ai_player_reset_stops() {
        let mut player = AiPlayer::new(44100.0);
        player.start_generation();
        assert!(player.generating);
        player.reset();
        assert!(!player.generating);
    }

    #[test]
    fn ai_player_with_config() {
        let config = AiPlayerConfig {
            mode: PlaybackMode::Accompaniment,
            key: 5,
            minor: true,
            creativity: 0.9,
            bpm: 140.0,
        };
        let player = AiPlayer::with_config(44100.0, config);
        assert!((player.params[AiPlayerParam::Creativity.index()].value - 0.9).abs() < 0.01);
        assert!((player.params[AiPlayerParam::Key.index()].value - 5.0).abs() < 0.01);
    }

    #[test]
    fn playback_mode_serializes() {
        let mode = PlaybackMode::CallAndResponse;
        let json = serde_json::to_string(&mode).unwrap();
        let rt: PlaybackMode = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, mode);
    }

    #[test]
    fn ai_player_multiple_blocks() {
        let mut player = AiPlayer::new(44100.0);
        player.start_generation();

        // Process multiple blocks
        for _ in 0..10 {
            let mut buf = AudioBuffer::new(2, 1024);
            player.process(&[], &[], &mut buf);
        }

        // Should still be generating without issues
        assert!(player.generating);
    }

    #[test]
    fn ai_player_config_default() {
        let config = AiPlayerConfig::default();
        assert_eq!(config.mode, PlaybackMode::Improvisation);
        assert_eq!(config.key, 0);
        assert!(!config.minor);
        assert!((config.creativity - 0.5).abs() < 0.01);
    }
}
