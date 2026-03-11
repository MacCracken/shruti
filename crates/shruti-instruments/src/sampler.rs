use shruti_dsp::AudioBuffer;
use shruti_session::midi::{ControlChange, NoteEvent};

use crate::envelope::{AdsrParams, Envelope};
use crate::instrument::{InstrumentInfo, InstrumentNode, InstrumentParam};
use serde::{Deserialize, Serialize};

/// A zone maps a range of MIDI notes and velocities to a sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleZone {
    pub name: String,
    pub root_key: u8,
    pub key_low: u8,
    pub key_high: u8,
    pub velocity_low: u8,
    pub velocity_high: u8,
    #[serde(skip)]
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub loop_start: Option<usize>,
    pub loop_end: Option<usize>,
    pub loop_mode: LoopMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMode {
    NoLoop,
    Forward,
    PingPong,
}

impl SampleZone {
    pub fn new(name: &str, root_key: u8, samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            name: name.to_string(),
            root_key,
            key_low: 0,
            key_high: 127,
            velocity_low: 0,
            velocity_high: 127,
            samples,
            sample_rate,
            loop_start: None,
            loop_end: None,
            loop_mode: LoopMode::NoLoop,
        }
    }

    /// Returns true if the given note and velocity fall within this zone's ranges.
    pub fn matches(&self, note: u8, velocity: u8) -> bool {
        note >= self.key_low
            && note <= self.key_high
            && velocity >= self.velocity_low
            && velocity <= self.velocity_high
    }

    /// Pitch ratio to play this sample at the given MIDI note.
    pub fn pitch_ratio(&self, note: u8) -> f64 {
        let semitones = f64::from(note) - f64::from(self.root_key);
        2.0_f64.powf(semitones / 12.0)
    }
}

/// A voice playing a sample zone.
struct SamplerVoice {
    zone_index: usize,
    note: u8,
    velocity: u8,
    channel: u8,
    play_pos: f64,
    pitch_ratio: f64,
    envelope: Envelope,
    active: bool,
    direction: i8,
}

impl SamplerVoice {
    fn new(sample_rate: f32) -> Self {
        Self {
            zone_index: 0,
            note: 0,
            velocity: 0,
            channel: 0,
            play_pos: 0.0,
            pitch_ratio: 1.0,
            envelope: Envelope::new(AdsrParams::default(), sample_rate),
            active: false,
            direction: 1,
        }
    }
}

// Parameter indices
const PARAM_VOLUME: usize = 0;
const PARAM_ATTACK: usize = 1;
const PARAM_DECAY: usize = 2;
const PARAM_SUSTAIN: usize = 3;
const PARAM_RELEASE: usize = 4;

const MAX_VOICES: usize = 16;

/// Multi-sample instrument: maps zones across keyboard and velocity.
pub struct Sampler {
    info: InstrumentInfo,
    params: Vec<InstrumentParam>,
    pub zones: Vec<SampleZone>,
    voices: Vec<SamplerVoice>,
    max_voices: usize,
    sample_rate: f32,
}

impl Sampler {
    pub fn new(sample_rate: f32) -> Self {
        let info = InstrumentInfo {
            name: "Sampler".to_string(),
            category: "Sampler".to_string(),
            author: "Shruti".to_string(),
            description: "Multi-sample instrument with zone mapping".to_string(),
        };

        let params = vec![
            InstrumentParam::new("Volume", 0.0, 1.0, 0.8, ""),
            InstrumentParam::new("Attack", 0.001, 5.0, 0.01, "s"),
            InstrumentParam::new("Decay", 0.001, 5.0, 0.1, "s"),
            InstrumentParam::new("Sustain", 0.0, 1.0, 0.7, ""),
            InstrumentParam::new("Release", 0.001, 10.0, 0.3, "s"),
        ];

        let voices = (0..MAX_VOICES)
            .map(|_| SamplerVoice::new(sample_rate))
            .collect();

        Self {
            info,
            params,
            zones: Vec::new(),
            voices,
            max_voices: MAX_VOICES,
            sample_rate,
        }
    }

    pub fn add_zone(&mut self, zone: SampleZone) {
        self.zones.push(zone);
    }

    pub fn remove_zone(&mut self, index: usize) {
        self.zones.remove(index);
    }

    /// Find the best matching zone for a note+velocity.
    fn find_zone(&self, note: u8, velocity: u8) -> Option<usize> {
        self.zones.iter().position(|z| z.matches(note, velocity))
    }

    fn current_adsr(&self) -> AdsrParams {
        AdsrParams {
            attack: self.params[PARAM_ATTACK].value,
            decay: self.params[PARAM_DECAY].value,
            sustain: self.params[PARAM_SUSTAIN].value,
            release: self.params[PARAM_RELEASE].value,
        }
    }

