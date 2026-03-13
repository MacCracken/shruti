//! Drum kit preset management.
//!
//! A `DrumKit` captures the configuration of all pads in a `DrumMachine`,
//! allowing save/load of complete drum kits as JSON presets.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::drum_machine::{DrumMachine, DrumPad, NUM_PADS, PlayMode};

/// Saved configuration for a single drum pad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumKitPad {
    pub name: String,
    pub midi_note: u8,
    pub pitch: f32,
    pub gain: f32,
    pub pan: f32,
    pub decay: f32,
    pub play_mode: PlayMode,
    /// Per-pad filter cutoff (0.0–1.0, default 1.0 = open).
    #[serde(default = "default_filter_cutoff")]
    pub filter_cutoff: f32,
    /// Per-pad drive amount (1.0 = clean).
    #[serde(default = "default_drive")]
    pub drive: f32,
    /// Send level to reverb (0.0–1.0).
    #[serde(default)]
    pub reverb_send: f32,
    /// Send level to delay (0.0–1.0).
    #[serde(default)]
    pub delay_send: f32,
    /// Optional path to the sample file (for reload on kit load).
    pub sample_path: Option<String>,
}

fn default_filter_cutoff() -> f32 {
    1.0
}
fn default_drive() -> f32 {
    1.0
}

impl DrumKitPad {
    /// Capture pad settings (excluding sample data).
    pub fn from_pad(pad: &DrumPad) -> Self {
        Self {
            name: pad.name.clone(),
            midi_note: pad.midi_note,
            pitch: pad.pitch,
            gain: pad.gain,
            pan: pad.pan,
            decay: pad.decay,
            play_mode: pad.play_mode,
            filter_cutoff: pad.effects.filter_cutoff,
            drive: pad.effects.drive,
            reverb_send: pad.effects.reverb_send,
            delay_send: pad.effects.delay_send,
            sample_path: None,
        }
    }

    /// Apply settings to a pad (excluding sample data).
    pub fn apply_to(&self, pad: &mut DrumPad) {
        pad.name = self.name.clone();
        pad.midi_note = self.midi_note;
        pad.pitch = self.pitch;
        pad.gain = self.gain;
        pad.pan = self.pan;
        pad.decay = self.decay;
        pad.play_mode = self.play_mode;
        pad.effects.filter_cutoff = self.filter_cutoff;
        pad.effects.drive = self.drive;
        pad.effects.reverb_send = self.reverb_send;
        pad.effects.delay_send = self.delay_send;
    }
}

/// A drum kit preset: a complete snapshot of all pad configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumKit {
    pub name: String,
    pub category: String,
    pub author: String,
    pub pads: Vec<DrumKitPad>,
}

impl DrumKit {
    /// Capture the current state of a drum machine as a kit preset.
    pub fn from_drum_machine(dm: &DrumMachine, name: &str) -> Self {
        let pads = dm.pads.iter().map(DrumKitPad::from_pad).collect();
        Self {
            name: name.to_string(),
            category: "Drum Kit".to_string(),
            author: "User".to_string(),
            pads,
        }
    }

    /// Apply this kit's pad settings to a drum machine.
    ///
    /// Pads are applied by index up to `min(kit.pads.len(), NUM_PADS)`.
    /// Sample data is not loaded by this method — the caller must load
    /// samples separately using the `sample_path` fields if present.
    pub fn apply_to(&self, dm: &mut DrumMachine) {
        let count = self.pads.len().min(NUM_PADS);
        for i in 0..count {
            self.pads[i].apply_to(&mut dm.pads[i]);
        }
    }

    /// Save the kit as JSON to a file.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load a kit from a JSON file.
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_configured_dm() -> DrumMachine {
        let mut dm = DrumMachine::new(44100.0);
        dm.pads[0].name = "Custom Kick".to_string();
        dm.pads[0].pitch = 0.8;
        dm.pads[0].gain = 0.9;
        dm.pads[0].pan = -0.3;
        dm.pads[0].decay = 0.5;
        dm.pads[0].play_mode = PlayMode::Looped;
        dm.pads[1].name = "Custom Snare".to_string();
        dm.pads[1].pitch = 1.2;
        dm
    }

