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

/// Selection mode when multiple samples match a velocity range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerSelection {
    /// Cycle through samples in order.
    RoundRobin,
    /// Pick a random sample each trigger.
    Random,
}

/// A velocity-mapped sample layer for a drum pad.
#[derive(Debug, Clone)]
pub struct SampleLayer {
    /// Sample data.
    pub samples: Vec<f32>,
    /// Sample rate of this layer.
    pub sample_rate: u32,
    /// Minimum velocity (0–127) to trigger this layer.
    pub velocity_low: u8,
    /// Maximum velocity (0–127) to trigger this layer.
    pub velocity_high: u8,
}

impl SampleLayer {
    pub fn new(samples: Vec<f32>, sample_rate: u32, velocity_low: u8, velocity_high: u8) -> Self {
        Self {
            samples,
            sample_rate,
            velocity_low,
            velocity_high,
        }
    }

    /// Check if this layer matches the given velocity.
    pub fn matches(&self, velocity: u8) -> bool {
        velocity >= self.velocity_low && velocity <= self.velocity_high
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
    /// Velocity-mapped sample layers (up to 8). If empty, uses `samples` field.
    pub layers: Vec<SampleLayer>,
    /// How to select among multiple matching layers.
    pub layer_selection: LayerSelection,
    /// Round-robin counter for layer selection.
    round_robin_idx: usize,
    play_pos: f64,
    playing: bool,
    velocity: f32,
    envelope: f32,
    /// Currently active sample data pointer (index into layers, or usize::MAX for main samples).
    active_layer: usize,
    /// Cached pan gains (recomputed when pan changes).
    pan_gain_l: f32,
    pan_gain_r: f32,
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
            layers: Vec::new(),
            layer_selection: LayerSelection::RoundRobin,
            round_robin_idx: 0,
            play_pos: 0.0,
            playing: false,
            velocity: 0.0,
            envelope: 0.0,
            active_layer: usize::MAX,
            pan_gain_l: std::f32::consts::FRAC_PI_4.cos(),
            pan_gain_r: std::f32::consts::FRAC_PI_4.sin(),
        }
    }

    /// Recompute cached pan gains from the current `pan` value.
    /// Call this after changing `pan`.
    pub fn update_pan_gains(&mut self) {
        let pan_normalized = (self.pan + 1.0) * 0.5;
        let angle = pan_normalized * std::f32::consts::FRAC_PI_2;
        self.pan_gain_l = angle.cos();
        self.pan_gain_r = angle.sin();
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

        // Select active layer based on velocity
        self.active_layer = self.select_layer(velocity);
    }

    /// Add a velocity-mapped sample layer.
    pub fn add_layer(&mut self, layer: SampleLayer) {
        self.layers.push(layer);
    }

    /// Remove a layer by index.
    pub fn remove_layer(&mut self, index: usize) -> Option<SampleLayer> {
        if index < self.layers.len() {
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Remove all layers.
    pub fn clear_layers(&mut self) {
        self.layers.clear();
        self.active_layer = usize::MAX;
    }

    /// Select a layer index matching the given velocity, or usize::MAX for main samples.
    /// Uses iterator-based counting to avoid heap allocation in the audio thread.
    fn select_layer(&mut self, velocity: u8) -> usize {
        if self.layers.is_empty() {
            return usize::MAX;
        }

        // Count matching layers without allocating a Vec
        let match_count = self.layers.iter().filter(|l| l.matches(velocity)).count();

        if match_count == 0 {
            return usize::MAX; // fall back to main samples
        }

        if match_count == 1 {
            return self
                .layers
                .iter()
                .enumerate()
                .find(|(_, l)| l.matches(velocity))
                .map(|(i, _)| i)
                .unwrap();
        }

        // Multiple matches -- select the nth matching layer without allocating
        let selected_nth = match self.layer_selection {
            LayerSelection::RoundRobin => {
                let idx = self.round_robin_idx % match_count;
                self.round_robin_idx = self.round_robin_idx.wrapping_add(1);
                idx
            }
            LayerSelection::Random => {
                let pseudo = self.round_robin_idx.wrapping_mul(2654435761);
                self.round_robin_idx = self.round_robin_idx.wrapping_add(1);
                pseudo % match_count
            }
        };

        // Find the nth matching layer by iterating (no allocation)
        self.layers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.matches(velocity))
            .nth(selected_nth)
            .map(|(i, _)| i)
            .unwrap_or(usize::MAX)
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }

    /// Get the active sample buffer (layer or main).
    fn active_samples(&self) -> &[f32] {
        if self.active_layer < self.layers.len() {
            &self.layers[self.active_layer].samples
        } else {
            &self.samples
        }
    }

    /// Generate one frame of audio. Returns (left, right) incorporating pan.
    pub fn tick(&mut self) -> (f32, f32) {
        let samples = self.active_samples();
        if !self.playing || samples.is_empty() {
            return (0.0, 0.0);
        }

        let len = samples.len();
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
                    let idx = self.play_pos as usize;
                    if idx >= len {
                        return (0.0, 0.0);
                    }
                }
            }
        }

        // Read sample values before mutating self
        let idx = self.play_pos as usize;
        let frac = (self.play_pos - idx as f64) as f32;
        let samples = self.active_samples();
        let s0 = samples[idx];
        let s1 = if idx + 1 < len {
            samples[idx + 1]
        } else {
            match self.play_mode {
                PlayMode::Looped => samples[0],
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

        let (l, r) = (out * self.pan_gain_l, out * self.pan_gain_r);

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

use crate::instrument::ParamIndex;

/// Type-safe parameter indices for [`DrumMachine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum DrumMachineParam {
    Volume = 0,
}

impl ParamIndex for DrumMachineParam {
    fn index(self) -> usize {
        self as usize
    }
    fn count() -> usize {
        DrumMachineParam::Volume as usize + 1
    }
}

impl From<DrumMachineParam> for usize {
    fn from(p: DrumMachineParam) -> usize {
        p as usize
    }
}

impl TryFrom<usize> for DrumMachineParam {
    type Error = ();
    fn try_from(v: usize) -> Result<Self, ()> {
        match v {
            0 => Ok(Self::Volume),
            _ => Err(()),
        }
    }
}

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

    /// Get a parameter value using a type-safe [`DrumMachineParam`] key.
    pub fn get_param(&self, param: DrumMachineParam) -> f32 {
        self.params[param.index()].value
    }

    /// Set a parameter value using a type-safe [`DrumMachineParam`] key.
    pub fn set_param(&mut self, param: DrumMachineParam, value: f32) {
        self.params[param.index()].set(value);
    }

    fn find_pad_by_note(&self, note: u8) -> Option<usize> {
        self.pads.iter().position(|p| p.midi_note == note)
    }

    /// Render pads into output buffer (adds to existing content).
    fn render_pads(&mut self, note_events: &[NoteEvent], output: &mut AudioBuffer) {
        let frames = output.frames();
        let volume = self.params[DrumMachineParam::Volume.index()].value;

        for event in note_events {
            self.note_on(event.note, event.velocity, event.channel);
        }

        // Update cached pan gains once per buffer (avoids sin/cos per sample)
        for pad in &mut self.pads {
            pad.update_pan_gains();
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
        pad.update_pan_gains();
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
        pad.update_pan_gains();
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
        pad.update_pan_gains();
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
            position: shruti_session::FramePos(0),
            duration: shruti_session::FramePos(1000),
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
        let mut effects = PadEffects {
            filter_cutoff: 0.05, // very low cutoff
            ..PadEffects::default()
        };

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
        let mut effects = PadEffects {
            drive: 5.0,
            ..PadEffects::default()
        };

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
        let mut effects = PadEffects {
            filter_cutoff: 0.5,
            ..PadEffects::default()
        };
        effects.process(1.0, 1.0);
        assert!(effects.filter_state[0] != 0.0);

        effects.reset();
        assert_eq!(effects.filter_state[0], 0.0);
        assert_eq!(effects.filter_state[1], 0.0);
    }

    // --- Sample layering tests ---

    #[test]
    fn layer_velocity_selection() {
        let mut pad = DrumPad::new("Kick", 36);
        // Main sample (fallback)
        pad.load_sample(vec![0.1; 50], 44100);
        // Soft layer: velocity 1–63
        pad.add_layer(SampleLayer::new(vec![0.5; 50], 44100, 1, 63));
        // Hard layer: velocity 64–127
        pad.add_layer(SampleLayer::new(vec![1.0; 50], 44100, 64, 127));

        // Trigger with soft velocity
        pad.trigger(32);
        assert_eq!(pad.active_layer, 0, "soft velocity should select layer 0");
        pad.decay = 0.0;
        let (l, _) = pad.tick();
        // Layer 0 has samples of 0.5
        let expected = 0.5 * (32.0 / 127.0);
        assert!(
            (l.abs() - expected).abs() < 0.1,
            "soft layer should produce ~{expected}, got {l}"
        );

        // Trigger with hard velocity
        pad.trigger(100);
        assert_eq!(pad.active_layer, 1, "hard velocity should select layer 1");
    }

    #[test]
    fn layer_fallback_to_main_samples() {
        let mut pad = DrumPad::new("Kick", 36);
        pad.load_sample(vec![0.8; 50], 44100);
        // Layer only matches velocity 100–127
        pad.add_layer(SampleLayer::new(vec![1.0; 50], 44100, 100, 127));

        // Trigger with velocity 50 — no layer matches, should use main
        pad.trigger(50);
        assert_eq!(
            pad.active_layer,
            usize::MAX,
            "should fall back to main samples"
        );
        pad.decay = 0.0;
        let (l, _) = pad.tick();
        let expected = 0.8 * (50.0 / 127.0);
        assert!(
            (l.abs() - expected).abs() < 0.1,
            "should use main sample value ~{expected}, got {l}"
        );
    }

    #[test]
    fn layer_round_robin_cycles() {
        let mut pad = DrumPad::new("Snare", 38);
        pad.layer_selection = LayerSelection::RoundRobin;
        // Three layers all matching full velocity range
        pad.add_layer(SampleLayer::new(vec![0.1; 50], 44100, 0, 127));
        pad.add_layer(SampleLayer::new(vec![0.2; 50], 44100, 0, 127));
        pad.add_layer(SampleLayer::new(vec![0.3; 50], 44100, 0, 127));

        let mut selected = Vec::new();
        for _ in 0..6 {
            pad.trigger(100);
            selected.push(pad.active_layer);
        }
        // Should cycle: 0, 1, 2, 0, 1, 2
        assert_eq!(selected, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn layer_random_selects_valid_layers() {
        let mut pad = DrumPad::new("HH", 42);
        pad.layer_selection = LayerSelection::Random;
        pad.add_layer(SampleLayer::new(vec![0.1; 50], 44100, 0, 127));
        pad.add_layer(SampleLayer::new(vec![0.2; 50], 44100, 0, 127));

        for _ in 0..20 {
            pad.trigger(100);
            assert!(
                pad.active_layer < 2,
                "random should pick valid layer index, got {}",
                pad.active_layer
            );
        }
    }

    #[test]
    fn layer_no_layers_uses_main() {
        let mut pad = DrumPad::new("Kick", 36);
        pad.load_sample(vec![0.5; 50], 44100);
        // No layers added
        pad.trigger(100);
        assert_eq!(pad.active_layer, usize::MAX);
    }

    #[test]
    fn add_remove_clear_layers() {
        let mut pad = DrumPad::new("Kick", 36);
        assert_eq!(pad.layers.len(), 0);

        pad.add_layer(SampleLayer::new(vec![1.0; 10], 44100, 0, 127));
        pad.add_layer(SampleLayer::new(vec![0.5; 10], 44100, 0, 127));
        assert_eq!(pad.layers.len(), 2);

        let removed = pad.remove_layer(0);
        assert!(removed.is_some());
        assert_eq!(pad.layers.len(), 1);

        assert!(pad.remove_layer(5).is_none()); // out of bounds

        pad.clear_layers();
        assert_eq!(pad.layers.len(), 0);
    }

    #[test]
    fn sample_layer_matches_velocity() {
        let layer = SampleLayer::new(vec![], 44100, 32, 96);
        assert!(!layer.matches(0));
        assert!(!layer.matches(31));
        assert!(layer.matches(32));
        assert!(layer.matches(64));
        assert!(layer.matches(96));
        assert!(!layer.matches(97));
        assert!(!layer.matches(127));
    }

    // ── 8G.6: comprehensive sample playback tests ──────────────────────

    #[test]
    fn oneshot_completes_and_produces_silence() {
        let mut pad = DrumPad::new("Kick", 36);
        pad.load_sample(vec![0.8; 30], 44100);
        pad.play_mode = PlayMode::OneShot;
        pad.decay = 0.0;
        pad.trigger(127);

        let mut non_silent_count = 0;
        for _ in 0..60 {
            let (l, r) = pad.tick();
            if l.abs() > 1e-6 || r.abs() > 1e-6 {
                non_silent_count += 1;
            }
        }
        assert!(
            !pad.is_playing(),
            "one-shot should stop after exhausting samples"
        );
        assert!(
            (28..=32).contains(&non_silent_count),
            "expected ~30 non-silent frames, got {non_silent_count}"
        );
    }

    #[test]
    fn oneshot_retrigger_restarts_from_beginning() {
        let mut pad = DrumPad::new("Snare", 38);
        let samples: Vec<f32> = (0..50).map(|i| i as f32 / 50.0).collect();
        pad.load_sample(samples, 44100);
        pad.play_mode = PlayMode::OneShot;
        pad.decay = 0.0;
        pad.trigger(127);

        for _ in 0..25 {
            pad.tick();
        }
        assert!(pad.is_playing());

        pad.trigger(100);
        let (l, _) = pad.tick();
        assert!(
            l.abs() < 0.05,
            "retrigger should restart from sample beginning, got {l}"
        );
    }

    #[test]
    fn looped_mode_wraps_and_continues_producing_audio() {
        let mut pad = DrumPad::new("HiHat", 42);
        pad.load_sample(vec![0.5; 20], 44100);
        pad.play_mode = PlayMode::Looped;
        pad.decay = 0.0;
        pad.trigger(127);

        let mut total_nonzero = 0;
        for _ in 0..100 {
            let (l, _) = pad.tick();
            if l.abs() > 1e-6 {
                total_nonzero += 1;
            }
        }

        assert!(
            pad.is_playing(),
            "looped pad should still be playing after 100 ticks"
        );
        assert!(
            total_nonzero > 90,
            "looped pad should produce audio for most ticks, got {total_nonzero}/100"
        );
    }

    #[test]
    fn looped_mode_wraps_position_correctly() {
        let mut pad = DrumPad::new("Loop", 36);
        let len = 10;
        let samples: Vec<f32> = (0..len).map(|i| i as f32 / len as f32).collect();
        pad.load_sample(samples, 44100);
        pad.play_mode = PlayMode::Looped;
        pad.decay = 0.0;
        pad.trigger(127);

        let mut first_cycle = Vec::new();
        for _ in 0..len {
            let (l, _) = pad.tick();
            first_cycle.push(l);
        }

        let mut second_cycle = Vec::new();
        for _ in 0..len {
            let (l, _) = pad.tick();
            second_cycle.push(l);
        }

        for i in 1..len {
            let first_increasing = first_cycle[i] >= first_cycle[i - 1];
            let second_increasing = second_cycle[i] >= second_cycle[i - 1];
            assert_eq!(
                first_increasing, second_increasing,
                "loop cycle shape should repeat at index {i}"
            );
        }
    }

    #[test]
    fn looped_mode_note_off_stops_playback() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(vec![0.5; 50], 44100);
        dm.pads[0].play_mode = PlayMode::Looped;

        dm.note_on(36, 100, 0);
        assert!(dm.pads[0].is_playing());

        dm.note_off(36, 0);
        assert!(!dm.pads[0].is_playing(), "note_off should stop looped pad");
    }

    #[test]
    fn oneshot_mode_note_off_does_not_stop() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(vec![0.5; 100], 44100);
        dm.pads[0].play_mode = PlayMode::OneShot;

        dm.note_on(36, 100, 0);
        assert!(dm.pads[0].is_playing());

        dm.note_off(36, 0);
        assert!(
            dm.pads[0].is_playing(),
            "note_off should NOT stop one-shot pad"
        );
    }

    #[test]
    fn pitch_shifting_half_speed_doubles_playback_time() {
        let sample_len = 100;
        let mut pad = DrumPad::new("Kick", 36);
        pad.load_sample(vec![0.5; sample_len], 44100);
        pad.play_mode = PlayMode::OneShot;
        pad.decay = 0.0;
        pad.pitch = 0.5;

        pad.trigger(127);

        let mut ticks = 0;
        while pad.is_playing() {
            pad.tick();
            ticks += 1;
            if ticks > 500 {
                break;
            }
        }
        assert!(
            (195..=205).contains(&ticks),
            "pitch=0.5 should ~double playback time, got {ticks} ticks for {sample_len} samples"
        );
    }

    #[test]
    fn pitch_shifting_changes_output_frequency() {
        let sample_len = 200;
        let sine: Vec<f32> = (0..sample_len)
            .map(|i| (2.0 * std::f32::consts::PI * 4.0 * i as f32 / sample_len as f32).sin())
            .collect();

        let mut pad_normal = DrumPad::new("Normal", 36);
        pad_normal.load_sample(sine.clone(), 44100);
        pad_normal.decay = 0.0;
        pad_normal.pitch = 1.0;
        pad_normal.trigger(127);

        let mut crossings_normal = 0u32;
        let mut prev = 0.0_f32;
        for _ in 0..100 {
            let (l, _) = pad_normal.tick();
            if l * prev < 0.0 {
                crossings_normal += 1;
            }
            prev = l;
        }

        let mut pad_fast = DrumPad::new("Fast", 36);
        pad_fast.load_sample(sine, 44100);
        pad_fast.decay = 0.0;
        pad_fast.pitch = 2.0;
        pad_fast.trigger(127);

        let mut crossings_fast = 0u32;
        prev = 0.0;
        for _ in 0..100 {
            let (l, _) = pad_fast.tick();
            if l * prev < 0.0 {
                crossings_fast += 1;
            }
            prev = l;
        }

        assert!(
            crossings_fast > crossings_normal,
            "2x pitch should have more zero crossings: normal={crossings_normal}, fast={crossings_fast}"
        );
    }

    #[test]
    fn pitch_shifting_in_looped_mode_wraps_correctly() {
        let mut pad = DrumPad::new("Loop", 36);
        pad.load_sample(vec![0.5; 20], 44100);
        pad.play_mode = PlayMode::Looped;
        pad.decay = 0.0;
        pad.pitch = 3.0;

        pad.trigger(127);

        for _ in 0..100 {
            pad.tick();
        }
        assert!(
            pad.is_playing(),
            "looped pad with pitch=3.0 should keep playing"
        );
    }

    #[test]
    fn velocity_scales_output_proportionally() {
        let sample = vec![1.0; 10];

        let mut pad_full = DrumPad::new("Full", 36);
        pad_full.load_sample(sample.clone(), 44100);
        pad_full.decay = 0.0;
        pad_full.trigger(127);
        let (full_l, _) = pad_full.tick();

        let mut pad_half = DrumPad::new("Half", 36);
        pad_half.load_sample(sample, 44100);
        pad_half.decay = 0.0;
        pad_half.trigger(64);
        let (half_l, _) = pad_half.tick();

        let ratio = half_l / full_l;
        let expected_ratio = 64.0 / 127.0;
        assert!(
            (ratio - expected_ratio).abs() < 0.02,
            "velocity scaling should be proportional: got ratio {ratio}, expected ~{expected_ratio}"
        );
    }

    #[test]
    fn oneshot_via_drum_machine_process_completes() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(vec![0.5; 30], 44100);
        dm.pads[0].play_mode = PlayMode::OneShot;
        dm.pads[0].decay = 0.0;

        dm.note_on(36, 100, 0);
        assert_eq!(dm.active_voices(), 1);

        let mut buf = AudioBuffer::new(2, 64);
        dm.process(&[], &[], &mut buf);

        assert_eq!(
            dm.active_voices(),
            0,
            "one-shot pad should finish after processing past sample end"
        );
    }

    #[test]
    fn looped_via_drum_machine_process_continues() {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].load_sample(vec![0.5; 30], 44100);
        dm.pads[0].play_mode = PlayMode::Looped;
        dm.pads[0].decay = 0.0;

        dm.note_on(36, 100, 0);

        for _ in 0..5 {
            let mut buf = AudioBuffer::new(2, 64);
            dm.process(&[], &[], &mut buf);
        }

        assert_eq!(
            dm.active_voices(),
            1,
            "looped pad should still be playing after many blocks"
        );
    }

    // ── DrumMachineParam enum tests ────────────────────────────────────

    #[test]
    fn drum_machine_param_round_trip() {
        for i in 0..DrumMachineParam::count() {
            let param = DrumMachineParam::try_from(i).expect("valid index");
            assert_eq!(usize::from(param), i);
            assert_eq!(param.index(), i);
        }
    }

    #[test]
    fn drum_machine_param_count_matches_params_vec() {
        let dm = DrumMachine::new(44100.0);
        assert_eq!(
            DrumMachineParam::count(),
            dm.params().len(),
            "DrumMachineParam::count() must match actual params length"
        );
    }

    #[test]
    fn drum_machine_param_all_indices_distinct() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..DrumMachineParam::count() {
            let param = DrumMachineParam::try_from(i).unwrap();
            assert!(
                seen.insert(param.index()),
                "duplicate index {}",
                param.index()
            );
        }
    }

    #[test]
    fn drum_machine_param_out_of_range_returns_err() {
        assert!(DrumMachineParam::try_from(DrumMachineParam::count()).is_err());
        assert!(DrumMachineParam::try_from(usize::MAX).is_err());
    }

    #[test]
    fn drum_machine_get_set_param_typed() {
        let mut dm = DrumMachine::new(44100.0);
        dm.set_param(DrumMachineParam::Volume, 0.6);
        assert!((dm.get_param(DrumMachineParam::Volume) - 0.6).abs() < 1e-6);
    }
}
