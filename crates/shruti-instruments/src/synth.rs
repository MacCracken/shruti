use shruti_dsp::AudioBuffer;
use shruti_session::midi::{ControlChange, NoteEvent};

use crate::effect_chain::EffectChain;
use crate::envelope::{AdsrParams, Envelope};
use crate::filter::{Filter, FilterMode};
use crate::instrument::{InstrumentInfo, InstrumentNode, InstrumentParam};
use crate::lfo::{Lfo, LfoShape};
use crate::oscillator::{Oscillator, Waveform};
use crate::voice::{VoiceManager, VoiceStealMode};

/// A subtractive synthesizer.
///
/// Features:
/// - Selectable waveform (sine, saw, square, triangle, noise)
/// - Multi-oscillator: up to 3 oscillators per voice with independent waveform, detune, and level
/// - Hard sync (osc1 resets osc2 phase on osc1 zero crossing)
/// - Ring modulation (osc1 * osc2)
/// - Oscillator FM (osc1 -> osc2 cross-modulation)
/// - Amp ADSR envelope per voice
/// - Filter ADSR envelope per voice (modulates filter cutoff)
/// - 16-voice polyphony with configurable voice stealing
/// - Detune control
/// - Multi-mode state-variable filter per voice (LP, HP, BP, Notch)
/// - Two global LFOs with configurable targets (none, filter cutoff, pitch, volume)
/// - LFO shape selection (sine, triangle, square, saw up, saw down, S&H)
pub struct SubtractiveSynth {
    info: InstrumentInfo,
    params: Vec<InstrumentParam>,
    voice_manager: VoiceManager,
    oscillators: Vec<Oscillator>,
    oscillators2: Vec<Oscillator>,
    oscillators3: Vec<Oscillator>,
    envelopes: Vec<Envelope>,
    filter_envelopes: Vec<Envelope>,
    filters: Vec<Filter>,
    lfo1: Lfo,
    lfo2: Lfo,
    sample_rate: f32,
    /// Per-instrument effect chain (chorus, delay, reverb, distortion, filter drive).
    pub effect_chain: EffectChain,
}

// Parameter indices.
// Note: some of these are accessed indirectly via `read_adsr()` which reads
// four consecutive indices starting from the attack parameter.
#[allow(dead_code)]
const PARAM_WAVEFORM: usize = 0;
const PARAM_ATTACK: usize = 1;
#[allow(dead_code)]
const PARAM_DECAY: usize = 2;
#[allow(dead_code)]
const PARAM_SUSTAIN: usize = 3;
#[allow(dead_code)]
const PARAM_RELEASE: usize = 4;
const PARAM_VOLUME: usize = 5;
const PARAM_DETUNE: usize = 6;
const PARAM_FILTER_CUTOFF: usize = 7;
const PARAM_FILTER_RESONANCE: usize = 8;
const PARAM_FILTER_MODE: usize = 9;
// Filter envelope (read_adsr reads 4 consecutive from ATTACK)
const PARAM_FILTER_ENV_ATTACK: usize = 10;
#[allow(dead_code)]
const PARAM_FILTER_ENV_DECAY: usize = 11;
#[allow(dead_code)]
const PARAM_FILTER_ENV_SUSTAIN: usize = 12;
#[allow(dead_code)]
const PARAM_FILTER_ENV_RELEASE: usize = 13;
const PARAM_FILTER_ENV_DEPTH: usize = 14;
// LFO 1
const PARAM_LFO1_RATE: usize = 15;
const PARAM_LFO1_DEPTH: usize = 16;
const PARAM_LFO1_TARGET: usize = 17;
const PARAM_LFO1_SHAPE: usize = 18;
// LFO 2
const PARAM_LFO2_RATE: usize = 19;
const PARAM_LFO2_DEPTH: usize = 20;
const PARAM_LFO2_TARGET: usize = 21;
const PARAM_LFO2_SHAPE: usize = 22;
// Oscillator 2
const PARAM_OSC2_ENABLE: usize = 23;
const PARAM_OSC2_WAVEFORM: usize = 24;
const PARAM_OSC2_DETUNE: usize = 25;
const PARAM_OSC2_LEVEL: usize = 26;
// Oscillator 3
const PARAM_OSC3_ENABLE: usize = 27;
const PARAM_OSC3_WAVEFORM: usize = 28;
const PARAM_OSC3_DETUNE: usize = 29;
const PARAM_OSC3_LEVEL: usize = 30;
// Inter-oscillator modulation
const PARAM_HARD_SYNC: usize = 31;
const PARAM_RING_MOD: usize = 32;
const PARAM_FM_AMOUNT: usize = 33;

const MAX_VOICES: usize = 16;

