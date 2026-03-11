use shruti_dsp::AudioBuffer;
use shruti_session::midi::{ControlChange, NoteEvent};

use crate::envelope::{AdsrParams, Envelope};
use crate::instrument::{InstrumentInfo, InstrumentNode, InstrumentParam};
use crate::oscillator::{Oscillator, Waveform};
use crate::voice::{VoiceManager, VoiceStealMode};

/// A basic subtractive synthesizer.
///
/// Features:
/// - Selectable waveform (sine, saw, square, triangle, noise)
/// - ADSR envelope per voice
/// - 16-voice polyphony with configurable voice stealing
/// - Detune control
pub struct SubtractiveSynth {
    info: InstrumentInfo,
    params: Vec<InstrumentParam>,
    voice_manager: VoiceManager,
    oscillators: Vec<Oscillator>,
    envelopes: Vec<Envelope>,
    sample_rate: f32,
}

// Parameter indices
const PARAM_WAVEFORM: usize = 0;
const PARAM_ATTACK: usize = 1;
const PARAM_DECAY: usize = 2;
const PARAM_SUSTAIN: usize = 3;
const PARAM_RELEASE: usize = 4;
const PARAM_VOLUME: usize = 5;
const PARAM_DETUNE: usize = 6;

const MAX_VOICES: usize = 16;

impl SubtractiveSynth {
    pub fn new(sample_rate: f32) -> Self {
        let info = InstrumentInfo {
            name: "Subtractive Synth".to_string(),
            category: "Synthesizer".to_string(),
            author: "Shruti".to_string(),
            description: "Basic subtractive synthesizer with ADSR and PolyBLEP oscillators"
                .to_string(),
        };

        let params = vec![
            InstrumentParam::new("Waveform", 0.0, 4.0, 1.0, ""), // 0=Sine,1=Saw,2=Square,3=Tri,4=Noise
            InstrumentParam::new("Attack", 0.001, 5.0, 0.01, "s"),
            InstrumentParam::new("Decay", 0.001, 5.0, 0.1, "s"),
            InstrumentParam::new("Sustain", 0.0, 1.0, 0.7, ""),
            InstrumentParam::new("Release", 0.001, 10.0, 0.3, "s"),
            InstrumentParam::new("Volume", 0.0, 1.0, 0.8, ""),
            InstrumentParam::new("Detune", -100.0, 100.0, 0.0, "cents"),
        ];

        let oscillators = (0..MAX_VOICES)
            .map(|_| Oscillator::new(Waveform::Saw, sample_rate as f64))
            .collect();
        let envelopes = (0..MAX_VOICES)
            .map(|_| Envelope::new(AdsrParams::default(), sample_rate))
            .collect();

        Self {
            info,
            params,
            voice_manager: VoiceManager::new(MAX_VOICES, VoiceStealMode::Oldest),
            oscillators,
            envelopes,
            sample_rate,
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
        let waveform = self.current_waveform();
        let detune = self.params[PARAM_DETUNE].value as f64;

        // Update oscillator settings
        for osc in &mut self.oscillators {
            osc.waveform = waveform;
            osc.detune = detune;
        }

        // Process note events at the start of the block
        for event in note_events {
            self.note_on(event.note, event.velocity, event.channel);
        }

        // Render each active voice
        let adsr = self.current_adsr();
        for i in 0..MAX_VOICES {
            let voice = &self.voice_manager.voices[i];
            if voice.is_idle() {
                continue;
            }

            self.envelopes[i].params = adsr.clone();
            let freq = voice.frequency();
            let vel_gain = voice.velocity as f32 / 127.0;
            let mut phase = voice.phase;

            for frame in 0..frames {
                let env_level = self.envelopes[i].tick();

                if self.envelopes[i].is_finished() {
                    self.voice_manager.free_voice(i);
                    break;
                }

                let sample = self.oscillators[i].sample(phase, freq);
                let out = sample * env_level * vel_gain * volume;

                for ch in 0..channels {
                    let current = output.get(frame as u32, ch);
                    output.set(frame as u32, ch, current + out);
                }

                phase = Oscillator::advance_phase(phase, freq, self.sample_rate as f64);
            }

            self.voice_manager.voices[i].phase = phase;
            self.voice_manager.voices[i].envelope_level = self.envelopes[i].level;
        }

        self.voice_manager.tick_age();
    }

    fn note_on(&mut self, note: u8, velocity: u8, channel: u8) {
        if let Some(idx) = self.voice_manager.note_on(note, velocity, channel) {
            self.envelopes[idx].params = self.current_adsr();
            self.envelopes[idx].trigger();
        }
    }

    fn note_off(&mut self, note: u8, channel: u8) {
        for (i, voice) in self.voice_manager.voices.iter().enumerate() {
            if voice.note == note && voice.channel == channel && !voice.is_idle() {
                self.envelopes[i].release();
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
        assert_eq!(synth.params().len(), 7);
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
}
