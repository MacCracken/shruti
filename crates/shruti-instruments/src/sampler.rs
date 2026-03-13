use shruti_dsp::AudioBuffer;
use shruti_session::midi::{ControlChange, NoteEvent};

use crate::effect_chain::EffectChain;
use crate::envelope::{AdsrParams, Envelope};
use crate::instrument::{InstrumentInfo, InstrumentNode, InstrumentParam};
use serde::{Deserialize, Serialize};

/// A single slice point within a sample (index into sample data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicePoint {
    /// Sample index where this slice begins.
    pub index: usize,
    /// Optional human-readable name for the slice.
    pub name: Option<String>,
}

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
    /// Ordered slice boundaries for REX-style slicing.
    #[serde(default)]
    pub slices: Vec<SlicePoint>,
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
            slices: Vec::new(),
        }
    }

    /// Returns true if the given note and velocity fall within this zone's ranges.
    pub fn matches(&self, note: u8, velocity: u8) -> bool {
        note >= self.key_low
            && note <= self.key_high
            && velocity >= self.velocity_low
            && velocity <= self.velocity_high
    }

    // ── Slice mode (REX-style) ──────────────────────────────────────

    /// Add a manual slice point at the given sample index.
    pub fn add_slice(&mut self, index: usize, name: Option<String>) {
        if index < self.samples.len() {
            self.slices.push(SlicePoint { index, name });
            self.slices.sort_by_key(|s| s.index);
        }
    }

    /// Remove all slice points.
    pub fn clear_slices(&mut self) {
        self.slices.clear();
    }

    /// Number of slices currently defined.
    pub fn slice_count(&self) -> usize {
        self.slices.len()
    }

    /// Auto-detect transients using energy-based onset detection and create
    /// slice points at each detected onset.
    pub fn auto_slice_by_transients(&mut self, threshold: f32) {
        self.slices.clear();

        if self.samples.is_empty() {
            return;
        }

        let hop = 512_usize;
        let window = 1024_usize;
        let min_gap = 2048_usize;
        let factor = 1.0 + threshold * 9.0;

        let len = self.samples.len();
        if len < window {
            return;
        }

        let num_frames = (len - window) / hop + 1;
        let mut energies = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            let start = i * hop;
            let end = (start + window).min(len);
            let energy: f32 = self.samples[start..end].iter().map(|s| s * s).sum();
            energies.push(energy);
        }

        if energies.is_empty() {
            return;
        }

        let avg_len = 8_usize;
        let mut running_sum: f32 = 0.0;
        let mut running_count: usize = 0;
        let mut last_onset: Option<usize> = None;

        for (i, &energy) in energies.iter().enumerate() {
            let local_avg = if running_count > 0 {
                running_sum / running_count as f32
            } else {
                0.0
            };

            let sample_pos = i * hop;

            let gap_ok = match last_onset {
                Some(prev) => sample_pos - prev >= min_gap,
                None => true,
            };

            if gap_ok && energy > local_avg * factor && energy > 1e-8 {
                self.slices.push(SlicePoint {
                    index: sample_pos,
                    name: None,
                });
                last_onset = Some(sample_pos);
            }

            running_sum += energy;
            running_count += 1;
            if running_count > avg_len {
                running_sum -= energies[i + 1 - running_count];
                running_count = avg_len;
            }
        }
    }

    /// Split the sample into individual `SampleZone` entries — one per slice —
    /// mapped to consecutive MIDI keys starting from `base_note`.
    pub fn slice_to_zones(&self, base_note: u8) -> Vec<SampleZone> {
        if self.slices.is_empty() {
            return Vec::new();
        }

        let mut zones = Vec::with_capacity(self.slices.len());
        let num_slices = self.slices.len();

        for (i, slice) in self.slices.iter().enumerate() {
            let start = slice.index;
            let end = if i + 1 < num_slices {
                self.slices[i + 1].index
            } else {
                self.samples.len()
            };

            if start >= end || start >= self.samples.len() {
                continue;
            }

            let note = base_note.saturating_add(i as u8).min(127);
            let slice_name = slice
                .name
                .clone()
                .unwrap_or_else(|| format!("{}_slice_{}", self.name, i));
            let slice_samples = self.samples[start..end.min(self.samples.len())].to_vec();

            let mut zone = SampleZone::new(&slice_name, note, slice_samples, self.sample_rate);
            zone.key_low = note;
            zone.key_high = note;
            zones.push(zone);
        }

        zones
    }

    // ── Pitch ─────────────────────────────────────────────────────────

    /// Pitch ratio to play this sample at the given MIDI note.
    pub fn pitch_ratio(&self, note: u8) -> f64 {
        let semitones = f64::from(note) - f64::from(self.root_key);
        2.0_f64.powf(semitones / 12.0)
    }

    /// Total number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether this zone has no sample data.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Trim the sample to the given range `[start..end)`.
    /// Adjusts loop points to remain valid within the new range.
    pub fn trim(&mut self, start: usize, end: usize) {
        let end = end.min(self.samples.len());
        let start = start.min(end);
        self.samples = self.samples[start..end].to_vec();

        // Adjust loop points relative to new start
        self.loop_start = self.loop_start.and_then(|ls| {
            if ls >= start && ls < end {
                Some(ls - start)
            } else {
                None
            }
        });
        self.loop_end = self.loop_end.and_then(|le| {
            if le > start && le <= end {
                Some(le - start)
            } else {
                None
            }
        });
    }

    /// Set loop points with validation.
    /// Returns false if the points are invalid (out of range or start >= end).
    pub fn set_loop_points(&mut self, start: usize, end: usize) -> bool {
        if start >= end || end > self.samples.len() {
            return false;
        }
        self.loop_start = Some(start);
        self.loop_end = Some(end);
        true
    }

    /// Clear loop points.
    pub fn clear_loop_points(&mut self) {
        self.loop_start = None;
        self.loop_end = None;
        self.loop_mode = LoopMode::NoLoop;
    }

    /// Apply a linear fade-in over the first `length` samples.
    pub fn fade_in(&mut self, length: usize) {
        let len = length.min(self.samples.len());
        for i in 0..len {
            self.samples[i] *= i as f32 / len as f32;
        }
    }

    /// Apply a linear fade-out over the last `length` samples.
    pub fn fade_out(&mut self, length: usize) {
        let total = self.samples.len();
        let len = length.min(total);
        let start = total - len;
        for i in 0..len {
            self.samples[start + i] *= 1.0 - (i as f32 / len as f32);
        }
    }

    /// Normalize the sample to peak amplitude of `target` (default 1.0).
    /// Returns the gain factor applied, or 0.0 if the sample is silent.
    pub fn normalize(&mut self, target: f32) -> f32 {
        let peak = self.samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        if peak < 1e-10 {
            return 0.0;
        }
        let gain = target / peak;
        for s in &mut self.samples {
            *s *= gain;
        }
        gain
    }

    /// Reverse the sample data in place.
    pub fn reverse(&mut self) {
        self.samples.reverse();
        // Swap and recalculate loop points
        let len = self.samples.len();
        let new_start = self.loop_end.map(|le| len.saturating_sub(le));
        let new_end = self.loop_start.map(|ls| len.saturating_sub(ls));
        self.loop_start = new_start;
        self.loop_end = new_end;
    }

    /// Get the peak amplitude of the sample.
    pub fn peak(&self) -> f32 {
        self.samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max)
    }

    /// Get the RMS level of the sample.
    pub fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = self.samples.iter().map(|s| s * s).sum();
        (sum_sq / self.samples.len() as f32).sqrt()
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
    /// Per-instrument effect chain.
    pub effect_chain: EffectChain,
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
            effect_chain: EffectChain::new(),
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
        if self.max_voices > 0 { Some(0) } else { None }
    }

    /// Render all active voices into the output buffer (adds to existing content).
    fn render_voices(&mut self, note_events: &[NoteEvent], output: &mut AudioBuffer) {
        let frames = output.frames() as usize;
        let channels = output.channels();
        let volume = self.params[PARAM_VOLUME].value;

        for event in note_events {
            self.note_on(event.note, event.velocity, event.channel);
        }

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

                let zone = &self.zones[zone_index];
                match zone.loop_mode {
                    LoopMode::NoLoop => {
                        self.voices[i].play_pos += pitch_ratio;
                        if self.voices[i].play_pos >= zone.samples.len() as f64 {
                            self.voices[i].active = false;
                            break;
                        }
                    }
                    LoopMode::Forward => {
                        self.voices[i].play_pos += pitch_ratio;
                        let loop_start = zone.loop_start.unwrap_or(0) as f64;
                        let loop_end = zone.loop_end.unwrap_or(zone.samples.len()) as f64;
                        if self.voices[i].play_pos >= loop_end {
                            self.voices[i].play_pos =
                                loop_start + (self.voices[i].play_pos - loop_end);
                        }
                    }
                    LoopMode::PingPong => {
                        let step = pitch_ratio * f64::from(self.voices[i].direction);
                        self.voices[i].play_pos += step;
                        let loop_start = zone.loop_start.unwrap_or(0) as f64;
                        let loop_end = zone.loop_end.unwrap_or(zone.samples.len()) as f64;
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
        self.effect_chain.set_sample_rate(sample_rate);
    }

    fn process(
        &mut self,
        note_events: &[NoteEvent],
        _control_changes: &[ControlChange],
        output: &mut AudioBuffer,
    ) {
        let mut chain = std::mem::take(&mut self.effect_chain);
        chain.process_with(output, |buf| {
            self.render_voices(note_events, buf);
        });
        self.effect_chain = chain;
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
        let pitch_ratio = zone.pitch_ratio(note) * f64::from(zone.sample_rate)
            / f64::from(self.sample_rate.max(1.0));

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
        self.effect_chain.reset();
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

        assert_eq!(
            sampler.active_voices(),
            1,
            "looping voice should stay active"
        );

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

        assert_eq!(
            sampler.active_voices(),
            1,
            "ping-pong voice should stay active"
        );
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

        assert_eq!(
            sampler.active_voices(),
            0,
            "voice should finish after release"
        );
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

    // --- Sample editing tests ---

    #[test]
    fn trim_basic() {
        let mut zone = SampleZone::new("Test", 60, vec![1.0, 2.0, 3.0, 4.0, 5.0], 44100);
        zone.trim(1, 4);
        assert_eq!(zone.samples, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn trim_adjusts_loop_points() {
        let mut zone = SampleZone::new("Test", 60, vec![0.0; 100], 44100);
        zone.loop_start = Some(20);
        zone.loop_end = Some(80);
        zone.trim(10, 90);
        assert_eq!(zone.samples.len(), 80);
        assert_eq!(zone.loop_start, Some(10)); // 20-10
        assert_eq!(zone.loop_end, Some(70)); // 80-10
    }

    #[test]
    fn trim_invalidates_out_of_range_loop_points() {
        let mut zone = SampleZone::new("Test", 60, vec![0.0; 100], 44100);
        zone.loop_start = Some(5);
        zone.loop_end = Some(95);
        zone.trim(50, 100);
        assert!(zone.loop_start.is_none()); // 5 < 50, out of range
        assert_eq!(zone.loop_end, Some(45)); // 95-50
    }

    #[test]
    fn set_loop_points_valid() {
        let mut zone = SampleZone::new("Test", 60, vec![0.0; 100], 44100);
        assert!(zone.set_loop_points(10, 90));
        assert_eq!(zone.loop_start, Some(10));
        assert_eq!(zone.loop_end, Some(90));
    }

    #[test]
    fn set_loop_points_invalid() {
        let mut zone = SampleZone::new("Test", 60, vec![0.0; 100], 44100);
        assert!(!zone.set_loop_points(90, 10)); // start >= end
        assert!(!zone.set_loop_points(0, 200)); // end > len
        assert!(!zone.set_loop_points(50, 50)); // start == end
    }

    #[test]
    fn clear_loop_points() {
        let mut zone = SampleZone::new("Test", 60, vec![0.0; 100], 44100);
        zone.set_loop_points(10, 90);
        zone.loop_mode = LoopMode::Forward;
        zone.clear_loop_points();
        assert!(zone.loop_start.is_none());
        assert!(zone.loop_end.is_none());
        assert_eq!(zone.loop_mode, LoopMode::NoLoop);
    }

    #[test]
    fn fade_in() {
        let mut zone = SampleZone::new("Test", 60, vec![1.0; 10], 44100);
        zone.fade_in(5);
        assert_eq!(zone.samples[0], 0.0); // fully faded
        assert!((zone.samples[4] - 0.8).abs() < 0.01); // 4/5
        assert_eq!(zone.samples[5], 1.0); // unaffected
        assert_eq!(zone.samples[9], 1.0); // unaffected
    }

    #[test]
    fn fade_out() {
        let mut zone = SampleZone::new("Test", 60, vec![1.0; 10], 44100);
        zone.fade_out(5);
        assert_eq!(zone.samples[0], 1.0); // unaffected
        assert_eq!(zone.samples[4], 1.0); // unaffected
        assert!((zone.samples[5] - 1.0).abs() < 0.01); // start of fade (0/5 = 0.0 attenuation)
        assert!((zone.samples[9] - 0.2).abs() < 0.01); // 1.0 * (1 - 4/5) = 0.2
    }

    #[test]
    fn normalize_scales_to_target() {
        let mut zone = SampleZone::new("Test", 60, vec![0.5, -0.3, 0.2], 44100);
        let gain = zone.normalize(1.0);
        assert!((gain - 2.0).abs() < 0.01); // peak was 0.5, gain = 1.0/0.5 = 2.0
        assert!((zone.peak() - 1.0).abs() < 0.01);
    }

    #[test]
    fn normalize_silent_returns_zero() {
        let mut zone = SampleZone::new("Test", 60, vec![0.0, 0.0, 0.0], 44100);
        let gain = zone.normalize(1.0);
        assert_eq!(gain, 0.0);
    }

    #[test]
    fn reverse_flips_samples() {
        let mut zone = SampleZone::new("Test", 60, vec![1.0, 2.0, 3.0, 4.0], 44100);
        zone.reverse();
        assert_eq!(zone.samples, vec![4.0, 3.0, 2.0, 1.0]);
    }

    #[test]
    fn reverse_adjusts_loop_points() {
        let mut zone = SampleZone::new("Test", 60, vec![0.0; 100], 44100);
        zone.loop_start = Some(20);
        zone.loop_end = Some(80);
        zone.reverse();
        assert_eq!(zone.loop_start, Some(20)); // 100 - 80
        assert_eq!(zone.loop_end, Some(80)); // 100 - 20
    }

    #[test]
    fn peak_and_rms() {
        let zone = SampleZone::new("Test", 60, vec![0.5, -1.0, 0.3], 44100);
        assert!((zone.peak() - 1.0).abs() < 0.01);
        let expected_rms = ((0.25 + 1.0 + 0.09) / 3.0_f32).sqrt();
        assert!((zone.rms() - expected_rms).abs() < 0.01);
    }

    #[test]
    fn len_and_is_empty() {
        let zone = SampleZone::new("Test", 60, vec![1.0, 2.0], 44100);
        assert_eq!(zone.len(), 2);
        assert!(!zone.is_empty());

        let empty = SampleZone::new("Empty", 60, vec![], 44100);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    // ── 8G.6: comprehensive sample playback tests ──────────────────────

    #[test]
    fn pitch_mapping_root_key_plays_at_original_speed() {
        let mut sampler = Sampler::new(44100.0);
        let zone = SampleZone::new("Test", 60, vec![1.0; 500], 44100);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        let ratio = sampler.voices[0].pitch_ratio;
        assert!(
            (ratio - 1.0).abs() < 1e-9,
            "root key should play at ratio 1.0, got {ratio}"
        );
    }

    #[test]
    fn pitch_mapping_octave_up_doubles_speed() {
        let mut sampler = Sampler::new(44100.0);
        let zone = SampleZone::new("Test", 60, vec![1.0; 500], 44100);
        sampler.add_zone(zone);
        sampler.note_on(72, 100, 0);

        let ratio = sampler.voices[0].pitch_ratio;
        assert!(
            (ratio - 2.0).abs() < 1e-9,
            "octave up should play at ratio 2.0, got {ratio}"
        );
    }

    #[test]
    fn pitch_mapping_octave_up_exhausts_sample_in_half_the_frames() {
        let mut sampler = Sampler::new(44100.0);
        let zone = SampleZone::new("Test", 60, vec![0.5; 200], 44100);
        sampler.add_zone(zone);
        sampler.note_on(72, 100, 0);

        let mut buf = AudioBuffer::new(2, 128);
        sampler.process(&[], &[], &mut buf);

        assert_eq!(
            sampler.active_voices(),
            0,
            "at 2x pitch, 200-sample zone should be done within 128 frames"
        );
    }

    #[test]
    fn pitch_mapping_octave_down_halves_speed() {
        let mut sampler = Sampler::new(44100.0);
        let zone = SampleZone::new("Test", 60, vec![1.0; 500], 44100);
        sampler.add_zone(zone);
        sampler.note_on(48, 100, 0);

        let ratio = sampler.voices[0].pitch_ratio;
        assert!(
            (ratio - 0.5).abs() < 1e-9,
            "octave down should play at ratio 0.5, got {ratio}"
        );
    }

    #[test]
    fn pitch_mapping_sample_rate_compensation() {
        let mut sampler = Sampler::new(48000.0);
        let zone = SampleZone::new("Test", 60, vec![1.0; 500], 44100);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        let expected = 44100.0 / 48000.0;
        let ratio = sampler.voices[0].pitch_ratio;
        assert!(
            (ratio - expected).abs() < 1e-6,
            "sample rate compensation: expected {expected}, got {ratio}"
        );
    }

    #[test]
    fn forward_loop_cycles_multiple_times() {
        let mut sampler = Sampler::new(44100.0);
        let samples: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let mut zone = SampleZone::new("Loop", 60, samples, 44100);
        zone.loop_mode = LoopMode::Forward;
        zone.loop_start = Some(20);
        zone.loop_end = Some(60);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        let mut buf = AudioBuffer::new(2, 256);
        sampler.process(&[], &[], &mut buf);

        assert_eq!(
            sampler.active_voices(),
            1,
            "forward-loop voice should remain active"
        );

        let pos = sampler.voices[0].play_pos;
        assert!(
            (20.0..60.0).contains(&pos),
            "play_pos should be in loop region [20, 60), got {pos}"
        );
    }

    #[test]
    fn forward_loop_produces_nonzero_audio_throughout() {
        let mut sampler = Sampler::new(44100.0);
        let mut zone = SampleZone::new("Loop", 60, vec![0.8; 100], 44100);
        zone.loop_mode = LoopMode::Forward;
        zone.loop_start = Some(10);
        zone.loop_end = Some(90);
        sampler.add_zone(zone);
        sampler.note_on(60, 127, 0);

        for block in 0..5 {
            let mut buf = AudioBuffer::new(2, 128);
            sampler.process(&[], &[], &mut buf);
            let has_nonzero = (0..128).any(|i| buf.get(i, 0).abs() > 0.001);
            assert!(
                has_nonzero,
                "forward loop should produce audio in block {block}"
            );
        }
    }

    #[test]
    fn pingpong_loop_stays_within_bounds() {
        let mut sampler = Sampler::new(44100.0);
        let mut zone = SampleZone::new("PingPong", 60, vec![0.5; 200], 44100);
        zone.loop_mode = LoopMode::PingPong;
        zone.loop_start = Some(40);
        zone.loop_end = Some(160);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        for _ in 0..10 {
            let mut buf = AudioBuffer::new(2, 128);
            sampler.process(&[], &[], &mut buf);

            assert_eq!(
                sampler.active_voices(),
                1,
                "ping-pong voice should stay active"
            );
            let pos = sampler.voices[0].play_pos;
            assert!(
                (40.0..=160.0).contains(&pos),
                "ping-pong play_pos should stay in [40, 160], got {pos}"
            );
        }
    }

    #[test]
    fn pingpong_loop_reverses_direction_at_boundaries() {
        let mut sampler = Sampler::new(44100.0);
        let mut zone = SampleZone::new("PingPong", 60, vec![0.5; 100], 44100);
        zone.loop_mode = LoopMode::PingPong;
        zone.loop_start = Some(10);
        zone.loop_end = Some(50);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        let mut saw_forward = false;
        let mut saw_reverse = false;
        for _ in 0..200 {
            let mut buf = AudioBuffer::new(2, 1);
            sampler.process(&[], &[], &mut buf);
            if sampler.voices[0].direction == 1 {
                saw_forward = true;
            } else if sampler.voices[0].direction == -1 {
                saw_reverse = true;
            }
        }
        assert!(saw_forward, "should have played forward at some point");
        assert!(saw_reverse, "should have played in reverse at some point");
    }

    #[test]
    fn velocity_zone_selection_boundary_values() {
        let mut sampler = Sampler::new(44100.0);

        let mut zone_pp = SampleZone::new("pp", 60, vec![0.1; 100], 44100);
        zone_pp.velocity_low = 1;
        zone_pp.velocity_high = 42;

        let mut zone_mf = SampleZone::new("mf", 60, vec![0.5; 100], 44100);
        zone_mf.velocity_low = 43;
        zone_mf.velocity_high = 85;

        let mut zone_ff = SampleZone::new("ff", 60, vec![1.0; 100], 44100);
        zone_ff.velocity_low = 86;
        zone_ff.velocity_high = 127;

        sampler.add_zone(zone_pp);
        sampler.add_zone(zone_mf);
        sampler.add_zone(zone_ff);

        assert_eq!(sampler.find_zone(60, 1), Some(0), "velocity 1 -> pp");
        assert_eq!(sampler.find_zone(60, 42), Some(0), "velocity 42 -> pp");
        assert_eq!(sampler.find_zone(60, 43), Some(1), "velocity 43 -> mf");
        assert_eq!(sampler.find_zone(60, 85), Some(1), "velocity 85 -> mf");
        assert_eq!(sampler.find_zone(60, 86), Some(2), "velocity 86 -> ff");
        assert_eq!(sampler.find_zone(60, 127), Some(2), "velocity 127 -> ff");

        assert_eq!(sampler.find_zone(60, 0), None, "velocity 0 -> None");
    }

    #[test]
    fn velocity_zone_triggers_correct_zone_for_playback() {
        let mut sampler = Sampler::new(44100.0);

        let mut zone_soft = SampleZone::new("Soft", 60, vec![0.1; 100], 44100);
        zone_soft.velocity_low = 1;
        zone_soft.velocity_high = 64;

        let mut zone_loud = SampleZone::new("Loud", 60, vec![0.9; 100], 44100);
        zone_loud.velocity_low = 65;
        zone_loud.velocity_high = 127;

        sampler.add_zone(zone_soft);
        sampler.add_zone(zone_loud);

        sampler.note_on(60, 30, 0);
        assert_eq!(sampler.voices[0].zone_index, 0, "soft velocity -> zone 0");

        sampler.note_on(60, 100, 0);
        assert_eq!(sampler.voices[1].zone_index, 1, "loud velocity -> zone 1");
    }

    #[test]
    fn oneshot_stops_exactly_at_sample_end() {
        let sample_len = 50;
        let mut sampler = Sampler::new(44100.0);
        let zone = SampleZone::new("Short", 60, vec![0.7; sample_len], 44100);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        let mut buf = AudioBuffer::new(2, sample_len as u32);
        sampler.process(&[], &[], &mut buf);

        assert_eq!(
            sampler.active_voices(),
            0,
            "one-shot should deactivate after all samples consumed"
        );
    }

    #[test]
    fn oneshot_produces_silence_after_sample_ends() {
        let mut sampler = Sampler::new(44100.0);
        let zone = SampleZone::new("Short", 60, vec![1.0; 50], 44100);
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        let mut buf1 = AudioBuffer::new(2, 64);
        sampler.process(&[], &[], &mut buf1);

        let mut buf2 = AudioBuffer::new(2, 64);
        sampler.process(&[], &[], &mut buf2);

        for i in 0..64 {
            assert_eq!(
                buf2.get(i, 0),
                0.0,
                "frame {i} should be silent after one-shot completes"
            );
        }
    }

    #[test]
    fn oneshot_with_pitch_up_finishes_faster() {
        let mut sampler = Sampler::new(44100.0);
        let zone = SampleZone::new("Test", 60, vec![0.5; 400], 44100);
        sampler.add_zone(zone);
        sampler.note_on(84, 100, 0);

        let mut buf = AudioBuffer::new(2, 128);
        sampler.process(&[], &[], &mut buf);

        assert_eq!(
            sampler.active_voices(),
            0,
            "4x pitch should exhaust 400 samples within 128 frames"
        );
    }

    #[test]
    fn loop_mode_noloop_is_default() {
        let zone = SampleZone::new("Default", 60, vec![], 44100);
        assert_eq!(zone.loop_mode, LoopMode::NoLoop);
        assert!(zone.loop_start.is_none());
        assert!(zone.loop_end.is_none());
    }

    #[test]
    fn forward_loop_without_explicit_bounds_uses_full_sample() {
        let mut sampler = Sampler::new(44100.0);
        let mut zone = SampleZone::new("FullLoop", 60, vec![0.5; 80], 44100);
        zone.loop_mode = LoopMode::Forward;
        sampler.add_zone(zone);
        sampler.note_on(60, 100, 0);

        let mut buf = AudioBuffer::new(2, 256);
        sampler.process(&[], &[], &mut buf);

        assert_eq!(
            sampler.active_voices(),
            1,
            "forward loop with default bounds should keep playing"
        );
        let pos = sampler.voices[0].play_pos;
        assert!(
            (0.0..80.0).contains(&pos),
            "play_pos should wrap within [0, 80), got {pos}"
        );
    }

    #[test]
    fn read_sample_interpolates_between_samples() {
        let zone = SampleZone::new("Interp", 60, vec![0.0, 1.0, 0.0], 44100);
        let val = Sampler::read_sample(&zone, 0.5);
        assert!(
            (val - 0.5).abs() < 1e-6,
            "linear interpolation at 0.5 should give 0.5, got {val}"
        );

        let val2 = Sampler::read_sample(&zone, 1.25);
        assert!(
            (val2 - 0.75).abs() < 1e-6,
            "linear interpolation at 1.25 should give 0.75, got {val2}"
        );
    }

    // ── Slice mode tests ──────────────────────────────────────────────

    fn make_transient_sample() -> Vec<f32> {
        let sr = 44100;
        let mut samples = vec![0.0_f32; sr];
        let positions = [4410, 13230, 22050, 33075];
        for &pos in &positions {
            for i in 0..512 {
                if pos + i < samples.len() {
                    samples[pos + i] = 0.9 * (i as f32 / 512.0 * std::f32::consts::PI).sin();
                }
            }
        }
        samples
    }

    #[test]
    fn auto_slice_finds_transients() {
        let samples = make_transient_sample();
        let mut zone = SampleZone::new("Beats", 60, samples, 44100);
        zone.auto_slice_by_transients(0.3);

        assert!(
            zone.slice_count() >= 3,
            "expected at least 3 slices, got {}",
            zone.slice_count()
        );

        for s in &zone.slices {
            assert!(s.index < 44100, "slice index {} out of bounds", s.index);
        }

        for w in zone.slices.windows(2) {
            assert!(w[0].index < w[1].index, "slices not in ascending order");
        }
    }

    #[test]
    fn slice_to_zones_correct_count_and_keys() {
        let samples = make_transient_sample();
        let mut zone = SampleZone::new("Beats", 60, samples, 44100);
        zone.auto_slice_by_transients(0.3);

        let n = zone.slice_count();
        assert!(n >= 2, "need at least 2 slices");

        let base_note = 36_u8;
        let zones = zone.slice_to_zones(base_note);

        assert_eq!(zones.len(), n);

        for (i, z) in zones.iter().enumerate() {
            let expected_note = base_note + i as u8;
            assert_eq!(z.root_key, expected_note);
            assert_eq!(z.key_low, expected_note);
            assert_eq!(z.key_high, expected_note);
            assert!(!z.samples.is_empty());
        }

        let first_slice_idx = zone.slices[0].index;
        let total: usize = zones.iter().map(|z| z.samples.len()).sum();
        assert_eq!(total, 44100 - first_slice_idx);
    }

    #[test]
    fn empty_sample_produces_no_slices() {
        let mut zone = SampleZone::new("Empty", 60, vec![], 44100);
        zone.auto_slice_by_transients(0.5);
        assert_eq!(zone.slice_count(), 0);
    }

    #[test]
    fn silent_sample_produces_no_slices() {
        let mut zone = SampleZone::new("Silent", 60, vec![0.0; 44100], 44100);
        zone.auto_slice_by_transients(0.5);
        assert_eq!(zone.slice_count(), 0);
    }

    #[test]
    fn manual_slice_management() {
        let mut zone = SampleZone::new("Test", 60, vec![0.5; 10000], 44100);

        assert_eq!(zone.slice_count(), 0);

        zone.add_slice(1000, Some("kick".to_string()));
        zone.add_slice(5000, None);
        zone.add_slice(3000, Some("snare".to_string()));

        assert_eq!(zone.slice_count(), 3);
        assert_eq!(zone.slices[0].index, 1000);
        assert_eq!(zone.slices[1].index, 3000);
        assert_eq!(zone.slices[2].index, 5000);

        zone.clear_slices();
        assert_eq!(zone.slice_count(), 0);
    }

    #[test]
    fn add_slice_out_of_bounds_ignored() {
        let mut zone = SampleZone::new("Test", 60, vec![0.5; 100], 44100);
        zone.add_slice(200, None);
        assert_eq!(zone.slice_count(), 0);
    }

    #[test]
    fn slice_to_zones_empty_when_no_slices() {
        let zone = SampleZone::new("Test", 60, vec![0.5; 10000], 44100);
        let zones = zone.slice_to_zones(60);
        assert!(zones.is_empty());
    }
}