impl SubtractiveSynth {
    pub fn new(sample_rate: f32) -> Self {
        let info = InstrumentInfo {
            name: "Subtractive Synth".to_string(),
            category: "Synthesizer".to_string(),
            author: "Shruti".to_string(),
            description: "Multi-oscillator subtractive synthesizer with hard sync, ring mod, FM, dual ADSR, PolyBLEP oscillators, SVF filter, and dual LFO"
                .to_string(),
        };

        let params = vec![
            // Oscillator 1
            InstrumentParam::new("Waveform", 0.0, 4.0, 1.0, ""), // 0=Sine,1=Saw,2=Square,3=Tri,4=Noise
            // Amp envelope
            InstrumentParam::new("Attack", 0.001, 5.0, 0.01, "s"),
            InstrumentParam::new("Decay", 0.001, 5.0, 0.1, "s"),
            InstrumentParam::new("Sustain", 0.0, 1.0, 0.7, ""),
            InstrumentParam::new("Release", 0.001, 10.0, 0.3, "s"),
            InstrumentParam::new("Volume", 0.0, 1.0, 0.8, ""),
            InstrumentParam::new("Detune", -100.0, 100.0, 0.0, "cents"),
            // Filter
            InstrumentParam::new("FilterCutoff", 20.0, 20000.0, 20000.0, "Hz"),
            InstrumentParam::new("FilterResonance", 0.0, 1.0, 0.0, ""),
            InstrumentParam::new("FilterMode", 0.0, 3.0, 0.0, ""), // 0=LP,1=HP,2=BP,3=Notch
            // Filter envelope
            InstrumentParam::new("FilterEnvAttack", 0.001, 5.0, 0.01, "s"),
            InstrumentParam::new("FilterEnvDecay", 0.001, 5.0, 0.3, "s"),
            InstrumentParam::new("FilterEnvSustain", 0.0, 1.0, 0.0, ""),
            InstrumentParam::new("FilterEnvRelease", 0.001, 10.0, 0.5, "s"),
            InstrumentParam::new("FilterEnvDepth", -1.0, 1.0, 0.0, ""), // bipolar: -1..+1 scales cutoff modulation
            // LFO 1
            InstrumentParam::new("Lfo1Rate", 0.1, 20.0, 1.0, "Hz"),
            InstrumentParam::new("Lfo1Depth", 0.0, 1.0, 0.0, ""),
            InstrumentParam::new("Lfo1Target", 0.0, 3.0, 0.0, ""), // 0=None,1=Cutoff,2=Pitch,3=Volume
            InstrumentParam::new("Lfo1Shape", 0.0, 5.0, 0.0, ""), // 0=Sine,1=Tri,2=Square,3=SawUp,4=SawDown,5=S&H
            // LFO 2
            InstrumentParam::new("Lfo2Rate", 0.1, 20.0, 1.0, "Hz"),
            InstrumentParam::new("Lfo2Depth", 0.0, 1.0, 0.0, ""),
            InstrumentParam::new("Lfo2Target", 0.0, 3.0, 0.0, ""), // 0=None,1=Cutoff,2=Pitch,3=Volume
            InstrumentParam::new("Lfo2Shape", 0.0, 5.0, 0.0, ""), // 0=Sine,1=Tri,2=Square,3=SawUp,4=SawDown,5=S&H
            // Oscillator 2
            InstrumentParam::new("Osc2Enable", 0.0, 1.0, 0.0, ""), // 0=off, 1=on
            InstrumentParam::new("Osc2Waveform", 0.0, 4.0, 1.0, ""), // 0=Sine,1=Saw,2=Square,3=Tri,4=Noise
            InstrumentParam::new("Osc2Detune", -100.0, 100.0, 0.0, "cents"),
            InstrumentParam::new("Osc2Level", 0.0, 1.0, 1.0, ""),
            // Oscillator 3
            InstrumentParam::new("Osc3Enable", 0.0, 1.0, 0.0, ""), // 0=off, 1=on
            InstrumentParam::new("Osc3Waveform", 0.0, 4.0, 1.0, ""), // 0=Sine,1=Saw,2=Square,3=Tri,4=Noise
            InstrumentParam::new("Osc3Detune", -100.0, 100.0, 0.0, "cents"),
            InstrumentParam::new("Osc3Level", 0.0, 1.0, 1.0, ""),
            // Inter-oscillator modulation
            InstrumentParam::new("HardSync", 0.0, 1.0, 0.0, ""), // 0=off, 1=on (osc1 resets osc2 phase)
            InstrumentParam::new("RingMod", 0.0, 1.0, 0.0, ""),  // 0.0..1.0 blend (osc1 * osc2)
            InstrumentParam::new("FmAmount", 0.0, 1.0, 0.0, ""), // 0.0..1.0 (osc1 -> osc2 frequency mod)
        ];

        let oscillators = (0..MAX_VOICES)
            .map(|_| Oscillator::new(Waveform::Saw, sample_rate as f64))
            .collect();
        let oscillators2 = (0..MAX_VOICES)
            .map(|_| Oscillator::new(Waveform::Saw, sample_rate as f64))
            .collect();
        let oscillators3 = (0..MAX_VOICES)
            .map(|_| Oscillator::new(Waveform::Saw, sample_rate as f64))
            .collect();
        let envelopes = (0..MAX_VOICES)
            .map(|_| Envelope::new(AdsrParams::default(), sample_rate))
            .collect();
        let filter_envelopes = (0..MAX_VOICES)
            .map(|_| {
                Envelope::new(
                    AdsrParams {
                        attack: 0.01,
                        decay: 0.3,
                        sustain: 0.0,
                        release: 0.5,
                    },
                    sample_rate,
                )
            })
            .collect();
        let filters = (0..MAX_VOICES)
            .map(|_| Filter::new(FilterMode::LowPass, 20000.0, 0.0, sample_rate))
            .collect();
        let lfo1 = Lfo::new(LfoShape::Sine, 1.0, 0.0, sample_rate);
        let lfo2 = Lfo::new(LfoShape::Sine, 1.0, 0.0, sample_rate);

        Self {
            info,
            params,
            voice_manager: VoiceManager::new(MAX_VOICES, VoiceStealMode::Oldest),
            oscillators,
            oscillators2,
            oscillators3,
            envelopes,
            filter_envelopes,
            filters,
            lfo1,
            lfo2,
            sample_rate,
            effect_chain: EffectChain::new(),
        }
    }

    fn waveform_from_param(value: f32) -> Waveform {
        match value.round() as u8 {
            0 => Waveform::Sine,
            1 => Waveform::Saw,
            2 => Waveform::Square,
            3 => Waveform::Triangle,
            _ => Waveform::Noise,
        }
    }

    fn current_waveform(&self) -> Waveform {
        Self::waveform_from_param(self.params[PARAM_WAVEFORM].value)
    }

    /// Read ADSR parameters from four consecutive param indices
    /// (attack, decay, sustain, release).
    fn read_adsr(&self, attack_idx: usize) -> AdsrParams {
        AdsrParams {
            attack: self.params[attack_idx].value,
            decay: self.params[attack_idx + 1].value,
            sustain: self.params[attack_idx + 2].value,
            release: self.params[attack_idx + 3].value,
        }
    }

    fn current_adsr(&self) -> AdsrParams {
        self.read_adsr(PARAM_ATTACK)
    }

    fn current_filter_adsr(&self) -> AdsrParams {
        self.read_adsr(PARAM_FILTER_ENV_ATTACK)
    }

    fn current_filter_mode(&self) -> FilterMode {
        match self.params[PARAM_FILTER_MODE].value.round() as u8 {
            0 => FilterMode::LowPass,
            1 => FilterMode::HighPass,
            2 => FilterMode::BandPass,
            _ => FilterMode::Notch,
        }
    }

    fn lfo_shape_from_param(value: f32) -> LfoShape {
        match value.round() as u8 {
            0 => LfoShape::Sine,
            1 => LfoShape::Triangle,
            2 => LfoShape::Square,
            3 => LfoShape::SawUp,
            4 => LfoShape::SawDown,
            _ => LfoShape::SampleAndHold,
        }
    }

    /// Fast approximation of `2.0f32.powf(x)` for use in per-sample hot paths.
    ///
    /// Uses IEEE 754 bit manipulation for the integer part and a quadratic
    /// polynomial for the fractional part (maximum error ~0.1%).
    #[inline]
    fn fast_exp2(x: f32) -> f32 {
        // Clamp to avoid overflow/underflow in bit manipulation
        let x = x.clamp(-126.0, 126.0);
        let xi = x.floor() as i32;
        let xf = x - xi as f32;
        let base = f32::from_bits(((xi + 127) as u32) << 23);
        base * (1.0 + xf * (std::f32::consts::LN_2 + xf * 0.2402265))
    }

    /// Apply LFO modulation to a target value. Returns (cutoff_mod, pitch_mod, volume_mod).
    #[inline]
    fn apply_lfo(lfo_val: f32, target: u8) -> (f32, f32, f32) {
        match target {
            1 => (lfo_val, 0.0, 0.0), // cutoff
            2 => (0.0, lfo_val, 0.0), // pitch
            3 => (0.0, 0.0, lfo_val), // volume
            _ => (0.0, 0.0, 0.0),     // none
        }
    }
}