    #[test]
    fn from_drum_machine_captures_all_pads() {
        let dm = make_configured_dm();
        let kit = DrumKit::from_drum_machine(&dm, "My Kit");

        assert_eq!(kit.name, "My Kit");
        assert_eq!(kit.pads.len(), NUM_PADS);
        assert_eq!(kit.pads[0].name, "Custom Kick");
        assert!((kit.pads[0].pitch - 0.8).abs() < f32::EPSILON);
        assert!((kit.pads[0].gain - 0.9).abs() < f32::EPSILON);
        assert!((kit.pads[0].pan - (-0.3)).abs() < f32::EPSILON);
        assert_eq!(kit.pads[0].play_mode, PlayMode::Looped);
        assert_eq!(kit.pads[1].name, "Custom Snare");
    }

    #[test]
    fn apply_to_restores_pad_settings() {
        let dm = make_configured_dm();
        let kit = DrumKit::from_drum_machine(&dm, "Restore Test");

        let mut fresh_dm = DrumMachine::new(44100.0);
        assert_eq!(fresh_dm.pads[0].name, "Bass Drum");

        kit.apply_to(&mut fresh_dm);

        assert_eq!(fresh_dm.pads[0].name, "Custom Kick");
        assert!((fresh_dm.pads[0].pitch - 0.8).abs() < f32::EPSILON);
        assert!((fresh_dm.pads[0].gain - 0.9).abs() < f32::EPSILON);
        assert!((fresh_dm.pads[0].pan - (-0.3)).abs() < f32::EPSILON);
        assert_eq!(fresh_dm.pads[0].play_mode, PlayMode::Looped);
        assert_eq!(fresh_dm.pads[1].name, "Custom Snare");
    }

    #[test]
    fn kit_with_fewer_pads_only_applies_available() {
        let kit = DrumKit {
            name: "Minimal".to_string(),
            category: "Drum Kit".to_string(),
            author: "Test".to_string(),
            pads: vec![DrumKitPad {
                name: "Only Kick".to_string(),
                midi_note: 36,
                pitch: 0.5,
                gain: 0.6,
                pan: 0.0,
                decay: 0.0,
                play_mode: PlayMode::OneShot,
                filter_cutoff: 1.0,
                drive: 1.0,
                reverb_send: 0.0,
                delay_send: 0.0,
                sample_path: None,
            }],
        };

        let mut dm = DrumMachine::new(44100.0);
        kit.apply_to(&mut dm);

        assert_eq!(dm.pads[0].name, "Only Kick");
        assert!((dm.pads[0].pitch - 0.5).abs() < f32::EPSILON);
        // Pad 1 should remain unchanged
        assert_eq!(dm.pads[1].name, "Rim Shot");
    }

    #[test]
    fn serde_roundtrip() {
        let dm = make_configured_dm();
        let kit = DrumKit::from_drum_machine(&dm, "Serde Test");

        let json = serde_json::to_string(&kit).unwrap();
        let loaded: DrumKit = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.name, "Serde Test");
        assert_eq!(loaded.pads.len(), NUM_PADS);
        assert_eq!(loaded.pads[0].name, "Custom Kick");
        assert!((loaded.pads[0].pitch - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn file_save_load_roundtrip() {
        let dm = make_configured_dm();
        let kit = DrumKit::from_drum_machine(&dm, "File Test");

        let dir = std::env::temp_dir().join("shruti_test_kits");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_kit.json");

        kit.save(&path).unwrap();
        let loaded = DrumKit::load(&path).unwrap();

        assert_eq!(loaded.name, "File Test");
        assert_eq!(loaded.pads.len(), NUM_PADS);
        assert_eq!(loaded.pads[0].name, "Custom Kick");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_nonexistent_returns_error() {
        let result = DrumKit::load(Path::new("/nonexistent/path/kit.json"));
        assert!(result.is_err());
    }

    #[test]
    fn load_bad_json_returns_error() {
        let dir = std::env::temp_dir().join("shruti_test_kits_bad");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_kit.json");
        std::fs::write(&path, "not json at all").unwrap();

        let result = DrumKit::load(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn sample_path_stored_in_kit() {
        let mut kit = DrumKit::from_drum_machine(&DrumMachine::new(44100.0), "Paths");
        kit.pads[0].sample_path = Some("/samples/kick.wav".to_string());
        kit.pads[1].sample_path = Some("/samples/snare.wav".to_string());

        let json = serde_json::to_string(&kit).unwrap();
        let loaded: DrumKit = serde_json::from_str(&json).unwrap();

        assert_eq!(
            loaded.pads[0].sample_path.as_deref(),
            Some("/samples/kick.wav")
        );
        assert_eq!(
            loaded.pads[1].sample_path.as_deref(),
            Some("/samples/snare.wav")
        );
        assert!(loaded.pads[2].sample_path.is_none());
    }
}