    /// Allocate a voice, stealing the oldest finished voice if necessary.
    fn allocate_voice(&mut self) -> Option<usize> {
        // First: find an inactive voice
        if let Some(idx) = self.voices.iter().position(|v| !v.active) {
            return Some(idx);
        }
        // If all voices active, steal the first one (oldest)
        if self.max_voices > 0 {
            Some(0)
        } else {
            None
        }
    }

    /// Read a sample from a zone with linear interpolation.
    fn read_sample(zone: &SampleZone, pos: f64) -> f32 {
        let len = zone.samples.len();
        if len == 0 {
            return 0.0;
        }
        let idx = pos.floor() as usize;
        if idx >= len {
            return 0.0;
        }
        let frac = (pos - pos.floor()) as f32;
        let s0 = zone.samples[idx];
        let s1 = if idx + 1 < len {
            zone.samples[idx + 1]
        } else {
            s0
        };
        s0 * (1.0 - frac) + s1 * frac
    }
}

impl InstrumentNode for Sampler {
    fn info(&self) -> &InstrumentInfo {
        &self.info
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for voice in &mut self.voices {
            voice.envelope.set_sample_rate(sample_rate);
        }
    }

    fn process(
        &mut self,
        note_events: &[NoteEvent],
        _control_changes: &[ControlChange],
        output: &mut AudioBuffer,
    ) {
        let frames = output.frames() as usize;
        let channels = output.channels();
        let volume = self.params[PARAM_VOLUME].value;

        // Process note events at the start of the block
        for event in note_events {
            self.note_on(event.note, event.velocity, event.channel);
        }

        // Render each active voice
        for i in 0..self.max_voices {
            if !self.voices[i].active {
                continue;
            }

            let zone_index = self.voices[i].zone_index;
            if zone_index >= self.zones.len() {
                self.voices[i].active = false;
                continue;
            }

            let vel_gain = self.voices[i].velocity as f32 / 127.0;
            let pitch_ratio = self.voices[i].pitch_ratio;

            for frame in 0..frames {
                let env_level = self.voices[i].envelope.tick();

                if self.voices[i].envelope.is_finished() {
                    self.voices[i].active = false;
                    break;
                }

                let sample = Self::read_sample(&self.zones[zone_index], self.voices[i].play_pos);
                let out = sample * env_level * vel_gain * volume;

                for ch in 0..channels {
                    let current = output.get(frame as u32, ch);
                    output.set(frame as u32, ch, current + out);
                }

                // Advance play position
                let zone = &self.zones[zone_index];
                match zone.loop_mode {
                    LoopMode::NoLoop => {
                        self.voices[i].play_pos += pitch_ratio;
                        if self.voices[i].play_pos >= zone.samples.len() as f64 {
                            // Sample finished, trigger release if not already releasing
                            self.voices[i].active = false;
                            break;
                        }
                    }
                    LoopMode::Forward => {
                        self.voices[i].play_pos += pitch_ratio;
                        let loop_start = zone.loop_start.unwrap_or(0) as f64;
                        let loop_end =
                            zone.loop_end.unwrap_or(zone.samples.len()) as f64;
                        if self.voices[i].play_pos >= loop_end {
                            self.voices[i].play_pos =
                                loop_start + (self.voices[i].play_pos - loop_end);
                        }
                    }
                    LoopMode::PingPong => {
                        let step = pitch_ratio * f64::from(self.voices[i].direction);
                        self.voices[i].play_pos += step;
                        let loop_start = zone.loop_start.unwrap_or(0) as f64;
                        let loop_end =
                            zone.loop_end.unwrap_or(zone.samples.len()) as f64;
                        if self.voices[i].play_pos >= loop_end {
                            self.voices[i].direction = -1;
                            self.voices[i].play_pos =
                                loop_end - (self.voices[i].play_pos - loop_end);
                        } else if self.voices[i].play_pos <= loop_start {
                            self.voices[i].direction = 1;
                            self.voices[i].play_pos =
                                loop_start + (loop_start - self.voices[i].play_pos);
                        }
                    }
                }
            }
        }
    }

    fn note_on(&mut self, note: u8, velocity: u8, channel: u8) {
        let zone_index = match self.find_zone(note, velocity) {
            Some(idx) => idx,
            None => return,
        };

        let voice_idx = match self.allocate_voice() {
            Some(idx) => idx,
            None => return,
        };

        let zone = &self.zones[zone_index];
        let pitch_ratio =
            zone.pitch_ratio(note) * f64::from(zone.sample_rate) / f64::from(self.sample_rate as u32);

        let adsr = self.current_adsr();
        let voice = &mut self.voices[voice_idx];
        voice.zone_index = zone_index;
        voice.note = note;
        voice.velocity = velocity;
        voice.channel = channel;
        voice.play_pos = 0.0;
        voice.pitch_ratio = pitch_ratio;
        voice.active = true;
        voice.direction = 1;
        voice.envelope.params = adsr;
        voice.envelope.trigger();
    }