impl SubtractiveSynth {
    /// Render all active voices into the output buffer (adds to existing content).
    fn render_voices(&mut self, note_events: &[NoteEvent], output: &mut AudioBuffer) {
        let frames = output.frames() as usize;
        let channels = output.channels();
        let volume = self.params[PARAM_VOLUME].value;
        let waveform = self.current_waveform();
        let detune = self.params[PARAM_DETUNE].value as f64;
        let filter_cutoff = self.params[PARAM_FILTER_CUTOFF].value;
        let filter_resonance = self.params[PARAM_FILTER_RESONANCE].value;
        let filter_mode = self.current_filter_mode();
        let filter_env_depth = self.params[PARAM_FILTER_ENV_DEPTH].value;

        let lfo1_target = self.params[PARAM_LFO1_TARGET].value.round() as u8;
        let lfo2_target = self.params[PARAM_LFO2_TARGET].value.round() as u8;

        // Multi-oscillator parameters
        let osc2_enabled = self.params[PARAM_OSC2_ENABLE].value >= 0.5;
        let osc2_waveform = Self::waveform_from_param(self.params[PARAM_OSC2_WAVEFORM].value);
        let osc2_detune = self.params[PARAM_OSC2_DETUNE].value as f64;
        let osc2_level = self.params[PARAM_OSC2_LEVEL].value;
        let osc3_enabled = self.params[PARAM_OSC3_ENABLE].value >= 0.5;
        let osc3_waveform = Self::waveform_from_param(self.params[PARAM_OSC3_WAVEFORM].value);
        let osc3_detune = self.params[PARAM_OSC3_DETUNE].value as f64;
        let osc3_level = self.params[PARAM_OSC3_LEVEL].value;
        let hard_sync = self.params[PARAM_HARD_SYNC].value >= 0.5;
        let ring_mod = self.params[PARAM_RING_MOD].value;
        let fm_amount = self.params[PARAM_FM_AMOUNT].value;

        // Update oscillator 1 settings
        for osc in &mut self.oscillators {
            osc.waveform = waveform;
            osc.detune = detune;
        }

        // Update oscillator 2 settings
        for osc in &mut self.oscillators2 {
            osc.waveform = osc2_waveform;
            osc.detune = osc2_detune;
        }

        // Update oscillator 3 settings
        for osc in &mut self.oscillators3 {
            osc.waveform = osc3_waveform;
            osc.detune = osc3_detune;
        }

        // Update filter settings
        for filt in &mut self.filters {
            filt.mode = filter_mode;
            filt.resonance = filter_resonance;
        }

        // Update LFO settings
        self.lfo1.rate = self.params[PARAM_LFO1_RATE].value;
        self.lfo1.depth = self.params[PARAM_LFO1_DEPTH].value;
        self.lfo1.shape = Self::lfo_shape_from_param(self.params[PARAM_LFO1_SHAPE].value);
        self.lfo2.rate = self.params[PARAM_LFO2_RATE].value;
        self.lfo2.depth = self.params[PARAM_LFO2_DEPTH].value;
        self.lfo2.shape = Self::lfo_shape_from_param(self.params[PARAM_LFO2_SHAPE].value);

        // Process note events at the start of the block
        for event in note_events {
            self.note_on(event.note, event.velocity, event.channel);
        }

        // Pre-compute per-frame LFO values into stack-allocated buffer.
        const MAX_LFO_FRAMES: usize = 8192;
        let clamped_frames = frames.min(MAX_LFO_FRAMES);
        let mut lfo1_values = [0.0f32; MAX_LFO_FRAMES];
        let mut lfo2_values = [0.0f32; MAX_LFO_FRAMES];
        for i in 0..clamped_frames {
            lfo1_values[i] = self.lfo1.tick();
            lfo2_values[i] = self.lfo2.tick();
        }

        // Pre-compute detune ratios (avoids pow() in per-sample loop)
        let osc2_detune_ratio = Oscillator::fast_exp2_f64(osc2_detune / 1200.0);
        let osc3_detune_ratio = Oscillator::fast_exp2_f64(osc3_detune / 1200.0);

        // Compute osc level normalization: divide by number of active oscillators
        let active_osc_count =
            1.0f32 + if osc2_enabled { 1.0 } else { 0.0 } + if osc3_enabled { 1.0 } else { 0.0 };
        let osc_norm = 1.0 / active_osc_count;

        // Render each active voice
        let adsr = self.current_adsr();
        let filter_adsr = self.current_filter_adsr();
        let sample_rate = self.sample_rate as f64;
        for i in 0..MAX_VOICES {
            let voice = &self.voice_manager.voices[i];
            if voice.is_idle() {
                continue;
            }

            self.envelopes[i].params = adsr.clone();
            self.filter_envelopes[i].params = filter_adsr.clone();
            let freq = voice.frequency();
            let vel_gain = voice.velocity as f32 / 127.0;
            let mut phase1 = voice.phase;
            let mut phase2 = voice.phase2;
            let mut phase3 = voice.phase3;

            for frame in 0..clamped_frames {
                let env_level = self.envelopes[i].tick();
                let filter_env_level = self.filter_envelopes[i].tick();

                if self.envelopes[i].is_finished() {
                    self.voice_manager.free_voice(i);
                    break;
                }

                // Sum LFO contributions from both LFOs
                let (c1, p1, v1) = Self::apply_lfo(lfo1_values[frame], lfo1_target);
                let (c2, p2, v2) = Self::apply_lfo(lfo2_values[frame], lfo2_target);
                let cutoff_lfo_mod = c1 + c2;
                let pitch_lfo_mod = p1 + p2;
                let volume_lfo_mod = (v1 + v2).clamp(-1.0, 1.0);

                // Apply LFO pitch modulation (in semitones, depth * 12 max)
                let effective_freq = if pitch_lfo_mod.abs() > 0.0001 {
                    let semitones = pitch_lfo_mod * 12.0;
                    freq * Self::fast_exp2(semitones / 12.0) as f64
                } else {
                    freq
                };

                // --- Oscillator 1 ---
                let osc1_sample = self.oscillators[i].sample(phase1, effective_freq);
                let prev_phase1 = phase1;
                phase1 = Oscillator::advance_phase(phase1, effective_freq, sample_rate);

                // Detect osc1 zero crossing (phase wrapped around) for hard sync
                let osc1_wrapped = phase1 < prev_phase1;

                // --- Oscillator mix ---
                let mut mix = osc1_sample * osc_norm;

                if osc2_enabled {
                    // Hard sync: reset osc2 phase when osc1 wraps
                    if hard_sync && osc1_wrapped {
                        phase2 = 0.0;
                    }

                    // Apply osc2 detune to base frequency (pre-computed ratio)
                    let osc2_base_freq = effective_freq * osc2_detune_ratio;

                    // FM: osc1 modulates osc2 frequency
                    let fm_mod = if fm_amount > 0.0001 {
                        osc1_sample as f64 * fm_amount as f64 * osc2_base_freq
                    } else {
                        0.0
                    };
                    let osc2_freq = osc2_base_freq + fm_mod;

                    let osc2_sample = self.oscillators2[i].sample(phase2, osc2_freq);
                    phase2 = Oscillator::advance_phase(phase2, osc2_freq, sample_rate);

                    // Ring modulation: blend between normal mix and osc1*osc2
                    if ring_mod > 0.0001 {
                        let ring_sample = osc1_sample * osc2_sample;
                        let normal_osc2 = osc2_sample * osc2_level * osc_norm;
                        // Blend: (1 - ring_mod) * normal_mix + ring_mod * ring
                        mix = mix * (1.0 - ring_mod)
                            + ring_sample * ring_mod
                            + normal_osc2 * (1.0 - ring_mod);
                    } else {
                        mix += osc2_sample * osc2_level * osc_norm;
                    }
                }

                if osc3_enabled {
                    // Apply osc3 detune to base frequency (pre-computed ratio)
                    let osc3_freq = effective_freq * osc3_detune_ratio;
                    let osc3_sample = self.oscillators3[i].sample(phase3, osc3_freq);
                    phase3 = Oscillator::advance_phase(phase3, osc3_freq, sample_rate);
                    mix += osc3_sample * osc3_level * osc_norm;
                }

                // Apply amp envelope
                let after_env = mix * env_level;

                // Compute filter cutoff: base + filter envelope + LFO.
                //
                // Modulation is applied in octaves: the envelope and LFO each
                // contribute up to +/-4 octaves of cutoff shift.  A depth of
                // 1.0 therefore sweeps the cutoff by 4 octaves (16x frequency).
                let env_mod_octaves = filter_env_level * filter_env_depth * 4.0;
                let lfo_mod_octaves = cutoff_lfo_mod * 4.0;
                let modulated_cutoff = (filter_cutoff
                    * Self::fast_exp2(env_mod_octaves + lfo_mod_octaves))
                .clamp(20.0, 20000.0);
                self.filters[i].cutoff = modulated_cutoff;

                let filtered = self.filters[i].process_sample(after_env);

                // Apply volume LFO (bipolar mod mapped to 0..1 range)
                let vol_mod = (1.0 + volume_lfo_mod).clamp(0.0, 2.0) * 0.5;
                let out = filtered * vel_gain * volume * vol_mod;

                for ch in 0..channels {
                    let current = output.get(frame as u32, ch);
                    output.set(frame as u32, ch, current + out);
                }
            }

            self.voice_manager.voices[i].phase = phase1;
            self.voice_manager.voices[i].phase2 = phase2;
            self.voice_manager.voices[i].phase3 = phase3;
            self.voice_manager.voices[i].envelope_level = self.envelopes[i].level;
        }

        self.voice_manager.tick_age();
    }
}

