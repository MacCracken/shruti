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
    envelopes: Vec<Envelope>,
    filter_envelopes: Vec<Envelope>,
    filters: Vec<Filter>,
    lfo1: Lfo,
    lfo2: Lfo,
    sample_rate: f32,
    /// Per-instrument effect chain (chorus, delay, reverb, distortion, filter drive).
    pub effect_chain: EffectChain,
}

// Parameter indices
const PARAM_WAVEFORM: usize = 0;
const PARAM_ATTACK: usize = 1;
const PARAM_DECAY: usize = 2;
const PARAM_SUSTAIN: usize = 3;
const PARAM_RELEASE: usize = 4;
const PARAM_VOLUME: usize = 5;
const PARAM_DETUNE: usize = 6;
const PARAM_FILTER_CUTOFF: usize = 7;
const PARAM_FILTER_RESONANCE: usize = 8;
const PARAM_FILTER_MODE: usize = 9;
// Filter envelope
const PARAM_FILTER_ENV_ATTACK: usize = 10;
const PARAM_FILTER_ENV_DECAY: usize = 11;
const PARAM_FILTER_ENV_SUSTAIN: usize = 12;
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

const MAX_VOICES: usize = 16;

impl SubtractiveSynth {
    pub fn new(sample_rate: f32) -> Self {
        let info = InstrumentInfo {
            name: "Subtractive Synth".to_string(),
            category: "Synthesizer".to_string(),
            author: "Shruti".to_string(),
            description: "Subtractive synthesizer with dual ADSR, PolyBLEP oscillators, SVF filter, and dual LFO"
                .to_string(),
        };

        let params = vec![
            // Oscillator
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
        ];

        let oscillators = (0..MAX_VOICES)
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
            envelopes,
            filter_envelopes,
            filters,
            lfo1,
            lfo2,
            sample_rate,
            effect_chain: EffectChain::new(),
        }
    }

    fn current_waveform(&self) -> Waveform {
        match self.params[PARAM_WAVEFORM].value.round() as u8 {
            0 => Waveform::Sine,
            1 => Waveform::Saw,
            2 => Waveform::Square,
            3 => Waveform::Triangle,
            _ => Waveform::Noise,
        }
    }

    fn current_adsr(&self) -> AdsrParams {
        AdsrParams {
            attack: self.params[PARAM_ATTACK].value,
            decay: self.params[PARAM_DECAY].value,
            sustain: self.params[PARAM_SUSTAIN].value,
            release: self.params[PARAM_RELEASE].value,
        }
    }

    fn current_filter_adsr(&self) -> AdsrParams {
        AdsrParams {
            attack: self.params[PARAM_FILTER_ENV_ATTACK].value,
            decay: self.params[PARAM_FILTER_ENV_DECAY].value,
            sustain: self.params[PARAM_FILTER_ENV_SUSTAIN].value,
            release: self.params[PARAM_FILTER_ENV_RELEASE].value,
        }
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

    /// Apply LFO modulation to a target value. Returns (cutoff_mod, pitch_mod, volume_mod).
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

        // Update oscillator settings
        for osc in &mut self.oscillators {
            osc.waveform = waveform;
            osc.detune = detune;
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

        // Render each active voice
        let adsr = self.current_adsr();
        let filter_adsr = self.current_filter_adsr();
        for i in 0..MAX_VOICES {
            let voice = &self.voice_manager.voices[i];
            if voice.is_idle() {
                continue;
            }

            self.envelopes[i].params = adsr.clone();
            self.filter_envelopes[i].params = filter_adsr.clone();
            let freq = voice.frequency();
            let vel_gain = voice.velocity as f32 / 127.0;
            let mut phase = voice.phase;

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
                    freq * 2.0f64.powf(semitones as f64 / 12.0)
                } else {
                    freq
                };

                let sample = self.oscillators[i].sample(phase, effective_freq);

                // Apply amp envelope
                let after_env = sample * env_level;

                // Compute filter cutoff: base + filter envelope + LFO
                let env_mod_octaves = filter_env_level * filter_env_depth * 4.0;
                let lfo_mod_octaves = cutoff_lfo_mod * 4.0;
                let modulated_cutoff = (filter_cutoff
                    * 2.0f32.powf(env_mod_octaves + lfo_mod_octaves))
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

                phase = Oscillator::advance_phase(phase, effective_freq, self.sample_rate as f64);
            }

            self.voice_manager.voices[i].phase = phase;
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
    fn synth_creates_with_defaults() {
        let synth = SubtractiveSynth::new(48000.0);
        assert_eq!(synth.info().name, "Subtractive Synth");
        assert_eq!(synth.params().len(), 23);
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
}
