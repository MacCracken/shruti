use serde::{Deserialize, Serialize};
use shruti_dsp::AudioBuffer;
use shruti_session::midi::{ControlChange, NoteEvent};

use crate::effect_chain::EffectChain;
use crate::instrument::{InstrumentInfo, InstrumentNode, InstrumentParam};

pub const NUM_PADS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayMode {
    OneShot,
    Looped,
}

/// Per-pad effect settings.
#[derive(Debug, Clone)]
pub struct PadEffects {
    /// Filter cutoff (0.0–1.0, where 1.0 = fully open / no filtering).
    pub filter_cutoff: f32,
    /// Drive amount (1.0 = clean, higher = more saturation).
    pub drive: f32,
    /// Send level to reverb bus (0.0–1.0).
    pub reverb_send: f32,
    /// Send level to delay bus (0.0–1.0).
    pub delay_send: f32,
    /// One-pole filter state (per channel: left, right).
    filter_state: [f32; 2],
}

impl Default for PadEffects {
    fn default() -> Self {
        Self {
            filter_cutoff: 1.0,
            drive: 1.0,
            reverb_send: 0.0,
            delay_send: 0.0,
            filter_state: [0.0; 2],
        }
    }
}

impl PadEffects {
    /// Process a stereo sample pair through filter and drive.
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let (mut l, mut r) = (left, right);

        // One-pole lowpass filter (cutoff < 1.0 enables filtering)
        if self.filter_cutoff < 1.0 {
            // Map cutoff 0..1 to coefficient: 0 = fully filtered, 1 = open
            let coeff = self.filter_cutoff.clamp(0.0, 1.0);
            self.filter_state[0] += coeff * (l - self.filter_state[0]);
            self.filter_state[1] += coeff * (r - self.filter_state[1]);
            l = self.filter_state[0];
            r = self.filter_state[1];
        }

        // Tanh soft-clip drive (drive > 1.0 adds saturation)
        if self.drive > 1.0 {
            l = (l * self.drive).tanh();
            r = (r * self.drive).tanh();
        }

        (l, r)
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.filter_state = [0.0; 2];
    }
}

/// A single drum pad with a sample buffer and settings.
pub struct DrumPad {
    pub name: String,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub pitch: f32,
    pub gain: f32,
    pub pan: f32,
    pub decay: f32,
    pub play_mode: PlayMode,
    pub midi_note: u8,
    /// Per-pad effects (filter, drive, sends).
    pub effects: PadEffects,
    play_pos: f64,
    playing: bool,
    velocity: f32,
    envelope: f32,
}

impl DrumPad {
    pub fn new(name: &str, midi_note: u8) -> Self {
        Self {
            name: name.to_string(),
            samples: Vec::new(),
            sample_rate: 44100,
            pitch: 1.0,
            gain: 1.0,
            pan: 0.0,
            decay: 0.0,
            play_mode: PlayMode::OneShot,
            midi_note,
            effects: PadEffects::default(),
            play_pos: 0.0,
            playing: false,
            velocity: 0.0,
            envelope: 0.0,
        }
    }

    pub fn load_sample(&mut self, samples: Vec<f32>, sample_rate: u32) {
        self.samples = samples;
        self.sample_rate = sample_rate;
    }

    pub fn trigger(&mut self, velocity: u8) {
        self.play_pos = 0.0;
        self.playing = true;
        self.velocity = velocity as f32 / 127.0;
        self.envelope = 1.0;
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }

    /// Generate one frame of audio. Returns (left, right) incorporating pan.
    pub fn tick(&mut self) -> (f32, f32) {
        if !self.playing || self.samples.is_empty() {
            return (0.0, 0.0);
        }

        let len = self.samples.len();
        let pos = self.play_pos;
        let idx = pos as usize;

        // Check bounds
        if idx >= len {
            match self.play_mode {
                PlayMode::OneShot => {
                    self.playing = false;
                    return (0.0, 0.0);
                }
                PlayMode::Looped => {
                    self.play_pos %= len as f64;
                    // Recalculate after wrapping
                    let idx = self.play_pos as usize;
                    if idx >= len {
                        return (0.0, 0.0);
                    }
                }
            }
        }

        // Linear interpolation
        let idx = self.play_pos as usize;
        let frac = (self.play_pos - idx as f64) as f32;
        let s0 = self.samples[idx];
        let s1 = if idx + 1 < len {
            self.samples[idx + 1]
        } else {
            match self.play_mode {
                PlayMode::Looped => self.samples[0],
                PlayMode::OneShot => 0.0,
            }
        };
        let sample = s0 + frac * (s1 - s0);

        // Apply envelope, velocity, gain
        let out = sample * self.envelope * self.velocity * self.gain;

        // Advance position
        self.play_pos += self.pitch as f64;

        // Apply decay envelope
        // decay_rate: decay=0 means no decay (envelope stays at 1), decay=1 means fast decay
        let decay_rate = 1.0 - self.decay * 0.001;
        self.envelope *= decay_rate;

        // Check if we've gone past end after advancing
        if self.play_pos as usize >= len {
            match self.play_mode {
                PlayMode::OneShot => {
                    self.playing = false;
                }
                PlayMode::Looped => {
                    self.play_pos %= len as f64;
                }
            }
        }

        // Equal-power pan law
        // pan: -1.0 = full left, 0.0 = center, 1.0 = full right
        let pan_normalized = (self.pan + 1.0) * 0.5; // 0.0 to 1.0
        let angle = pan_normalized * std::f32::consts::FRAC_PI_2;
        let left_gain = angle.cos();
        let right_gain = angle.sin();

        let (l, r) = (out * left_gain, out * right_gain);

        // Apply per-pad effects (filter + drive)
        self.effects.process(l, r)
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }
}

const GM_DRUM_NAMES: [&str; NUM_PADS] = [
    "Bass Drum",
    "Rim Shot",
    "Snare",
    "Clap",
    "Snare 2",
    "Low Tom",
    "Closed HH",
    "Mid Tom",
    "Open HH",
    "High Tom",
    "Crash",
    "Ride",
    "Tambourine",
    "Cowbell",
    "Shaker",
    "Claves",
];

/// 16-pad drum machine implementing InstrumentNode.
pub struct DrumMachine {
    info: InstrumentInfo,
    params: Vec<InstrumentParam>,
    pub pads: Vec<DrumPad>,
    sample_rate: f32,
    /// Per-instrument effect chain.
    pub effect_chain: EffectChain,
}

impl DrumMachine {
    pub fn new(sample_rate: f32) -> Self {
        let info = InstrumentInfo {
            name: "Drum Machine".to_string(),
            category: "Drums".to_string(),
            author: "Shruti".to_string(),
            description: "16-pad drum machine with sample playback".to_string(),
        };

        let params = vec![InstrumentParam::new("Volume", 0.0, 1.0, 0.8, "")];

        let pads = (0..NUM_PADS)
            .map(|i| DrumPad::new(GM_DRUM_NAMES[i], 36 + i as u8))
            .collect();

        Self {
            info,
            params,
            pads,
            sample_rate,
            effect_chain: EffectChain::new(),
        }
    }

    fn find_pad_by_note(&self, note: u8) -> Option<usize> {
        self.pads.iter().position(|p| p.midi_note == note)
    }

    /// Render pads into output buffer (adds to existing content).
    fn render_pads(&mut self, note_events: &[NoteEvent], output: &mut AudioBuffer) {
        let frames = output.frames();
        let volume = self.params[0].value;

        for event in note_events {
            self.note_on(event.note, event.velocity, event.channel);
        }

        for frame in 0..frames {
            let mut left = 0.0_f32;
            let mut right = 0.0_f32;

            for pad in &mut self.pads {
                if pad.is_playing() {
                    let (l, r) = pad.tick();
                    left += l;
                    right += r;
                }
            }

            left *= volume;
            right *= volume;

            if output.channels() >= 2 {
                let cur_l = output.get(frame, 0);
                let cur_r = output.get(frame, 1);
                output.set(frame, 0, cur_l + left);
                output.set(frame, 1, cur_r + right);
            } else if output.channels() == 1 {
                let cur = output.get(frame, 0);
                output.set(frame, 0, cur + (left + right) * 0.5);
            }
        }
    }
}