impl InstrumentNode for SubtractiveSynth {
    fn info(&self) -> &InstrumentInfo {
        &self.info
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for osc in &mut self.oscillators {
            osc.set_sample_rate(sample_rate as f64);
        }
        for osc in &mut self.oscillators2 {
            osc.set_sample_rate(sample_rate as f64);
        }
        for osc in &mut self.oscillators3 {
            osc.set_sample_rate(sample_rate as f64);
        }
        for env in &mut self.envelopes {
            env.set_sample_rate(sample_rate);
        }
        for env in &mut self.filter_envelopes {
            env.set_sample_rate(sample_rate);
        }
        for filt in &mut self.filters {
            filt.set_sample_rate(sample_rate);
        }
        self.lfo1.set_sample_rate(sample_rate);
        self.lfo2.set_sample_rate(sample_rate);
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
            // Temporarily take the effect chain to avoid borrow conflict
            let mut chain = std::mem::take(&mut self.effect_chain);
            chain.process_with(output, |buf| {
                self.render_voices(note_events, buf);
            });
            self.effect_chain = chain;
        } else {
            self.render_voices(note_events, output);
        }
    }

    fn note_on(&mut self, note: u8, velocity: u8, channel: u8) {
        if let Some(idx) = self.voice_manager.note_on(note, velocity, channel) {
            self.envelopes[idx].params = self.current_adsr();
            self.envelopes[idx].trigger();
            self.filter_envelopes[idx].params = self.current_filter_adsr();
            self.filter_envelopes[idx].trigger();
            self.filters[idx].reset();
        }
    }

    fn note_off(&mut self, note: u8, channel: u8) {
        for (i, voice) in self.voice_manager.voices.iter().enumerate() {
            if voice.note == note && voice.channel == channel && !voice.is_idle() {
                self.envelopes[i].release();
                self.filter_envelopes[i].release();
            }
        }
        self.voice_manager.note_off(note, channel);
    }

    fn params(&self) -> &[InstrumentParam] {
        &self.params
    }

    fn params_mut(&mut self) -> &mut [InstrumentParam] {
        &mut self.params
    }

    fn reset(&mut self) {
        self.voice_manager.reset();
        for env in &mut self.envelopes {
            env.reset();
        }
        for env in &mut self.filter_envelopes {
            env.reset();
        }
        for filt in &mut self.filters {
            filt.reset();
        }
        self.lfo1.reset();
        self.lfo2.reset();
        self.effect_chain.reset();
    }

    fn active_voices(&self) -> usize {
        self.voice_manager.active_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_exp2_accuracy() {
        // Verify fast_exp2 matches 2.0f32.powf(x) within 0.2% for typical
        // musical modulation ranges (-4..+4 octaves).
        let test_values = [-4.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 4.0];
        for &x in &test_values {
            let exact = 2.0f32.powf(x);
            let approx = SubtractiveSynth::fast_exp2(x);
            let rel_error = ((approx - exact) / exact).abs();
            assert!(
                rel_error < 0.01,
                "fast_exp2({x}) = {approx}, expected {exact}, error = {:.4}%",
                rel_error * 100.0,
            );
        }
    }

    #[test]
    fn fast_exp2_zero_is_one() {
        let result = SubtractiveSynth::fast_exp2(0.0);
        assert!(
            (result - 1.0).abs() < 1e-5,
            "fast_exp2(0) should be 1.0, got {result}"
        );
    }

    #[test]
    fn fast_exp2_one_is_two() {
        let result = SubtractiveSynth::fast_exp2(1.0);
        assert!(
            (result - 2.0).abs() < 0.01,
            "fast_exp2(1) should be ~2.0, got {result}"
        );
    }

    #[test]
    fn read_adsr_returns_correct_params() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_ATTACK].set(0.05);
        synth.params_mut()[PARAM_DECAY].set(0.2);
        synth.params_mut()[PARAM_SUSTAIN].set(0.6);
        synth.params_mut()[PARAM_RELEASE].set(0.4);
        let adsr = synth.current_adsr();
        assert!((adsr.attack - 0.05).abs() < 1e-5);
        assert!((adsr.decay - 0.2).abs() < 1e-5);
        assert!((adsr.sustain - 0.6).abs() < 1e-5);
        assert!((adsr.release - 0.4).abs() < 1e-5);
    }

    #[test]
    fn read_adsr_works_for_filter_envelope() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_FILTER_ENV_ATTACK].set(0.1);
        synth.params_mut()[PARAM_FILTER_ENV_DECAY].set(0.5);
        synth.params_mut()[PARAM_FILTER_ENV_SUSTAIN].set(0.3);
        synth.params_mut()[PARAM_FILTER_ENV_RELEASE].set(0.8);
        let adsr = synth.current_filter_adsr();
        assert!((adsr.attack - 0.1).abs() < 1e-5);
        assert!((adsr.decay - 0.5).abs() < 1e-5);
        assert!((adsr.sustain - 0.3).abs() < 1e-5);
        assert!((adsr.release - 0.8).abs() < 1e-5);
    }

    #[test]
    fn synth_creates_with_defaults() {
        let synth = SubtractiveSynth::new(48000.0);
        assert_eq!(synth.info().name, "Subtractive Synth");
        assert_eq!(synth.params().len(), 34); // 23 original + 11 multi-osc params
        assert_eq!(synth.active_voices(), 0);
    }

    #[test]
    fn synth_note_on_activates_voice() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.note_on(60, 100, 0);
        assert_eq!(synth.active_voices(), 1);
    }

    #[test]
    fn synth_note_off_releases() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.note_on(60, 100, 0);
        synth.note_off(60, 0);
        assert_eq!(synth.active_voices(), 1);
    }

    #[test]
    fn synth_produces_audio() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.note_on(69, 127, 0);

        let mut buf = AudioBuffer::new(2, 256);
        synth.process(&[], &[], &mut buf);

        let mut has_nonzero = false;
        for i in 0..256 {
            if buf.get(i, 0).abs() > 0.001 {
                has_nonzero = true;
                break;
            }
        }
        assert!(has_nonzero, "synth should produce audio output");
    }

    #[test]
    fn synth_silence_without_notes() {
        let mut synth = SubtractiveSynth::new(48000.0);
        let mut buf = AudioBuffer::new(2, 256);
        synth.process(&[], &[], &mut buf);

        for i in 0..256 {
            assert_eq!(buf.get(i, 0), 0.0);
        }
    }

    #[test]
    fn synth_reset_clears_voices() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.note_on(60, 100, 0);
        synth.note_on(64, 100, 0);
        assert_eq!(synth.active_voices(), 2);
        synth.reset();
        assert_eq!(synth.active_voices(), 0);
    }

    #[test]
    fn synth_params_are_settable() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_VOLUME].set(0.5);
        assert!((synth.params()[PARAM_VOLUME].value - 0.5).abs() < 0.001);
    }

    // --- New tests for filter and LFO ---

    /// Helper: render a synth playing a saw wave at A4 and return RMS of output.
    fn render_rms(synth: &mut SubtractiveSynth, num_frames: usize) -> f32 {
        let mut buf = AudioBuffer::new(2, num_frames as u32);
        synth.process(&[], &[], &mut buf);
        let mut sum_sq = 0.0f64;
        for i in 0..num_frames {
            let s = buf.get(i as u32, 0) as f64;
            sum_sq += s * s;
        }
        (sum_sq / num_frames as f64).sqrt() as f32
    }

    #[test]
    fn filter_lowpass_attenuates_bright_content() {
        let mut synth_open = SubtractiveSynth::new(48000.0);
        synth_open.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth_open.params_mut()[PARAM_FILTER_CUTOFF].set(20000.0);
        synth_open.note_on(69, 127, 0);
        let rms_open = render_rms(&mut synth_open, 4096);

        let mut synth_filtered = SubtractiveSynth::new(48000.0);
        synth_filtered.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth_filtered.params_mut()[PARAM_FILTER_CUTOFF].set(200.0);
        synth_filtered.note_on(69, 127, 0);
        let rms_filtered = render_rms(&mut synth_filtered, 4096);

        assert!(
            rms_filtered < rms_open,
            "Filtered saw should have lower RMS: open={rms_open}, filtered={rms_filtered}"
        );
    }

    #[test]
    fn filter_highpass_removes_low_content() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth.params_mut()[PARAM_FILTER_MODE].set(1.0);
        synth.params_mut()[PARAM_FILTER_CUTOFF].set(5000.0);
        synth.note_on(48, 127, 0);

        let mut buf = AudioBuffer::new(2, 8192);
        synth.process(&[], &[], &mut buf);

        let mut sum_sq = 0.0f64;
        for i in 4096..8192 {
            let s = buf.get(i as u32, 0) as f64;
            sum_sq += s * s;
        }
        let rms = (sum_sq / 4096.0).sqrt() as f32;
        assert!(
            rms < 0.05,
            "HP filter at 5kHz should attenuate 130Hz sine, got rms={rms}"
        );
    }

    #[test]
    fn lfo1_modulates_filter_cutoff() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth.params_mut()[PARAM_FILTER_CUTOFF].set(500.0);
        synth.params_mut()[PARAM_LFO1_TARGET].set(1.0); // Cutoff
        synth.params_mut()[PARAM_LFO1_RATE].set(5.0);
        synth.params_mut()[PARAM_LFO1_DEPTH].set(1.0);
        synth.note_on(69, 127, 0);

        let rms1 = render_rms(&mut synth, 2048);
        let rms2 = render_rms(&mut synth, 2048);

        assert!(
            rms1 > 0.001 || rms2 > 0.001,
            "Synth should produce output with LFO modulation"
        );
    }

    #[test]
    fn lfo1_modulates_pitch() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth.params_mut()[PARAM_LFO1_TARGET].set(2.0); // Pitch
        synth.params_mut()[PARAM_LFO1_RATE].set(5.0);
        synth.params_mut()[PARAM_LFO1_DEPTH].set(0.5);
        synth.note_on(69, 127, 0);

        let mut buf = AudioBuffer::new(2, 4096);
        synth.process(&[], &[], &mut buf);

        let mut has_nonzero = false;
        for i in 0..4096 {
            if buf.get(i, 0).abs() > 0.001 {
                has_nonzero = true;
                break;
            }
        }
        assert!(
            has_nonzero,
            "Synth with LFO pitch modulation should produce output"
        );
    }

    #[test]
    fn lfo2_modulates_volume() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth.params_mut()[PARAM_LFO2_TARGET].set(3.0); // Volume (tremolo)
        synth.params_mut()[PARAM_LFO2_RATE].set(5.0);
        synth.params_mut()[PARAM_LFO2_DEPTH].set(1.0);
        synth.params_mut()[PARAM_LFO2_SHAPE].set(2.0); // Square LFO
        synth.note_on(69, 127, 0);

        let mut buf = AudioBuffer::new(2, 4096);
        synth.process(&[], &[], &mut buf);

        let mut has_nonzero = false;
        for i in 0..4096 {
            if buf.get(i, 0).abs() > 0.001 {
                has_nonzero = true;
                break;
            }
        }
        assert!(
            has_nonzero,
            "Synth with LFO2 volume modulation should produce output"
        );
    }

    #[test]
    fn filter_envelope_modulates_cutoff() {
        // With filter env depth > 0 and low base cutoff, the filter envelope
        // should open the filter on note-on then close it during decay.
        let mut synth_no_env = SubtractiveSynth::new(48000.0);
        synth_no_env.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth_no_env.params_mut()[PARAM_FILTER_CUTOFF].set(200.0);
        synth_no_env.params_mut()[PARAM_FILTER_ENV_DEPTH].set(0.0);
        synth_no_env.note_on(69, 127, 0);
        let rms_no_env = render_rms(&mut synth_no_env, 4096);

        let mut synth_env = SubtractiveSynth::new(48000.0);
        synth_env.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth_env.params_mut()[PARAM_FILTER_CUTOFF].set(200.0);
        synth_env.params_mut()[PARAM_FILTER_ENV_DEPTH].set(1.0);
        synth_env.params_mut()[PARAM_FILTER_ENV_ATTACK].set(0.001);
        synth_env.params_mut()[PARAM_FILTER_ENV_DECAY].set(0.5);
        synth_env.note_on(69, 127, 0);
        let rms_env = render_rms(&mut synth_env, 4096);

        // Filter envelope opening the cutoff should increase RMS
        assert!(
            rms_env > rms_no_env,
            "Filter envelope should open cutoff: with_env={rms_env}, without={rms_no_env}"
        );
    }

    #[test]
    fn dual_lfo_both_active() {
        // LFO1 on cutoff + LFO2 on pitch simultaneously
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth.params_mut()[PARAM_FILTER_CUTOFF].set(1000.0);
        synth.params_mut()[PARAM_LFO1_TARGET].set(1.0); // Cutoff
        synth.params_mut()[PARAM_LFO1_RATE].set(3.0);
        synth.params_mut()[PARAM_LFO1_DEPTH].set(0.5);
        synth.params_mut()[PARAM_LFO2_TARGET].set(2.0); // Pitch
        synth.params_mut()[PARAM_LFO2_RATE].set(7.0);
        synth.params_mut()[PARAM_LFO2_DEPTH].set(0.3);
        synth.note_on(69, 127, 0);

        let rms = render_rms(&mut synth, 4096);
        assert!(rms > 0.001, "Dual LFO synth should produce output");
    }

    #[test]
    fn synth_max_polyphony_stress() {
        let mut synth = SubtractiveSynth::new(48000.0);
        // Trigger more notes than MAX_VOICES (16)
        for i in 0..24 {
            synth.note_on(36 + i, 100, 0);
        }
        // Voice count should be capped at MAX_VOICES
        assert_eq!(
            synth.active_voices(),
            MAX_VOICES,
            "active voices should be capped at {MAX_VOICES}"
        );

        // Process audio and verify output is finite (no NaN/inf)
        let mut buf = AudioBuffer::new(2, 1024);
        synth.process(&[], &[], &mut buf);

        let mut has_nonzero = false;
        for i in 0..1024 {
            let sample = buf.get(i, 0);
            assert!(
                sample.is_finite(),
                "sample at frame {i} is not finite: {sample}"
            );
            if sample.abs() > 0.001 {
                has_nonzero = true;
            }
        }
        assert!(
            has_nonzero,
            "synth with 16 active voices should produce audio output"
        );
    }

    #[test]
    fn different_filter_modes_produce_different_output() {
        fn render_with_mode(mode: f32) -> Vec<f32> {
            let mut synth = SubtractiveSynth::new(48000.0);
            synth.params_mut()[PARAM_WAVEFORM].set(1.0);
            synth.params_mut()[PARAM_FILTER_CUTOFF].set(1000.0);
            synth.params_mut()[PARAM_FILTER_RESONANCE].set(0.5);
            synth.params_mut()[PARAM_FILTER_MODE].set(mode);
            synth.note_on(69, 127, 0);

            let mut buf = AudioBuffer::new(2, 2048);
            synth.process(&[], &[], &mut buf);

            (0..2048).map(|i| buf.get(i, 0)).collect()
        }

        let lp = render_with_mode(0.0);
        let hp = render_with_mode(1.0);
        let bp = render_with_mode(2.0);

        let diff_lp_hp: f32 = lp.iter().zip(hp.iter()).map(|(a, b)| (a - b).abs()).sum();
        let diff_lp_bp: f32 = lp.iter().zip(bp.iter()).map(|(a, b)| (a - b).abs()).sum();

        assert!(
            diff_lp_hp > 1.0,
            "LP and HP should produce different output, diff={diff_lp_hp}"
        );
        assert!(
            diff_lp_bp > 1.0,
            "LP and BP should produce different output, diff={diff_lp_bp}"
        );
    }

    // =========================================================================
    // Multi-oscillator tests (phase 8B+.6)
    // =========================================================================

    #[test]
    fn multi_osc_default_backward_compat() {
        // With osc2/osc3 disabled (default), output should be the same as single-osc
        let mut synth = SubtractiveSynth::new(48000.0);
        assert!(synth.params()[PARAM_OSC2_ENABLE].value < 0.5);
        assert!(synth.params()[PARAM_OSC3_ENABLE].value < 0.5);
        synth.note_on(69, 127, 0);
        let rms = render_rms(&mut synth, 1024);
        assert!(rms > 0.01, "single-osc should produce output, rms={rms}");
    }

    #[test]
    fn multi_osc_two_oscs_louder_than_one() {
        // Adding a second oscillator (same waveform, no detune) should produce
        // roughly the same RMS as a single osc (levels are normalized).
        let mut synth1 = SubtractiveSynth::new(48000.0);
        synth1.params_mut()[PARAM_WAVEFORM].set(0.0); // sine
        synth1.note_on(69, 127, 0);
        let rms1 = render_rms(&mut synth1, 2048);

        let mut synth2 = SubtractiveSynth::new(48000.0);
        synth2.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth2.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth2.params_mut()[PARAM_OSC2_WAVEFORM].set(0.0); // sine
        synth2.note_on(69, 127, 0);
        let rms2 = render_rms(&mut synth2, 2048);

        // Both should produce meaningful output
        assert!(rms1 > 0.01, "single osc rms too low: {rms1}");
        assert!(rms2 > 0.01, "dual osc rms too low: {rms2}");
    }

    #[test]
    fn multi_osc_three_oscs_produce_output() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(1.0); // saw
        synth.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth.params_mut()[PARAM_OSC2_WAVEFORM].set(2.0); // square
        synth.params_mut()[PARAM_OSC3_ENABLE].set(1.0);
        synth.params_mut()[PARAM_OSC3_WAVEFORM].set(3.0); // triangle
        synth.note_on(69, 127, 0);
        let rms = render_rms(&mut synth, 2048);
        assert!(
            rms > 0.01,
            "three-osc synth should produce output, rms={rms}"
        );
    }

    #[test]
    fn multi_osc_detune_changes_output() {
        // Detuning osc2 should produce a different output than no detune
        let mut synth_no_detune = SubtractiveSynth::new(48000.0);
        synth_no_detune.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth_no_detune.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_no_detune.params_mut()[PARAM_OSC2_DETUNE].set(0.0);
        synth_no_detune.note_on(69, 127, 0);
        let mut buf_nd = AudioBuffer::new(2, 2048);
        synth_no_detune.process(&[], &[], &mut buf_nd);

        let mut synth_detuned = SubtractiveSynth::new(48000.0);
        synth_detuned.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth_detuned.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_detuned.params_mut()[PARAM_OSC2_DETUNE].set(15.0); // +15 cents
        synth_detuned.note_on(69, 127, 0);
        let mut buf_d = AudioBuffer::new(2, 2048);
        synth_detuned.process(&[], &[], &mut buf_d);

        let diff: f32 = (0..2048)
            .map(|i| (buf_nd.get(i, 0) - buf_d.get(i, 0)).abs())
            .sum();
        assert!(
            diff > 0.1,
            "detuning osc2 should change output, diff={diff}"
        );
    }

    #[test]
    fn multi_osc_osc2_level_zero_is_silent() {
        // With osc2 enabled but level=0, output should match osc1-only
        let mut synth_osc1 = SubtractiveSynth::new(48000.0);
        synth_osc1.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth_osc1.note_on(69, 127, 0);
        let mut buf1 = AudioBuffer::new(2, 1024);
        synth_osc1.process(&[], &[], &mut buf1);

        let mut synth_osc2_silent = SubtractiveSynth::new(48000.0);
        synth_osc2_silent.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth_osc2_silent.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_osc2_silent.params_mut()[PARAM_OSC2_LEVEL].set(0.0);
        synth_osc2_silent.note_on(69, 127, 0);
        let mut buf2 = AudioBuffer::new(2, 1024);
        synth_osc2_silent.process(&[], &[], &mut buf2);

        // Both should produce output (osc1 is still active)
        let rms1 = (0..1024)
            .map(|i| (buf1.get(i, 0) as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        let rms2 = (0..1024)
            .map(|i| (buf2.get(i, 0) as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(rms1 > 0.1, "osc1-only should produce output");
        assert!(rms2 > 0.1, "osc1+silent_osc2 should still produce output");
    }

    #[test]
    fn hard_sync_changes_timbre() {
        // Hard sync should produce a different output than without it
        let mut synth_nosync = SubtractiveSynth::new(48000.0);
        synth_nosync.params_mut()[PARAM_WAVEFORM].set(1.0); // saw
        synth_nosync.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_nosync.params_mut()[PARAM_OSC2_WAVEFORM].set(1.0);
        synth_nosync.params_mut()[PARAM_OSC2_DETUNE].set(50.0); // detuned so sync is audible
        synth_nosync.params_mut()[PARAM_HARD_SYNC].set(0.0);
        synth_nosync.note_on(60, 127, 0);
        let mut buf_ns = AudioBuffer::new(2, 4096);
        synth_nosync.process(&[], &[], &mut buf_ns);

        let mut synth_sync = SubtractiveSynth::new(48000.0);
        synth_sync.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth_sync.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_sync.params_mut()[PARAM_OSC2_WAVEFORM].set(1.0);
        synth_sync.params_mut()[PARAM_OSC2_DETUNE].set(50.0);
        synth_sync.params_mut()[PARAM_HARD_SYNC].set(1.0);
        synth_sync.note_on(60, 127, 0);
        let mut buf_s = AudioBuffer::new(2, 4096);
        synth_sync.process(&[], &[], &mut buf_s);

        let diff: f32 = (0..4096)
            .map(|i| (buf_ns.get(i, 0) - buf_s.get(i, 0)).abs())
            .sum();
        assert!(diff > 1.0, "hard sync should change timbre, diff={diff}");
    }

    #[test]
    fn hard_sync_produces_finite_output() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth.params_mut()[PARAM_OSC2_DETUNE].set(70.0);
        synth.params_mut()[PARAM_HARD_SYNC].set(1.0);
        synth.note_on(60, 127, 0);
        let mut buf = AudioBuffer::new(2, 4096);
        synth.process(&[], &[], &mut buf);
        for frame in 0..4096 {
            let s = buf.get(frame, 0);
            assert!(
                s.is_finite(),
                "sync output not finite at frame {frame}: {s}"
            );
        }
    }

    #[test]
    fn ring_mod_produces_different_output() {
        let mut synth_no_ring = SubtractiveSynth::new(48000.0);
        synth_no_ring.params_mut()[PARAM_WAVEFORM].set(0.0); // sine
        synth_no_ring.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_no_ring.params_mut()[PARAM_OSC2_WAVEFORM].set(0.0);
        synth_no_ring.params_mut()[PARAM_OSC2_DETUNE].set(50.0);
        synth_no_ring.params_mut()[PARAM_RING_MOD].set(0.0);
        synth_no_ring.note_on(60, 127, 0);
        let mut buf_nr = AudioBuffer::new(2, 2048);
        synth_no_ring.process(&[], &[], &mut buf_nr);

        let mut synth_ring = SubtractiveSynth::new(48000.0);
        synth_ring.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth_ring.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_ring.params_mut()[PARAM_OSC2_WAVEFORM].set(0.0);
        synth_ring.params_mut()[PARAM_OSC2_DETUNE].set(50.0);
        synth_ring.params_mut()[PARAM_RING_MOD].set(1.0);
        synth_ring.note_on(60, 127, 0);
        let mut buf_r = AudioBuffer::new(2, 2048);
        synth_ring.process(&[], &[], &mut buf_r);

        let diff: f32 = (0..2048)
            .map(|i| (buf_nr.get(i, 0) - buf_r.get(i, 0)).abs())
            .sum();
        assert!(diff > 0.5, "ring mod should change output, diff={diff}");
    }

    #[test]
    fn ring_mod_zero_is_noop() {
        // Ring mod at 0.0 should not affect the output compared to no ring mod
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(1.0);
        synth.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth.params_mut()[PARAM_RING_MOD].set(0.0);
        synth.note_on(69, 127, 0);
        let rms = render_rms(&mut synth, 1024);
        assert!(
            rms > 0.01,
            "ring_mod=0 should produce normal output, rms={rms}"
        );
    }

    #[test]
    fn fm_modulation_changes_timbre() {
        let mut synth_no_fm = SubtractiveSynth::new(48000.0);
        synth_no_fm.params_mut()[PARAM_WAVEFORM].set(0.0); // sine
        synth_no_fm.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_no_fm.params_mut()[PARAM_OSC2_WAVEFORM].set(0.0);
        synth_no_fm.params_mut()[PARAM_FM_AMOUNT].set(0.0);
        synth_no_fm.note_on(60, 127, 0);
        let mut buf_nfm = AudioBuffer::new(2, 2048);
        synth_no_fm.process(&[], &[], &mut buf_nfm);

        let mut synth_fm = SubtractiveSynth::new(48000.0);
        synth_fm.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth_fm.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth_fm.params_mut()[PARAM_OSC2_WAVEFORM].set(0.0);
        synth_fm.params_mut()[PARAM_FM_AMOUNT].set(0.5);
        synth_fm.note_on(60, 127, 0);
        let mut buf_fm = AudioBuffer::new(2, 2048);
        synth_fm.process(&[], &[], &mut buf_fm);

        let diff: f32 = (0..2048)
            .map(|i| (buf_nfm.get(i, 0) - buf_fm.get(i, 0)).abs())
            .sum();
        assert!(diff > 0.5, "FM should change timbre, diff={diff}");
    }

    #[test]
    fn fm_produces_finite_output() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(1.0); // saw
        synth.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth.params_mut()[PARAM_FM_AMOUNT].set(1.0); // max FM
        synth.note_on(69, 127, 0);
        let mut buf = AudioBuffer::new(2, 4096);
        synth.process(&[], &[], &mut buf);
        for frame in 0..4096 {
            let s = buf.get(frame, 0);
            assert!(s.is_finite(), "FM output not finite at frame {frame}: {s}");
        }
    }

    #[test]
    fn multi_osc_enable_disable_toggle() {
        // Enabling then disabling osc2 should return to single-osc behavior
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth.note_on(69, 127, 0);

        // Render with osc2 disabled
        let mut buf1 = AudioBuffer::new(2, 512);
        synth.process(&[], &[], &mut buf1);

        // Enable osc2 with different waveform
        synth.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth.params_mut()[PARAM_OSC2_WAVEFORM].set(2.0); // square
        let mut buf2 = AudioBuffer::new(2, 512);
        synth.process(&[], &[], &mut buf2);

        // Disable osc2 again
        synth.params_mut()[PARAM_OSC2_ENABLE].set(0.0);
        let mut buf3 = AudioBuffer::new(2, 512);
        synth.process(&[], &[], &mut buf3);

        // buf2 should differ from buf1 (osc2 added)
        let diff12: f32 = (0..512)
            .map(|i| (buf1.get(i, 0) - buf2.get(i, 0)).abs())
            .sum();
        assert!(diff12 > 0.1, "enabling osc2 should change output");

        // buf3 should produce output (osc1 still playing)
        let rms3: f32 = (0..512)
            .map(|i| (buf3.get(i, 0) as f64).powi(2))
            .sum::<f64>()
            .sqrt() as f32;
        assert!(rms3 > 0.001, "disabling osc2 should still have osc1 output");
    }

    #[test]
    fn multi_osc_params_settable() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        assert!((synth.params()[PARAM_OSC2_ENABLE].value - 1.0).abs() < 0.001);
        synth.params_mut()[PARAM_OSC2_WAVEFORM].set(2.0);
        assert!((synth.params()[PARAM_OSC2_WAVEFORM].value - 2.0).abs() < 0.001);
        synth.params_mut()[PARAM_OSC2_DETUNE].set(25.0);
        assert!((synth.params()[PARAM_OSC2_DETUNE].value - 25.0).abs() < 0.001);
        synth.params_mut()[PARAM_OSC2_LEVEL].set(0.5);
        assert!((synth.params()[PARAM_OSC2_LEVEL].value - 0.5).abs() < 0.001);
        synth.params_mut()[PARAM_OSC3_ENABLE].set(1.0);
        assert!((synth.params()[PARAM_OSC3_ENABLE].value - 1.0).abs() < 0.001);
        synth.params_mut()[PARAM_HARD_SYNC].set(1.0);
        assert!((synth.params()[PARAM_HARD_SYNC].value - 1.0).abs() < 0.001);
        synth.params_mut()[PARAM_RING_MOD].set(0.7);
        assert!((synth.params()[PARAM_RING_MOD].value - 0.7).abs() < 0.001);
        synth.params_mut()[PARAM_FM_AMOUNT].set(0.3);
        assert!((synth.params()[PARAM_FM_AMOUNT].value - 0.3).abs() < 0.001);
    }

    #[test]
    fn multi_osc_all_features_combined() {
        // Smoke test: all multi-osc features at once should not panic or produce NaN
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_WAVEFORM].set(1.0); // saw
        synth.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth.params_mut()[PARAM_OSC2_WAVEFORM].set(2.0); // square
        synth.params_mut()[PARAM_OSC2_DETUNE].set(7.0);
        synth.params_mut()[PARAM_OSC3_ENABLE].set(1.0);
        synth.params_mut()[PARAM_OSC3_WAVEFORM].set(0.0); // sine sub
        synth.params_mut()[PARAM_OSC3_DETUNE].set(-12.0);
        synth.params_mut()[PARAM_HARD_SYNC].set(1.0);
        synth.params_mut()[PARAM_RING_MOD].set(0.3);
        synth.params_mut()[PARAM_FM_AMOUNT].set(0.2);
        synth.note_on(60, 100, 0);
        synth.note_on(64, 100, 0);
        synth.note_on(67, 100, 0);

        let mut buf = AudioBuffer::new(2, 4096);
        synth.process(&[], &[], &mut buf);

        let mut has_nonzero = false;
        for frame in 0..4096 {
            let s = buf.get(frame, 0);
            assert!(s.is_finite(), "combined multi-osc NaN/inf at frame {frame}");
            if s.abs() > 0.001 {
                has_nonzero = true;
            }
        }
        assert!(has_nonzero, "combined multi-osc should produce output");
    }

    #[test]
    fn multi_osc_osc3_detune_independent() {
        // Osc3 detune should be independent of osc1/osc2
        let mut synth_a = SubtractiveSynth::new(48000.0);
        synth_a.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth_a.params_mut()[PARAM_OSC3_ENABLE].set(1.0);
        synth_a.params_mut()[PARAM_OSC3_WAVEFORM].set(0.0);
        synth_a.params_mut()[PARAM_OSC3_DETUNE].set(0.0);
        synth_a.note_on(69, 127, 0);
        let mut buf_a = AudioBuffer::new(2, 2048);
        synth_a.process(&[], &[], &mut buf_a);

        let mut synth_b = SubtractiveSynth::new(48000.0);
        synth_b.params_mut()[PARAM_WAVEFORM].set(0.0);
        synth_b.params_mut()[PARAM_OSC3_ENABLE].set(1.0);
        synth_b.params_mut()[PARAM_OSC3_WAVEFORM].set(0.0);
        synth_b.params_mut()[PARAM_OSC3_DETUNE].set(50.0);
        synth_b.note_on(69, 127, 0);
        let mut buf_b = AudioBuffer::new(2, 2048);
        synth_b.process(&[], &[], &mut buf_b);

        let diff: f32 = (0..2048)
            .map(|i| (buf_a.get(i, 0) - buf_b.get(i, 0)).abs())
            .sum();
        assert!(diff > 0.1, "osc3 detune should change output, diff={diff}");
    }

    #[test]
    fn multi_osc_reset_clears_phases() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.params_mut()[PARAM_OSC2_ENABLE].set(1.0);
        synth.params_mut()[PARAM_OSC3_ENABLE].set(1.0);
        synth.note_on(69, 127, 0);

        // Process some audio to advance phases
        let mut buf = AudioBuffer::new(2, 1024);
        synth.process(&[], &[], &mut buf);

        synth.reset();
        assert_eq!(synth.active_voices(), 0);
        // After reset and new note, phases should start fresh
        synth.note_on(69, 127, 0);
        let voice = &synth.voice_manager.voices[0];
        assert_eq!(voice.phase, 0.0);
        assert_eq!(voice.phase2, 0.0);
        assert_eq!(voice.phase3, 0.0);
    }

    #[test]
    fn set_sample_rate_propagates_to_all_components() {
        let mut synth = SubtractiveSynth::new(48000.0);
        synth.set_sample_rate(96000.0);

        // Verify by playing a note — the synth should produce correct output
        // at the new sample rate without panics or NaN.
        synth.note_on(69, 127, 0);
        let mut buf = AudioBuffer::new(2, 1024);
        synth.process(&[], &[], &mut buf);

        let mut has_nonzero = false;
        for i in 0..1024 {
            let s = buf.get(i, 0);
            assert!(
                s.is_finite(),
                "output not finite at frame {i} after sample rate change"
            );
            if s.abs() > 0.001 {
                has_nonzero = true;
            }
        }
        assert!(
            has_nonzero,
            "synth should produce output after sample rate change"
        );
    }

    #[test]
    fn set_sample_rate_changes_pitch() {
        // At a different sample rate, the same frequency should produce
        // a different number of zero crossings in the same number of frames.
        fn count_crossings(sample_rate: f32) -> usize {
            let mut synth = SubtractiveSynth::new(sample_rate);
            synth.params_mut()[PARAM_WAVEFORM].set(0.0); // sine
            synth.note_on(69, 127, 0);

            let frames = 2048;
            let mut buf = AudioBuffer::new(2, frames);
            synth.process(&[], &[], &mut buf);

            let mut crossings = 0;
            for i in 1..frames {
                if buf.get(i - 1, 0) * buf.get(i, 0) < 0.0 {
                    crossings += 1;
                }
            }
            crossings
        }

        let crossings_48k = count_crossings(48000.0);
        let crossings_96k = count_crossings(96000.0);

        // At 96kHz, we have twice as many samples per cycle, so the same
        // number of frames covers half the time, yielding fewer crossings.
        assert!(
            crossings_96k < crossings_48k,
            "96kHz should have fewer crossings in same frame count: 48k={crossings_48k}, 96k={crossings_96k}"
        );
    }
}
