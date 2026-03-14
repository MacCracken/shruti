use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::automation::AutomationLane;
use crate::midi::MidiClip;
use crate::region::{Region, RegionId};
use crate::types::FramePos;

/// Unique identifier for a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub Uuid);

impl TrackId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

/// The kind of track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackKind {
    /// Audio track with regions on the timeline.
    Audio,
    /// Bus track for routing and grouping.
    Bus,
    /// MIDI track with MIDI clips.
    Midi,
    /// Master output bus.
    Master,
    /// Instrument track — hosts a virtual instrument identified by type name.
    Instrument {
        /// The instrument type loaded on this track (e.g. "SubtractiveSynth").
        /// `None` means no instrument is loaded yet.
        #[serde(default)]
        instrument_type: Option<String>,
    },
    /// Drum machine track with pad-based sequencing.
    DrumMachine {
        /// Kit name loaded on this track.
        #[serde(default)]
        kit_name: Option<String>,
        /// Number of pads (default 16).
        #[serde(default = "default_pad_count")]
        pad_count: u8,
    },
    /// Sampler track — hosts a sampler instrument with zones/multisamples.
    Sampler {
        /// Preset/multisample name loaded on this track.
        #[serde(default)]
        preset_name: Option<String>,
        /// Number of zones configured.
        #[serde(default)]
        zone_count: usize,
    },
    /// AI player track — generates or accompanies using an AI model.
    AiPlayer {
        /// Model name/ID for the AI player.
        #[serde(default)]
        model_name: Option<String>,
        /// Style preset (e.g. "jazz_piano", "fingerstyle_guitar").
        #[serde(default)]
        style: Option<String>,
        /// Creativity level 0.0 (conservative) to 1.0 (experimental).
        #[serde(default = "default_creativity")]
        creativity: f32,
    },
}

/// Default creativity level for AI player tracks.
fn default_creativity() -> f32 {
    0.5
}

impl PartialEq for TrackKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TrackKind::Audio, TrackKind::Audio)
            | (TrackKind::Bus, TrackKind::Bus)
            | (TrackKind::Midi, TrackKind::Midi)
            | (TrackKind::Master, TrackKind::Master) => true,
            (
                TrackKind::Instrument { instrument_type: a },
                TrackKind::Instrument { instrument_type: b },
            ) => a == b,
            (
                TrackKind::DrumMachine {
                    kit_name: a_kit,
                    pad_count: a_pads,
                },
                TrackKind::DrumMachine {
                    kit_name: b_kit,
                    pad_count: b_pads,
                },
            ) => a_kit == b_kit && a_pads == b_pads,
            (
                TrackKind::Sampler {
                    preset_name: a_preset,
                    zone_count: a_zones,
                },
                TrackKind::Sampler {
                    preset_name: b_preset,
                    zone_count: b_zones,
                },
            ) => a_preset == b_preset && a_zones == b_zones,
            (
                TrackKind::AiPlayer {
                    model_name: a_model,
                    style: a_style,
                    creativity: a_cr,
                },
                TrackKind::AiPlayer {
                    model_name: b_model,
                    style: b_style,
                    creativity: b_cr,
                },
            ) => a_model == b_model && a_style == b_style && a_cr.to_bits() == b_cr.to_bits(),
            _ => false,
        }
    }
}

impl Eq for TrackKind {}

/// Default pad count for drum machine tracks.
fn default_pad_count() -> u8 {
    16
}

impl TrackKind {
    /// Unicode icon character for this track kind.
    pub fn icon(&self) -> &'static str {
        match self {
            TrackKind::Audio => "\u{1F3B5}",              // musical note
            TrackKind::Bus => "\u{1F500}",                // shuffle (routing)
            TrackKind::Midi => "\u{1F3B9}",               // musical keyboard
            TrackKind::Master => "\u{1F50A}",             // speaker high volume
            TrackKind::Instrument { .. } => "\u{1F3B8}",  // guitar (instrument)
            TrackKind::DrumMachine { .. } => "\u{1F941}", // drum
            TrackKind::Sampler { .. } => "\u{1F4BF}",     // optical disc (sample)
            TrackKind::AiPlayer { .. } => "\u{1F916}",    // robot
        }
    }

    /// Default RGB color for this track kind.
    pub fn default_color(&self) -> [u8; 3] {
        match self {
            TrackKind::Audio => [66, 133, 244],             // blue
            TrackKind::Bus => [251, 188, 4],                // amber
            TrackKind::Midi => [52, 168, 83],               // green
            TrackKind::Master => [234, 67, 53],             // red
            TrackKind::Instrument { .. } => [171, 71, 188], // purple
            TrackKind::DrumMachine { .. } => [255, 152, 0], // orange
            TrackKind::Sampler { .. } => [0, 150, 136],     // teal
            TrackKind::AiPlayer { .. } => [103, 58, 183],   // deep purple
        }
    }

    /// Short label for this track kind.
    pub fn label(&self) -> &'static str {
        match self {
            TrackKind::Audio => "Audio",
            TrackKind::Bus => "Bus",
            TrackKind::Midi => "MIDI",
            TrackKind::Master => "Master",
            TrackKind::Instrument { .. } => "Instrument",
            TrackKind::DrumMachine { .. } => "Drum Machine",
            TrackKind::Sampler { .. } => "Sampler",
            TrackKind::AiPlayer { .. } => "AI Player",
        }
    }
}