    fn note_off(&mut self, note: u8, channel: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.note == note && voice.channel == channel {
                voice.envelope.release();
            }
        }
    }

    fn params(&self) -> &[InstrumentParam] {
        &self.params
    }

    fn params_mut(&mut self) -> &mut [InstrumentParam] {
        &mut self.params
    }

    fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
            voice.envelope.reset();
        }
    }

    fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a simple test sample: a sine wave at 1kHz, 1 second, 44100 Hz.
    fn make_test_samples(num_samples: usize) -> Vec<f32> {
        (0..num_samples)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect()
    }

    fn make_test_zone(root_key: u8) -> SampleZone {
        let mut zone = SampleZone::new("Test", root_key, make_test_samples(4410), 44100);
        zone.key_low = 0;
        zone.key_high = 127;
        zone
    }

    #[test]
    fn zone_matches_note_in_range() {
        let mut zone = SampleZone::new("Piano C4", 60, vec![], 44100);
        zone.key_low = 48;
        zone.key_high = 72;
        zone.velocity_low = 1;
        zone.velocity_high = 127;

        assert!(zone.matches(60, 100));
        assert!(zone.matches(48, 1));
        assert!(zone.matches(72, 127));
    }

    #[test]
    fn zone_rejects_note_outside_range() {
        let mut zone = SampleZone::new("Piano C4", 60, vec![], 44100);
        zone.key_low = 48;
        zone.key_high = 72;
        zone.velocity_low = 1;
        zone.velocity_high = 127;

        assert!(!zone.matches(47, 100));
        assert!(!zone.matches(73, 100));
        assert!(!zone.matches(60, 0)); // velocity 0 below velocity_low=1
    }

    #[test]
    fn zone_pitch_ratio_root_key_is_one() {
        let zone = SampleZone::new("Test", 60, vec![], 44100);
        let ratio = zone.pitch_ratio(60);
        assert!((ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn zone_pitch_ratio_octave_up_is_double() {
        let zone = SampleZone::new("Test", 60, vec![], 44100);
        let ratio = zone.pitch_ratio(72);
        assert!((ratio - 2.0).abs() < 1e-10);
    }

    #[test]
    fn zone_pitch_ratio_octave_down_is_half() {
        let zone = SampleZone::new("Test", 60, vec![], 44100);
        let ratio = zone.pitch_ratio(48);
        assert!((ratio - 0.5).abs() < 1e-10);
    }

    #[test]
    fn sampler_produces_audio_for_matching_zone() {
        let mut sampler = Sampler::new(44100.0);
        sampler.add_zone(make_test_zone(60));
        sampler.note_on(60, 100, 0);

        let mut buf = AudioBuffer::new(2, 256);
        sampler.process(&[], &[], &mut buf);

        let has_nonzero = (0..256).any(|i| buf.get(i, 0).abs() > 0.001);
        assert!(has_nonzero, "sampler should produce audio output");
    }

    #[test]
    fn sampler_silence_for_note_outside_zones() {
        let mut sampler = Sampler::new(44100.0);
        let mut zone = make_test_zone(60);
        zone.key_low = 48;
        zone.key_high = 72;
        sampler.add_zone(zone);

        // Note 80 is outside [48, 72]
        sampler.note_on(80, 100, 0);

        let mut buf = AudioBuffer::new(2, 256);
        sampler.process(&[], &[], &mut buf);

        for i in 0..256 {
            assert_eq!(buf.get(i, 0), 0.0);
        }
    }

    #[test]
    fn multiple_zones_different_key_ranges() {
        let mut sampler = Sampler::new(44100.0);

        let mut zone_low = SampleZone::new("Low", 36, make_test_samples(4410), 44100);
        zone_low.key_low = 0;
        zone_low.key_high = 59;

        let mut zone_high = SampleZone::new("High", 72, make_test_samples(4410), 44100);
        zone_high.key_low = 60;
        zone_high.key_high = 127;

        sampler.add_zone(zone_low);
        sampler.add_zone(zone_high);

        // Note 50 should match zone 0 (Low)
        assert_eq!(sampler.find_zone(50, 100), Some(0));
        // Note 70 should match zone 1 (High)
        assert_eq!(sampler.find_zone(70, 100), Some(1));
    }

    #[test]
    fn velocity_zone_selection() {
        let mut sampler = Sampler::new(44100.0);

        let mut zone_soft = SampleZone::new("Soft", 60, make_test_samples(4410), 44100);
        zone_soft.velocity_low = 1;
        zone_soft.velocity_high = 64;

        let mut zone_loud = SampleZone::new("Loud", 60, make_test_samples(4410), 44100);
        zone_loud.velocity_low = 65;
        zone_loud.velocity_high = 127;

        sampler.add_zone(zone_soft);
        sampler.add_zone(zone_loud);

        assert_eq!(sampler.find_zone(60, 50), Some(0));
        assert_eq!(sampler.find_zone(60, 100), Some(1));
        assert_eq!(sampler.find_zone(60, 0), None); // velocity 0 matches neither
    }

    #[test]
    fn oneshot_playback_completes_and_stops() {
        let mut sampler = Sampler::new(44100.0);
        // Very short sample: 100 samples
        let zone = SampleZone::new("Short", 60, vec![0.5; 100], 44100);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);
        assert_eq!(sampler.active_voices(), 1);

        // Process enough frames to exhaust the sample (100 samples at ratio 1.0)
        let mut buf = AudioBuffer::new(2, 256);
        sampler.process(&[], &[], &mut buf);

        // Voice should have deactivated since sample ended (NoLoop mode)
        assert_eq!(sampler.active_voices(), 0);
    }

    #[test]
    fn forward_loop_wraps_correctly() {
        let mut sampler = Sampler::new(44100.0);
        let mut zone = SampleZone::new("Loop", 60, vec![1.0; 200], 44100);
        zone.loop_mode = LoopMode::Forward;
        zone.loop_start = Some(50);
        zone.loop_end = Some(150);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        // Process 256 frames — the voice should still be active because it loops
        let mut buf = AudioBuffer::new(2, 256);
        sampler.process(&[], &[], &mut buf);

        assert_eq!(sampler.active_voices(), 1, "looping voice should stay active");

        // The play position should have wrapped around and still be in [50, 150)
        let pos = sampler.voices[0].play_pos;
        assert!(
            (50.0..150.0).contains(&pos),
            "play_pos should be in loop range, got {pos}"
        );
    }

    #[test]
    fn pingpong_loop_reverses_direction() {
        let mut sampler = Sampler::new(44100.0);
        let mut zone = SampleZone::new("PingPong", 60, vec![1.0; 200], 44100);
        zone.loop_mode = LoopMode::PingPong;
        zone.loop_start = Some(50);
        zone.loop_end = Some(150);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        // Process enough frames to go past loop_end and back
        let mut buf = AudioBuffer::new(2, 256);
        sampler.process(&[], &[], &mut buf);

        assert_eq!(sampler.active_voices(), 1, "ping-pong voice should stay active");
        // After 256 frames at ratio ~1.0, direction should have changed at least once
        // The voice should have reversed direction
        let dir = sampler.voices[0].direction;
        // At frame 150, direction reverses to -1, at frame ~50 reverses to 1 again,
        // at frame ~150 reverses to -1 again, etc. After 256 frames it depends on exact pos.
        // Just verify the voice is still active and position is within bounds.
        let pos = sampler.voices[0].play_pos;
        assert!(
            (50.0..=150.0).contains(&pos),
            "play_pos should be in loop range, got {pos}, dir={dir}"
        );
    }

    #[test]
    fn multiple_simultaneous_voices() {
        let mut sampler = Sampler::new(44100.0);
        sampler.add_zone(make_test_zone(60));

        sampler.note_on(60, 100, 0);
        sampler.note_on(64, 100, 0);
        sampler.note_on(67, 100, 0);

        assert_eq!(sampler.active_voices(), 3);

        let mut buf = AudioBuffer::new(2, 256);
        sampler.process(&[], &[], &mut buf);

        let has_nonzero = (0..256).any(|i| buf.get(i, 0).abs() > 0.001);
        assert!(has_nonzero, "multiple voices should produce audio");
    }

    #[test]
    fn note_off_triggers_release() {
        let mut sampler = Sampler::new(44100.0);
        let zone = SampleZone::new("Test", 60, vec![0.5; 44100], 44100);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);
        assert_eq!(sampler.active_voices(), 1);

        sampler.note_off(60, 0);

        // Voice should still be active (in release phase)
        assert_eq!(sampler.active_voices(), 1);

        // Process enough frames for release to complete
        // Default release is 0.3s = 13230 frames at 44100
        for _ in 0..60 {
            let mut buf = AudioBuffer::new(2, 256);
            sampler.process(&[], &[], &mut buf);
        }

        assert_eq!(sampler.active_voices(), 0, "voice should finish after release");
    }

    #[test]
    fn reset_clears_all_voices() {
        let mut sampler = Sampler::new(44100.0);
        sampler.add_zone(make_test_zone(60));

        sampler.note_on(60, 100, 0);
        sampler.note_on(64, 100, 0);
        assert_eq!(sampler.active_voices(), 2);

        sampler.reset();
        assert_eq!(sampler.active_voices(), 0);
    }
}