impl InstrumentNode for DrumMachine {
    fn info(&self) -> &InstrumentInfo {
        &self.info
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.effect_chain.set_sample_rate(sample_rate);
    }

    fn process(
        &mut self,
        note_events: &[NoteEvent],
        _control_changes: &[ControlChange],
        output: &mut AudioBuffer,
    ) {
        let has_active_effects = self.effect_chain.effects().iter().any(|e| e.enabled);

        if has_active_effects {
            let mut chain = std::mem::take(&mut self.effect_chain);
            chain.process_with(output, |buf| {
                self.render_pads(note_events, buf);
            });
            self.effect_chain = chain;
        } else {
            self.render_pads(note_events, output);
        }
    }

    fn note_on(&mut self, note: u8, velocity: u8, _channel: u8) {
        if let Some(idx) = self.find_pad_by_note(note) {
            self.pads[idx].trigger(velocity);
        }
    }

    fn note_off(&mut self, note: u8, _channel: u8) {
        if let Some(idx) = self.find_pad_by_note(note)
            && self.pads[idx].play_mode == PlayMode::Looped
        {
            self.pads[idx].stop();
        }
    }

    fn params(&self) -> &[InstrumentParam] {
        &self.params
    }

    fn params_mut(&mut self) -> &mut [InstrumentParam] {
        &mut self.params
    }

    fn reset(&mut self) {
        for pad in &mut self.pads {
            pad.stop();
        }
        self.effect_chain.reset();
    }

    fn active_voices(&self) -> usize {
        self.pads.iter().filter(|p| p.is_playing()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine_sample(len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / len as f32).sin())
            .collect()
    }

    #[test]
    fn pad_trigger_and_stop() {
        let mut pad = DrumPad::new("Kick", 36);
        pad.load_sample(vec![1.0; 100], 44100);

        assert!(!pad.is_playing());
        pad.trigger(127);
        assert!(pad.is_playing());
        pad.stop();
        assert!(!pad.is_playing());
    }

    #[test]
    fn pad_oneshot_completes_and_stops() {
        let mut pad = DrumPad::new("Kick", 36);
        pad.load_sample(vec![0.5; 10], 44100);
        pad.play_mode = PlayMode::OneShot;
        pad.trigger(127);

        // Tick through the entire sample
        for _ in 0..20 {
            pad.tick();
        }
        assert!(!pad.is_playing());
    }

    #[test]
    fn pad_pitch_shifting_faster() {
        let mut pad = DrumPad::new("Kick", 36);
        pad.load_sample(vec![1.0; 100], 44100);
        pad.decay = 0.0;
        pad.play_mode = PlayMode::OneShot;
        pad.pitch = 2.0;
        pad.trigger(127);

        let mut ticks = 0;
        while pad.is_playing() {
            pad.tick();
            ticks += 1;
            if ticks > 200 {
                break;
            }
        }
        // At pitch 2.0, should finish in ~50 ticks instead of 100
        assert!(
            ticks <= 55,
            "pitch=2.0 should halve playback time, got {ticks}"
        );
        assert!(!pad.is_playing());
    }

    #[test]
    fn pad_pan_left() {
        let mut pad = DrumPad::new("Kick", 36);
        pad.load_sample(vec![1.0; 100], 44100);
        pad.decay = 0.0;
        pad.pan = -1.0; // full left
        pad.trigger(127);

        let (l, r) = pad.tick();
        assert!(l.abs() > 0.01, "left channel should have signal");
        assert!(
            r.abs() < 0.001,
            "right channel should be near zero for full left pan"
        );
    }

    #[test]
    fn pad_pan_right() {
        let mut pad = DrumPad::new("HH", 42);
        pad.load_sample(vec![1.0; 100], 44100);
        pad.decay = 0.0;
        pad.pan = 1.0; // full right
        pad.trigger(127);

        let (l, r) = pad.tick();
        assert!(
            l.abs() < 0.001,
            "left channel should be near zero for full right pan"
        );
        assert!(r.abs() > 0.01, "right channel should have signal");
    }