/// Pre/post fader send position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SendPosition {
    PreFader,
    PostFader,
}

/// Output routing configuration for a track.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputRouting {
    /// Primary output target (a bus or master track ID). If None, routes to master.
    pub output: Option<TrackId>,
    /// Optional sidechain source for compressor keying.
    pub sidechain_input: Option<TrackId>,
}

/// A send from one track to a bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Send {
    /// Target bus track.
    pub target: TrackId,
    /// Send level (linear gain, 0.0 to 1.0).
    pub level: f32,
    /// Pre or post fader.
    pub position: SendPosition,
    /// Whether this send is enabled.
    pub enabled: bool,
}

/// Unique identifier for a track group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackGroupId(pub Uuid);

impl TrackGroupId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TrackGroupId {
    fn default() -> Self {
        Self::new()
    }
}

/// A named group of tracks for organizational purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackGroup {
    pub id: TrackGroupId,
    pub name: String,
    /// Ordered list of member track IDs.
    pub tracks: Vec<TrackId>,
    /// Whether the group is collapsed in the UI.
    #[serde(default)]
    pub collapsed: bool,
}

impl TrackGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: TrackGroupId::new(),
            name: name.into(),
            tracks: Vec::new(),
            collapsed: false,
        }
    }

    /// Add a track to this group if not already present.
    pub fn add_track(&mut self, track_id: TrackId) -> bool {
        if self.tracks.contains(&track_id) {
            return false;
        }
        self.tracks.push(track_id);
        true
    }

    /// Remove a track from this group.
    pub fn remove_track(&mut self, track_id: TrackId) -> bool {
        if let Some(pos) = self.tracks.iter().position(|&id| id == track_id) {
            self.tracks.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if a track is in this group.
    pub fn contains(&self, track_id: TrackId) -> bool {
        self.tracks.contains(&track_id)
    }
}

/// A track in the session.
///
/// NOTE: `name` uses `String` rather than a compact/small-string type. Track
/// names are typically short (< 30 chars) but are cloned infrequently (template
/// creation, serialization). The extra dependency cost of `compact_str` or
/// `smol_str` is not justified for the marginal allocation savings here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub kind: TrackKind,
    /// Regions placed on this track's timeline.
    pub regions: Vec<Region>,
    /// Track gain (linear, 1.0 = unity).
    pub gain: f32,
    /// Track pan (-1.0 = full left, 0.0 = center, 1.0 = full right).
    pub pan: f32,
    /// Track is muted.
    pub muted: bool,
    /// Track is soloed.
    pub solo: bool,
    /// Track is armed for recording.
    pub armed: bool,
    /// Number of channels (typically 2 for stereo).
    pub channels: u16,
    /// Aux sends to bus tracks.
    pub sends: Vec<Send>,
    /// Automation lanes for this track.
    pub automation: Vec<AutomationLane>,
    /// MIDI clips on this track (only used for Midi tracks).
    pub midi_clips: Vec<MidiClip>,
    /// Instrument parameter values for Instrument tracks (indexed by param position).
    #[serde(default)]
    pub instrument_params: Vec<f32>,
    /// Custom track color override (RGB). If `None`, uses `TrackKind::default_color()`.
    #[serde(default)]
    pub color: Option<[u8; 3]>,
    /// Output routing configuration.
    #[serde(default)]
    pub routing: OutputRouting,
}

