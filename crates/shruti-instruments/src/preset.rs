//! Instrument preset save/load system.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::instrument::{InstrumentNode, InstrumentParam};

/// A saved snapshot of an instrument's parameter state.
///
/// Presets are lightweight value types intended for save/load and one-shot
/// application via [`apply_to`](Self::apply_to).  `apply_to` borrows the
/// preset immutably and never clones the parameter list, so applying a preset
/// is allocation-free.  If a preset must be shared across threads, wrap it in
/// `Arc<InstrumentPreset>` at the call site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentPreset {
    /// Human-readable preset name.
    pub name: String,
    /// Category (e.g. "Bass", "Pad", "Lead").
    pub category: String,
    /// Author of the preset.
    pub author: String,
    /// Type of instrument this preset is for (e.g. "SubtractiveSynth").
    pub instrument_type: String,
    /// Saved parameter values.
    pub params: Vec<PresetParam>,
}

/// A single parameter name/value pair inside a preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetParam {
    pub name: String,
    pub value: f32,
}

impl InstrumentPreset {
    /// Capture the current state of an instrument as a preset.
    pub fn from_instrument(instrument: &dyn InstrumentNode, name: &str) -> Self {
        let info = instrument.info();
        let params = instrument
            .params()
            .iter()
            .map(|p| PresetParam {
                name: p.name.clone(),
                value: p.value,
            })
            .collect();
        Self {
            name: name.to_string(),
            category: info.category.clone(),
            author: info.author.clone(),
            instrument_type: info.name.clone(),
            params,
        }
    }

    /// Apply this preset's parameter values to an instrument.
    ///
    /// Parameters are matched by name. Any parameter in the preset that does
    /// not exist on the instrument is silently ignored, and any instrument
    /// parameter not present in the preset is left unchanged.
    pub fn apply_to(&self, instrument: &mut dyn InstrumentNode) {
        let inst_params: &mut [InstrumentParam] = instrument.params_mut();
        for preset_param in &self.params {
            if let Some(ip) = inst_params.iter_mut().find(|p| p.name == preset_param.name) {
                ip.set(preset_param.value);
            }
        }
    }

    /// Save the preset as JSON to a file.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load a preset from a JSON file.
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::SubtractiveSynth;