    #[test]
    fn pad_pan_center() {
        let mut pad = DrumPad::new("Snare", 38);
        pad.load_sample(vec![1.0; 100], 44100);
        pad.decay = 0.0;
        pad.pan = 0.0;
        pad.trigger(127);

        let (l, r) = pad.tick();
        assert!(
            (l - r).abs() < 0.01,
            "center pan should produce equal L/R, got l={l} r={r}"
        );
    }

    #[test]
    fn drum_machine_routes_midi_to_pads() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(vec![1.0; 100], 44100);
        dm.pads[1].load_sample(vec![1.0; 100], 44100);

        // Trigger pad 0 (note 36) and pad 1 (note 37)
        dm.note_on(36, 100, 0);
        assert!(dm.pads[0].is_playing());
        assert!(!dm.pads[1].is_playing());

        dm.note_on(37, 100, 0);
        assert!(dm.pads[1].is_playing());
    }

    #[test]
    fn drum_machine_silence_without_triggers() {
        let mut dm = DrumMachine::new(44100.0);
        let mut buf = AudioBuffer::new(2, 128);
        dm.process(&[], &[], &mut buf);

        for i in 0..128 {
            assert_eq!(buf.get(i, 0), 0.0);
            assert_eq!(buf.get(i, 1), 0.0);
        }
    }

    #[test]
    fn drum_machine_produces_audio_after_trigger() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(make_sine_sample(1000), 44100);
        dm.note_on(36, 127, 0);

        let mut buf = AudioBuffer::new(2, 256);
        dm.process(&[], &[], &mut buf);

        let mut has_nonzero = false;
        for i in 0..256 {
            if buf.get(i, 0).abs() > 0.001 {
                has_nonzero = true;
                break;
            }
        }
        assert!(
            has_nonzero,
            "drum machine should produce audio after trigger"
        );
    }

    #[test]
    fn drum_machine_multiple_simultaneous_pads() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(vec![0.5; 100], 44100);
        dm.pads[1].load_sample(vec![0.3; 100], 44100);

        dm.note_on(36, 127, 0);
        dm.note_on(37, 127, 0);

        assert_eq!(dm.active_voices(), 2);

        let mut buf = AudioBuffer::new(2, 10);
        dm.process(&[], &[], &mut buf);

        // Should have combined output from both pads
        let val = buf.get(0, 0);
        assert!(val.abs() > 0.3, "multiple pads should combine: got {val}");
    }

    #[test]
    fn velocity_affects_volume() {
        let mut pad_loud = DrumPad::new("Kick", 36);
        pad_loud.load_sample(vec![1.0; 100], 44100);
        pad_loud.decay = 0.0;
        pad_loud.trigger(127);
        let (loud_l, _) = pad_loud.tick();

        let mut pad_soft = DrumPad::new("Kick", 36);
        pad_soft.load_sample(vec![1.0; 100], 44100);
        pad_soft.decay = 0.0;
        pad_soft.trigger(32);
        let (soft_l, _) = pad_soft.tick();

        assert!(
            loud_l > soft_l,
            "higher velocity should be louder: loud={loud_l}, soft={soft_l}"
        );
    }

    #[test]
    fn load_sample_works() {
        let mut pad = DrumPad::new("Test", 36);
        assert!(pad.samples.is_empty());

        pad.load_sample(vec![0.1, 0.2, 0.3], 48000);
        assert_eq!(pad.samples.len(), 3);
        assert_eq!(pad.sample_rate, 48000);
    }

    #[test]
    fn pad_looped_mode_wraps() {
        let mut pad = DrumPad::new("Loop", 36);
        pad.load_sample(vec![1.0; 10], 44100);
        pad.play_mode = PlayMode::Looped;
        pad.decay = 0.0;
        pad.trigger(127);

        // Tick well past the sample length
        for _ in 0..30 {
            pad.tick();
        }
        // Should still be playing because it loops
        assert!(pad.is_playing());
    }

    #[test]
    fn drum_machine_process_with_note_events() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(make_sine_sample(1000), 44100);

        let events = vec![NoteEvent {
            position: 0,
            duration: 1000,
            note: 36,
            velocity: 100,
            channel: 0,
        }];

        let mut buf = AudioBuffer::new(2, 128);
        dm.process(&events, &[], &mut buf);

        let mut has_nonzero = false;
        for i in 0..128 {
            if buf.get(i, 0).abs() > 0.001 {
                has_nonzero = true;
                break;
            }
        }
        assert!(has_nonzero, "process should handle NoteEvent triggers");
    }

    #[test]
    fn drum_machine_reset_stops_all_pads() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(vec![1.0; 100], 44100);
        dm.pads[1].load_sample(vec![1.0; 100], 44100);
        dm.note_on(36, 100, 0);
        dm.note_on(37, 100, 0);
        assert_eq!(dm.active_voices(), 2);

        dm.reset();
        assert_eq!(dm.active_voices(), 0);
    }

    #[test]
    fn pad_effects_default_is_clean() {
        let effects = PadEffects::default();
        assert_eq!(effects.filter_cutoff, 1.0);
        assert_eq!(effects.drive, 1.0);
        assert_eq!(effects.reverb_send, 0.0);
        assert_eq!(effects.delay_send, 0.0);
    }

    #[test]
    fn pad_effects_passthrough_when_clean() {
        let mut effects = PadEffects::default();
        let (l, r) = effects.process(0.5, -0.3);
        assert!((l - 0.5).abs() < 1e-6);
        assert!((r - (-0.3)).abs() < 1e-6);
    }

    #[test]
    fn pad_effects_filter_attenuates() {
        let mut effects = PadEffects::default();
        effects.filter_cutoff = 0.05; // very low cutoff

        // Feed an alternating signal (high frequency content) — filter should smooth it
        let mut sum_filtered = 0.0_f32;
        let mut sum_dry = 0.0_f32;
        for i in 0..100 {
            let input = if i % 2 == 0 { 1.0 } else { -1.0 };
            let (l, _) = effects.process(input, 0.0);
            sum_filtered += l.abs();
            sum_dry += input.abs();
        }
        // Filtered signal should have less total energy than dry
        assert!(
            sum_filtered < sum_dry * 0.5,
            "low cutoff filter should attenuate high-freq content: filtered={sum_filtered}, dry={sum_dry}"
        );
    }

    #[test]
    fn pad_effects_drive_saturates() {
        let mut effects = PadEffects::default();
        effects.drive = 5.0;

        let (l, _) = effects.process(0.8, 0.0);
        // tanh(0.8 * 5.0) = tanh(4.0) ≈ 0.9993
        assert!(
            l > 0.99,
            "high drive should saturate signal near 1.0, got {l}"
        );

        // Soft signal should be less affected
        let (l_soft, _) = effects.process(0.1, 0.0);
        // tanh(0.1 * 5.0) = tanh(0.5) ≈ 0.462
        assert!(
            (l_soft - 0.1).abs() > 0.01,
            "drive should change the signal shape"
        );
    }

    #[test]
    fn pad_with_filter_produces_different_output() {
        let mut pad_clean = DrumPad::new("Clean", 36);
        pad_clean.load_sample(vec![1.0; 100], 44100);
        pad_clean.decay = 0.0;
        pad_clean.trigger(127);

        let mut pad_filtered = DrumPad::new("Filtered", 36);
        pad_filtered.load_sample(vec![1.0; 100], 44100);
        pad_filtered.decay = 0.0;
        pad_filtered.effects.filter_cutoff = 0.1;
        pad_filtered.trigger(127);

        let (clean_l, _) = pad_clean.tick();
        let (filt_l, _) = pad_filtered.tick();

        // First sample through a low-cutoff filter should be quieter
        assert!(
            filt_l.abs() < clean_l.abs(),
            "filtered pad should be quieter on first sample: clean={clean_l}, filtered={filt_l}"
        );
    }

    #[test]
    fn pad_effects_reset_clears_filter_state() {
        let mut effects = PadEffects::default();
        effects.filter_cutoff = 0.5;
        effects.process(1.0, 1.0);
        assert!(effects.filter_state[0] != 0.0);

        effects.reset();
        assert_eq!(effects.filter_state[0], 0.0);
        assert_eq!(effects.filter_state[1], 0.0);
    }
}