impl Track {
    pub fn new_audio(name: impl Into<String>) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::Audio,
            regions: Vec::new(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            armed: false,
            channels: 2,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: Vec::new(),
            color: None,
            routing: OutputRouting::default(),
        }
    }

    pub fn new_bus(name: impl Into<String>) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::Bus,
            regions: Vec::new(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            armed: false,
            channels: 2,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: Vec::new(),
            color: None,
            routing: OutputRouting::default(),
        }
    }

    pub fn new_midi(name: impl Into<String>) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::Midi,
            regions: Vec::new(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            armed: false,
            channels: 2,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: Vec::new(),
            color: None,
            routing: OutputRouting::default(),
        }
    }

    pub fn new_instrument(name: impl Into<String>, instrument_type: Option<String>) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::Instrument { instrument_type },
            regions: Vec::new(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            armed: false,
            channels: 2,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: Vec::new(),
            color: None,
            routing: OutputRouting::default(),
        }
    }

    pub fn new_drum_machine(name: impl Into<String>, kit_name: Option<String>) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::DrumMachine {
                kit_name,
                pad_count: default_pad_count(),
            },
            regions: Vec::new(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            armed: false,
            channels: 2,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: Vec::new(),
            color: None,
            routing: OutputRouting::default(),
        }
    }

    pub fn new_sampler(name: impl Into<String>, preset_name: Option<String>) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::Sampler {
                preset_name,
                zone_count: 0,
            },
            regions: Vec::new(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            armed: false,
            channels: 2,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: Vec::new(),
            color: None,
            routing: OutputRouting::default(),
        }
    }

    pub fn new_ai_player(
        name: impl Into<String>,
        model_name: Option<String>,
        style: Option<String>,
    ) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::AiPlayer {
                model_name,
                style,
                creativity: default_creativity(),
            },
            regions: Vec::new(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            armed: false,
            channels: 2,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: Vec::new(),
            color: None,
            routing: OutputRouting::default(),
        }
    }

    pub fn new_master() -> Self {
        Self {
            id: TrackId::new(),
            name: "Master".into(),
            kind: TrackKind::Master,
            regions: Vec::new(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            armed: false,
            channels: 2,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: Vec::new(),
            color: None,
            routing: OutputRouting::default(),
        }
    }

    /// Returns the track's display color: custom override or kind default.
    pub fn display_color(&self) -> [u8; 3] {
        self.color.unwrap_or_else(|| self.kind.default_color())
    }

    /// Add a region to this track, maintaining sorted order by `timeline_pos`.
    pub fn add_region(&mut self, region: Region) {
        let pos = self
            .regions
            .binary_search_by_key(&region.timeline_pos, |r| r.timeline_pos)
            .unwrap_or_else(|i| i);
        self.regions.insert(pos, region);
    }

    /// Remove a region by ID, returning it if found.
    pub fn remove_region(&mut self, id: RegionId) -> Option<Region> {
        if let Some(pos) = self.regions.iter().position(|r| r.id == id) {
            Some(self.regions.remove(pos))
        } else {
            None
        }
    }

    /// Get a region by ID.
    pub fn region(&self, id: RegionId) -> Option<&Region> {
        self.regions.iter().find(|r| r.id == id)
    }

    /// Get a mutable region by ID.
    pub fn region_mut(&mut self, id: RegionId) -> Option<&mut Region> {
        self.regions.iter_mut().find(|r| r.id == id)
    }

    /// Get all regions that overlap with the given frame range.
    ///
    /// Regions are kept sorted by `timeline_pos`, so we use binary search to
    /// skip regions that start at or after `end`, giving O(log n + k) lookup.
    pub fn regions_in_range(&self, start: FramePos, end: FramePos) -> Vec<&Region> {
        // Find the first region whose timeline_pos >= end — everything from
        // that index onward starts too late to overlap.
        let upper = self
            .regions
            .binary_search_by_key(&end, |r| r.timeline_pos)
            .unwrap_or_else(|i| i);

        self.regions[..upper]
            .iter()
            .filter(|r| !r.muted && r.end_pos() > start)
            .collect()
    }
}

/// A reusable track configuration template (kind + settings, no content).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackTemplate {
    /// Template name.
    pub name: String,
    /// Track kind.
    pub kind: TrackKind,
    /// Default gain.
    pub gain: f32,
    /// Default pan.
    pub pan: f32,
    /// Number of channels.
    pub channels: u16,
    /// Instrument parameter defaults.
    #[serde(default)]
    pub instrument_params: Vec<f32>,
    /// Custom color override.
    #[serde(default)]
    pub color: Option<[u8; 3]>,
}

impl TrackTemplate {
    /// Create a template from an existing track (captures settings, not content).
    pub fn from_track(track: &Track, template_name: &str) -> Self {
        Self {
            name: template_name.to_string(),
            kind: track.kind.clone(),
            gain: track.gain,
            pan: track.pan,
            channels: track.channels,
            instrument_params: track.instrument_params.clone(),
            color: track.color,
        }
    }

    /// Create a new track from this template.
    pub fn create_track(&self, track_name: &str) -> Track {
        Track {
            id: TrackId::new(),
            name: track_name.to_string(),
            kind: self.kind.clone(),
            regions: Vec::new(),
            gain: self.gain,
            pan: self.pan,
            muted: false,
            solo: false,
            armed: false,
            channels: self.channels,
            sends: Vec::new(),
            automation: Vec::new(),
            midi_clips: Vec::new(),
            instrument_params: self.instrument_params.clone(),
            color: self.color,
            routing: OutputRouting::default(),
        }
    }