    #[test]
    fn from_instrument_captures_params() {
        let synth = SubtractiveSynth::new(48000.0);
        let preset = InstrumentPreset::from_instrument(&synth, "Init Saw");
        assert_eq!(preset.name, "Init Saw");
        assert_eq!(preset.instrument_type, "Subtractive Synth");
        assert_eq!(preset.params.len(), synth.params().len());
        // Volume should match the default
        let vol = preset.params.iter().find(|p| p.name == "Volume").unwrap();
        assert!((vol.value - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_to_sets_params() {
        let mut synth = SubtractiveSynth::new(48000.0);
        let mut preset = InstrumentPreset::from_instrument(&synth, "Loud");
        // Change volume in the preset
        preset
            .params
            .iter_mut()
            .find(|p| p.name == "Volume")
            .unwrap()
            .value = 0.3;

        preset.apply_to(&mut synth);
        let vol = synth.params().iter().find(|p| p.name == "Volume").unwrap();
        assert!((vol.value - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_to_ignores_unknown_params() {
        let mut synth = SubtractiveSynth::new(48000.0);
        let mut preset = InstrumentPreset::from_instrument(&synth, "Test");
        preset.params.push(PresetParam {
            name: "NonExistent".to_string(),
            value: 42.0,
        });
        // Should not panic
        preset.apply_to(&mut synth);
    }

    #[test]
    fn roundtrip_save_load() {
        let synth = SubtractiveSynth::new(48000.0);
        let preset = InstrumentPreset::from_instrument(&synth, "Roundtrip");

        let dir = std::env::temp_dir().join("shruti_test_presets");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip_test.json");

        preset.save(&path).unwrap();
        let loaded = InstrumentPreset::load(&path).unwrap();

        assert_eq!(loaded.name, "Roundtrip");
        assert_eq!(loaded.instrument_type, preset.instrument_type);
        assert_eq!(loaded.params.len(), preset.params.len());
        for (a, b) in loaded.params.iter().zip(preset.params.iter()) {
            assert_eq!(a.name, b.name);
            assert!((a.value - b.value).abs() < f32::EPSILON);
        }

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_bad_json_returns_error() {
        let dir = std::env::temp_dir().join("shruti_test_presets_bad");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.json");
        std::fs::write(&path, "{ not valid json !!!").unwrap();

        let result = InstrumentPreset::load(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let path = std::env::temp_dir().join("shruti_nonexistent_preset_12345.json");
        let result = InstrumentPreset::load(&path);
        assert!(result.is_err());
    }

    #[test]
    fn synth_preset_roundtrip_identical_output() {
        use shruti_dsp::AudioBuffer;

        let synth = SubtractiveSynth::new(48000.0);
        let preset = InstrumentPreset::from_instrument(&synth, "Synth Default");

        // Apply to a fresh synth and verify all params match
        let mut synth2 = SubtractiveSynth::new(48000.0);
        // Modify some params first to ensure apply actually changes them
        synth2.params_mut()[0].set(0.1); // Volume
        preset.apply_to(&mut synth2);

        for (orig, restored) in synth.params().iter().zip(synth2.params().iter()) {
            assert_eq!(orig.name, restored.name);
            assert!(
                (orig.value - restored.value).abs() < f32::EPSILON,
                "param {} mismatch: {} vs {}",
                orig.name,
                orig.value,
                restored.value,
            );
        }

        // Verify both produce identical audio with same input
        synth2.note_on(60, 100, 0);
        let mut synth3 = SubtractiveSynth::new(48000.0);
        preset.apply_to(&mut synth3);
        synth3.note_on(60, 100, 0);

        let mut buf2 = AudioBuffer::new(2, 256);
        let mut buf3 = AudioBuffer::new(2, 256);
        synth2.process(&[], &[], &mut buf2);
        synth3.process(&[], &[], &mut buf3);

        for f in 0..256 {
            for ch in 0..2 {
                assert!(
                    (buf2.get(f, ch) - buf3.get(f, ch)).abs() < 1e-6,
                    "output mismatch at frame {f} ch {ch}"
                );
            }
        }
    }

    #[test]
    fn drum_machine_preset_roundtrip() {
        use crate::drum_machine::DrumMachine;

        let dm = DrumMachine::new(48000.0);
        let preset = InstrumentPreset::from_instrument(&dm, "DM Default");

        let mut dm2 = DrumMachine::new(48000.0);
        preset.apply_to(&mut dm2);

        for (orig, restored) in dm.params().iter().zip(dm2.params().iter()) {
            assert_eq!(orig.name, restored.name);
            assert!(
                (orig.value - restored.value).abs() < f32::EPSILON,
                "param {} mismatch",
                orig.name,
            );
        }
    }

    #[test]
    fn sampler_preset_roundtrip() {
        use crate::sampler::Sampler;

        let sampler = Sampler::new(48000.0);
        let preset = InstrumentPreset::from_instrument(&sampler, "Sampler Default");

        let mut sampler2 = Sampler::new(48000.0);
        // Modify a param
        sampler2.params_mut()[0].set(0.2);
        preset.apply_to(&mut sampler2);

        for (orig, restored) in sampler.params().iter().zip(sampler2.params().iter()) {
            assert_eq!(orig.name, restored.name);
            assert!(
                (orig.value - restored.value).abs() < f32::EPSILON,
                "param {} mismatch",
                orig.name,
            );
        }
    }

    #[test]
    fn preset_json_roundtrip_all_instruments() {
        use crate::drum_machine::DrumMachine;
        use crate::sampler::Sampler;

        let instruments: Vec<Box<dyn InstrumentNode>> = vec![
            Box::new(SubtractiveSynth::new(48000.0)),
            Box::new(DrumMachine::new(48000.0)),
            Box::new(Sampler::new(48000.0)),
        ];

        for inst in &instruments {
            let preset = InstrumentPreset::from_instrument(inst.as_ref(), "Test");
            let json = serde_json::to_string(&preset).unwrap();
            let loaded: InstrumentPreset = serde_json::from_str(&json).unwrap();

            assert_eq!(loaded.params.len(), preset.params.len());
            for (a, b) in loaded.params.iter().zip(preset.params.iter()) {
                assert_eq!(a.name, b.name);
                assert!((a.value - b.value).abs() < f32::EPSILON);
            }
        }
    }

    #[test]
    fn preset_values_are_clamped_on_apply() {
        let mut synth = SubtractiveSynth::new(48000.0);
        let mut preset = InstrumentPreset::from_instrument(&synth, "Clamp");
        // Set Volume way beyond max (max is 1.0)
        preset
            .params
            .iter_mut()
            .find(|p| p.name == "Volume")
            .unwrap()
            .value = 999.0;

        preset.apply_to(&mut synth);
        let vol = synth.params().iter().find(|p| p.name == "Volume").unwrap();
        assert!(
            (vol.value - 1.0).abs() < f32::EPSILON,
            "should be clamped to max"
        );
    }

    #[test]
    fn apply_to_borrows_immutably() {
        // Verify apply_to only borrows the preset, allowing reuse without clone.
        let synth = SubtractiveSynth::new(48000.0);
        let preset = InstrumentPreset::from_instrument(&synth, "Reusable");

        let mut s1 = SubtractiveSynth::new(48000.0);
        let mut s2 = SubtractiveSynth::new(48000.0);
        s1.params_mut()[0].set(0.1);
        s2.params_mut()[0].set(0.2);

        // Same preset applied twice without cloning
        preset.apply_to(&mut s1);
        preset.apply_to(&mut s2);

        assert!(
            (s1.params()[0].value - s2.params()[0].value).abs() < f32::EPSILON,
            "both instruments should have identical params after applying same preset"
        );
    }

    #[test]
    fn preset_clone_produces_independent_copy() {
        let synth = SubtractiveSynth::new(48000.0);
        let preset = InstrumentPreset::from_instrument(&synth, "Original");
        let mut cloned = preset.clone();
        cloned.name = "Cloned".to_string();
        cloned.params[0].value = 999.0;

        // Original should be unaffected
        assert_eq!(preset.name, "Original");
        assert!((preset.params[0].value - synth.params()[0].value).abs() < f32::EPSILON);
    }
}