    /// Save the template as JSON.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load a template from JSON.
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_regions() {
        let mut track = Track::new_audio("Track 1");
        let r1 = Region::new("file1".into(), 0u64, 0u64, 1000u64);
        let r2 = Region::new("file2".into(), 2000u64, 0u64, 500u64);
        let r1_id = r1.id;

        track.add_region(r1);
        track.add_region(r2);
        assert_eq!(track.regions.len(), 2);

        // Range query
        let in_range = track.regions_in_range(FramePos(500), FramePos(1500));
        assert_eq!(in_range.len(), 1);

        let in_range = track.regions_in_range(FramePos(0), FramePos(3000));
        assert_eq!(in_range.len(), 2);

        // Remove
        let removed = track.remove_region(r1_id).unwrap();
        assert_eq!(removed.id, r1_id);
        assert_eq!(track.regions.len(), 1);
    }

    #[test]
    fn test_instrument_track_creation() {
        let track = Track::new_instrument("Synth Lead", Some("SubtractiveSynth".to_string()));
        assert_eq!(
            track.kind,
            TrackKind::Instrument {
                instrument_type: Some("SubtractiveSynth".to_string())
            }
        );
        assert_eq!(track.name, "Synth Lead");
        assert!(track.midi_clips.is_empty());
        assert!(track.regions.is_empty());
    }

    #[test]
    fn test_instrument_track_no_instrument() {
        let track = Track::new_instrument("Empty Inst", None);
        assert_eq!(
            track.kind,
            TrackKind::Instrument {
                instrument_type: None
            }
        );
    }

    #[test]
    fn test_list_tracks_by_kind() {
        let tracks = [
            Track::new_audio("Audio 1"),
            Track::new_instrument("Synth", Some("SubtractiveSynth".to_string())),
            Track::new_midi("MIDI 1"),
            Track::new_instrument("Drums", Some("DrumMachine".to_string())),
            Track::new_master(),
        ];
        let instrument_count = tracks
            .iter()
            .filter(|t| matches!(t.kind, TrackKind::Instrument { .. }))
            .count();
        assert_eq!(instrument_count, 2);
    }

    #[test]
    fn test_instrument_track_serde_roundtrip() {
        let track = Track::new_instrument("Synth", Some("SubtractiveSynth".to_string()));
        let json = serde_json::to_string(&track).unwrap();
        let restored: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.kind, track.kind);
        assert_eq!(restored.name, "Synth");
    }

    #[test]
    fn test_midi_track() {
        let mut track = Track::new_midi("Synth Lead");
        assert_eq!(track.kind, TrackKind::Midi);
        assert!(track.midi_clips.is_empty());

        let mut clip = crate::midi::MidiClip::new("Intro", 0u64, 48000u64);
        clip.add_note(0u64, 12000u64, 60, 100, 0);
        clip.add_note(12000u64, 12000u64, 64, 90, 0);
        clip.add_cc(0u64, 1, 64, 0);

        track.midi_clips.push(clip);
        assert_eq!(track.midi_clips.len(), 1);
        assert_eq!(track.midi_clips[0].notes.len(), 2);
        assert_eq!(track.midi_clips[0].control_changes.len(), 1);
    }

    #[test]
    fn test_track_group_creation() {
        let group = TrackGroup::new("Drums");
        assert_eq!(group.name, "Drums");
        assert!(group.tracks.is_empty());
        assert!(!group.collapsed);
    }

    #[test]
    fn test_track_group_add_remove() {
        let mut group = TrackGroup::new("Vocals");
        let t1 = TrackId::new();
        let t2 = TrackId::new();

        assert!(group.add_track(t1));
        assert!(group.add_track(t2));
        assert_eq!(group.tracks.len(), 2);

        // Duplicate add returns false
        assert!(!group.add_track(t1));
        assert_eq!(group.tracks.len(), 2);

        assert!(group.contains(t1));
        assert!(group.contains(t2));

        assert!(group.remove_track(t1));
        assert!(!group.contains(t1));
        assert_eq!(group.tracks.len(), 1);

        // Remove non-member returns false
        assert!(!group.remove_track(t1));
    }

    #[test]
    fn test_track_group_serde_roundtrip() {
        let mut group = TrackGroup::new("FX");
        group.add_track(TrackId::new());
        group.collapsed = true;
        let json = serde_json::to_string(&group).unwrap();
        let restored: TrackGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, group.id);
        assert_eq!(restored.name, "FX");
        assert_eq!(restored.tracks.len(), 1);
        assert!(restored.collapsed);
    }

    #[test]
    fn track_kind_icons_are_distinct() {
        let kinds = [
            TrackKind::Audio,
            TrackKind::Bus,
            TrackKind::Midi,
            TrackKind::Master,
            TrackKind::Instrument {
                instrument_type: None,
            },
            TrackKind::DrumMachine {
                kit_name: None,
                pad_count: 16,
            },
            TrackKind::Sampler {
                preset_name: None,
                zone_count: 0,
            },
            TrackKind::AiPlayer {
                model_name: None,
                style: None,
                creativity: 0.5,
            },
        ];
        let icons: Vec<&str> = kinds.iter().map(|k| k.icon()).collect();
        // All icons should be unique
        for (i, a) in icons.iter().enumerate() {
            for (j, b) in icons.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "icons should be unique");
                }
            }
        }
    }

    #[test]
    fn track_kind_colors_are_distinct() {
        let kinds = [
            TrackKind::Audio,
            TrackKind::Bus,
            TrackKind::Midi,
            TrackKind::Master,
            TrackKind::Instrument {
                instrument_type: None,
            },
            TrackKind::DrumMachine {
                kit_name: None,
                pad_count: 16,
            },
            TrackKind::Sampler {
                preset_name: None,
                zone_count: 0,
            },
            TrackKind::AiPlayer {
                model_name: None,
                style: None,
                creativity: 0.5,
            },
        ];
        let colors: Vec<[u8; 3]> = kinds.iter().map(|k| k.default_color()).collect();
        for (i, a) in colors.iter().enumerate() {
            for (j, b) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "colors should be unique");
                }
            }
        }
    }

    #[test]
    fn track_kind_labels() {
        assert_eq!(TrackKind::Audio.label(), "Audio");
        assert_eq!(TrackKind::Bus.label(), "Bus");
        assert_eq!(TrackKind::Midi.label(), "MIDI");
        assert_eq!(TrackKind::Master.label(), "Master");
        assert_eq!(
            TrackKind::Instrument {
                instrument_type: None
            }
            .label(),
            "Instrument"
        );
        assert_eq!(
            TrackKind::Sampler {
                preset_name: None,
                zone_count: 0
            }
            .label(),
            "Sampler"
        );
        assert_eq!(
            TrackKind::DrumMachine {
                kit_name: None,
                pad_count: 16
            }
            .label(),
            "Drum Machine"
        );
    }

    // ── DrumMachine track tests ─────────────────────────────────────

    #[test]
    fn drum_machine_track_creation() {
        let track = Track::new_drum_machine("808 Kit", Some("TR-808".to_string()));
        assert_eq!(track.name, "808 Kit");
        assert_eq!(
            track.kind,
            TrackKind::DrumMachine {
                kit_name: Some("TR-808".to_string()),
                pad_count: 16,
            }
        );
        assert!(track.regions.is_empty());
        assert!(track.midi_clips.is_empty());
    }

    #[test]
    fn drum_machine_default_values() {
        let track = Track::new_drum_machine("Drums", None);
        match &track.kind {
            TrackKind::DrumMachine {
                kit_name,
                pad_count,
            } => {
                assert_eq!(*kit_name, None);
                assert_eq!(*pad_count, 16);
            }
            other => panic!("expected DrumMachine, got {:?}", other),
        }
    }

    #[test]
    fn drum_machine_icon_color_label() {
        let kind = TrackKind::DrumMachine {
            kit_name: None,
            pad_count: 16,
        };
        assert_eq!(kind.icon(), "\u{1F941}");
        assert_eq!(kind.default_color(), [255, 152, 0]);
        assert_eq!(kind.label(), "Drum Machine");
    }

    #[test]
    fn drum_machine_serde_roundtrip() {
        let track = Track::new_drum_machine("Kit", Some("Trap Kit".to_string()));
        let json = serde_json::to_string(&track).unwrap();
        let restored: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.kind, track.kind);
        assert_eq!(restored.name, "Kit");
    }

    #[test]
    fn drum_machine_serde_backward_compat() {
        let json = r#"{"DrumMachine":{}}"#;
        let kind: TrackKind = serde_json::from_str(json).unwrap();
        assert_eq!(
            kind,
            TrackKind::DrumMachine {
                kit_name: None,
                pad_count: 16
            }
        );
    }

    #[test]
    fn drum_machine_display_color_with_override() {
        let mut track = Track::new_drum_machine("Drums", None);
        assert_eq!(track.display_color(), [255, 152, 0]);
        track.color = Some([10, 20, 30]);
        assert_eq!(track.display_color(), [10, 20, 30]);
    }

    #[test]
    fn drum_machine_template_capture() {
        let mut track = Track::new_drum_machine("808", Some("TR-808".to_string()));
        track.gain = 0.8;
        track.color = Some([255, 100, 0]);
        let tmpl = TrackTemplate::from_track(&track, "808 Template");
        assert_eq!(tmpl.name, "808 Template");
        assert_eq!(
            tmpl.kind,
            TrackKind::DrumMachine {
                kit_name: Some("TR-808".to_string()),
                pad_count: 16,
            }
        );
        assert!((tmpl.gain - 0.8).abs() < f32::EPSILON);
        assert_eq!(tmpl.color, Some([255, 100, 0]));
        let new_track = tmpl.create_track("New 808");
        assert_eq!(new_track.name, "New 808");
        assert_eq!(new_track.kind, tmpl.kind);
        assert_ne!(new_track.id, track.id);
    }

    #[test]
    fn drum_machine_icon_distinct_from_all() {
        let dm_kind = TrackKind::DrumMachine {
            kit_name: None,
            pad_count: 16,
        };
        let others = [
            TrackKind::Audio,
            TrackKind::Bus,
            TrackKind::Midi,
            TrackKind::Master,
            TrackKind::Instrument {
                instrument_type: None,
            },
        ];
        for other in &others {
            assert_ne!(dm_kind.icon(), other.icon());
            assert_ne!(dm_kind.default_color(), other.default_color());
        }
    }

    #[test]
    fn track_display_color_uses_default() {
        let track = Track::new_audio("Test");
        assert_eq!(track.display_color(), TrackKind::Audio.default_color());
    }

    #[test]
    fn track_display_color_uses_override() {
        let mut track = Track::new_audio("Test");
        track.color = Some([255, 0, 128]);
        assert_eq!(track.display_color(), [255, 0, 128]);
    }

    #[test]
    fn track_color_serde_backward_compat() {
        // Serialize a track, strip the color field, and verify deserialization defaults to None
        let track = Track::new_audio("Test");
        let json = serde_json::to_string(&track).unwrap();
        let without_color = json.replace(",\"color\":null", "");
        let restored: Track = serde_json::from_str(&without_color).unwrap();
        assert!(restored.color.is_none());
    }

    #[test]
    fn template_from_track_captures_settings() {
        let mut track = Track::new_instrument("Synth Lead", Some("SubtractiveSynth".to_string()));
        track.gain = 0.7;
        track.pan = -0.2;
        track.instrument_params = vec![0.5, 0.3, 0.8];
        track.color = Some([100, 200, 50]);

        let tmpl = TrackTemplate::from_track(&track, "Lead Template");
        assert_eq!(tmpl.name, "Lead Template");
        assert_eq!(
            tmpl.kind,
            TrackKind::Instrument {
                instrument_type: Some("SubtractiveSynth".to_string())
            }
        );
        assert!((tmpl.gain - 0.7).abs() < f32::EPSILON);
        assert!((tmpl.pan - (-0.2)).abs() < f32::EPSILON);
        assert_eq!(tmpl.instrument_params, vec![0.5, 0.3, 0.8]);
        assert_eq!(tmpl.color, Some([100, 200, 50]));
    }

    #[test]
    fn template_creates_new_track() {
        let track = Track::new_audio("Original");
        let tmpl = TrackTemplate::from_track(&track, "Audio Template");

        let new_track = tmpl.create_track("New Audio");
        assert_eq!(new_track.name, "New Audio");
        assert_eq!(new_track.kind, TrackKind::Audio);
        assert_ne!(new_track.id, track.id); // New track gets a new ID
        assert!(new_track.regions.is_empty());
        assert!(new_track.midi_clips.is_empty());
    }

    #[test]
    fn template_serde_roundtrip() {
        let track = Track::new_midi("MIDI Keys");
        let tmpl = TrackTemplate::from_track(&track, "Keys Template");

        let json = serde_json::to_string(&tmpl).unwrap();
        let loaded: TrackTemplate = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.name, "Keys Template");
        assert_eq!(loaded.kind, TrackKind::Midi);
    }

    #[test]
    fn template_file_save_load() {
        let track = Track::new_bus("FX Bus");
        let tmpl = TrackTemplate::from_track(&track, "FX Bus Template");

        let dir = std::env::temp_dir().join("shruti_template_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fx_bus.json");

        tmpl.save(&path).unwrap();
        let loaded = TrackTemplate::load(&path).unwrap();

        assert_eq!(loaded.name, "FX Bus Template");
        assert_eq!(loaded.kind, TrackKind::Bus);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn template_load_error() {
        let result = TrackTemplate::load(Path::new("/nonexistent/template.json"));
        assert!(result.is_err());
    }

    // ── Sampler track tests ────────────────────────────────────────

    #[test]
    fn sampler_track_creation() {
        let track = Track::new_sampler("Pad Sampler", Some("Grand Piano".to_string()));
        assert_eq!(track.name, "Pad Sampler");
        assert_eq!(
            track.kind,
            TrackKind::Sampler {
                preset_name: Some("Grand Piano".to_string()),
                zone_count: 0,
            }
        );
        assert!(track.regions.is_empty());
        assert!(track.midi_clips.is_empty());
    }

    #[test]
    fn sampler_track_defaults() {
        let track = Track::new_sampler("Empty Sampler", None);
        assert_eq!(
            track.kind,
            TrackKind::Sampler {
                preset_name: None,
                zone_count: 0,
            }
        );
    }

    #[test]
    fn sampler_track_icon() {
        let kind = TrackKind::Sampler {
            preset_name: None,
            zone_count: 0,
        };
        assert_eq!(kind.icon(), "\u{1F4BF}");
    }

    #[test]
    fn sampler_track_color() {
        let kind = TrackKind::Sampler {
            preset_name: None,
            zone_count: 0,
        };
        assert_eq!(kind.default_color(), [0, 150, 136]);
    }

    #[test]
    fn sampler_track_label() {
        let kind = TrackKind::Sampler {
            preset_name: None,
            zone_count: 0,
        };
        assert_eq!(kind.label(), "Sampler");
    }

    #[test]
    fn sampler_track_serde_roundtrip() {
        let track = Track::new_sampler("Sampler", Some("Orchestra Hit".to_string()));
        let json = serde_json::to_string(&track).unwrap();
        let restored: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.kind, track.kind);
        assert_eq!(restored.name, "Sampler");
    }

    #[test]
    fn sampler_track_serde_backward_compat() {
        // Deserialize JSON without preset_name and zone_count — defaults should apply
        let json = r#"{"Sampler":{}}"#;
        let kind: TrackKind = serde_json::from_str(json).unwrap();
        assert_eq!(
            kind,
            TrackKind::Sampler {
                preset_name: None,
                zone_count: 0,
            }
        );
    }

    #[test]
    fn sampler_track_display_color_with_override() {
        let mut track = Track::new_sampler("Sampler", None);
        // Default color is teal
        assert_eq!(track.display_color(), [0, 150, 136]);
        // Override color
        track.color = Some([200, 100, 50]);
        assert_eq!(track.display_color(), [200, 100, 50]);
    }

    #[test]
    fn sampler_track_template_capture() {
        let mut track = Track::new_sampler("My Sampler", Some("Strings".to_string()));
        track.gain = 0.8;
        track.pan = 0.3;
        track.color = Some([10, 20, 30]);

        let tmpl = TrackTemplate::from_track(&track, "Sampler Template");
        assert_eq!(tmpl.name, "Sampler Template");
        assert_eq!(
            tmpl.kind,
            TrackKind::Sampler {
                preset_name: Some("Strings".to_string()),
                zone_count: 0,
            }
        );
        assert!((tmpl.gain - 0.8).abs() < f32::EPSILON);
        assert!((tmpl.pan - 0.3).abs() < f32::EPSILON);
        assert_eq!(tmpl.color, Some([10, 20, 30]));

        // Create a new track from the template
        let new_track = tmpl.create_track("New Sampler");
        assert_eq!(new_track.name, "New Sampler");
        assert_eq!(new_track.kind, tmpl.kind);
        assert_ne!(new_track.id, track.id);
    }

    // ── AI Player track tests ────────────────────────────────────────

    #[test]
    fn ai_player_track_creation() {
        let track = Track::new_ai_player(
            "Jazz Pianist",
            Some("gpt-music-v2".to_string()),
            Some("jazz_piano".to_string()),
        );
        assert_eq!(track.name, "Jazz Pianist");
        assert_eq!(
            track.kind,
            TrackKind::AiPlayer {
                model_name: Some("gpt-music-v2".to_string()),
                style: Some("jazz_piano".to_string()),
                creativity: 0.5,
            }
        );
        assert!(track.regions.is_empty());
        assert!(track.midi_clips.is_empty());
    }

    #[test]
    fn ai_player_track_defaults() {
        let track = Track::new_ai_player("AI", None, None);
        match &track.kind {
            TrackKind::AiPlayer {
                model_name,
                style,
                creativity,
            } => {
                assert!(model_name.is_none());
                assert!(style.is_none());
                assert!((*creativity - 0.5).abs() < f32::EPSILON);
            }
            _ => panic!("expected AiPlayer kind"),
        }
    }

    #[test]
    fn ai_player_icon_color_label() {
        let kind = TrackKind::AiPlayer {
            model_name: None,
            style: None,
            creativity: 0.5,
        };
        assert_eq!(kind.icon(), "\u{1F916}");
        assert_eq!(kind.default_color(), [103, 58, 183]);
        assert_eq!(kind.label(), "AI Player");
    }

    #[test]
    fn ai_player_serde_roundtrip() {
        let track = Track::new_ai_player(
            "AI Lead",
            Some("model-v3".to_string()),
            Some("fingerstyle_guitar".to_string()),
        );
        let json = serde_json::to_string(&track).unwrap();
        let restored: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.kind, track.kind);
        assert_eq!(restored.name, "AI Lead");
    }

    #[test]
    fn ai_player_serde_backward_compat() {
        // Simulate JSON without optional fields — serde(default) should fill them in.
        let json = r#"{"AiPlayer":{}}"#;
        let kind: TrackKind = serde_json::from_str(json).unwrap();
        match kind {
            TrackKind::AiPlayer {
                model_name,
                style,
                creativity,
            } => {
                assert!(model_name.is_none());
                assert!(style.is_none());
                assert!((creativity - 0.5).abs() < f32::EPSILON);
            }
            _ => panic!("expected AiPlayer"),
        }
    }

    #[test]
    fn ai_player_display_color_with_override() {
        let mut track = Track::new_ai_player("AI", None, None);
        // Default color
        assert_eq!(track.display_color(), [103, 58, 183]);
        // Override
        track.color = Some([255, 128, 0]);
        assert_eq!(track.display_color(), [255, 128, 0]);
    }

    #[test]
    fn ai_player_template_capture() {
        let mut track = Track::new_ai_player(
            "AI Jazz",
            Some("jazz-model".to_string()),
            Some("bebop".to_string()),
        );
        track.gain = 0.8;
        track.pan = 0.3;

        let tmpl = TrackTemplate::from_track(&track, "Jazz AI Template");
        assert_eq!(tmpl.name, "Jazz AI Template");
        assert_eq!(
            tmpl.kind,
            TrackKind::AiPlayer {
                model_name: Some("jazz-model".to_string()),
                style: Some("bebop".to_string()),
                creativity: 0.5,
            }
        );
        assert!((tmpl.gain - 0.8).abs() < f32::EPSILON);

        // Round-trip through template
        let new_track = tmpl.create_track("New Jazz AI");
        assert_eq!(new_track.name, "New Jazz AI");
        assert_eq!(new_track.kind, tmpl.kind);
    }

    #[test]
    fn ai_player_icon_color_distinct_from_others() {
        let ai = TrackKind::AiPlayer {
            model_name: None,
            style: None,
            creativity: 0.5,
        };
        let others = [
            TrackKind::Audio,
            TrackKind::Bus,
            TrackKind::Midi,
            TrackKind::Master,
            TrackKind::Instrument {
                instrument_type: None,
            },
            TrackKind::DrumMachine {
                kit_name: None,
                pad_count: 16,
            },
            TrackKind::Sampler {
                preset_name: None,
                zone_count: 0,
            },
        ];
        for other in &others {
            assert_ne!(
                ai.icon(),
                other.icon(),
                "AI Player icon should differ from {:?}",
                other
            );
            assert_ne!(
                ai.default_color(),
                other.default_color(),
                "AI Player color should differ from {:?}",
                other
            );
        }
    }

    #[test]
    fn test_add_region_maintains_sorted_order() {
        let mut track = Track::new_audio("Sorted");
        // Insert in reverse order
        track.add_region(Region::new("f3".into(), 3000u64, 0u64, 100u64));
        track.add_region(Region::new("f1".into(), 1000u64, 0u64, 100u64));
        track.add_region(Region::new("f2".into(), 2000u64, 0u64, 100u64));

        let positions: Vec<FramePos> = track.regions.iter().map(|r| r.timeline_pos).collect();
        assert_eq!(
            positions,
            vec![FramePos(1000), FramePos(2000), FramePos(3000)]
        );
    }

    #[test]
    fn test_regions_in_range_binary_search() {
        let mut track = Track::new_audio("BSearch");
        // Add many regions at evenly spaced positions
        for i in 0u64..100 {
            track.add_region(Region::new(format!("f{i}"), i * 1000, 0u64, 500u64));
        }

        // Query a range that only overlaps a few regions
        let found = track.regions_in_range(FramePos(5000), FramePos(7000));
        // Regions at 5000..5500, 6000..6500, 6500 doesn't exist so only those two
        // Plus region at 4500 doesn't exist (pos 4000 ends at 4500 < 5000)
        // Actually: pos 5000 end 5500, pos 6000 end 6500 — both overlap [5000,7000)
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].timeline_pos, FramePos(5000));
        assert_eq!(found[1].timeline_pos, FramePos(6000));
    }
}
