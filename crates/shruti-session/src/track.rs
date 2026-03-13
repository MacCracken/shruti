use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::automation::AutomationLane;
use crate::midi::MidiClip;
use crate::region::{Region, RegionId};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Pre/post fader send position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SendPosition {
    PreFader,
    PostFader,
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
        }
    }

    /// Add a region to this track.
    pub fn add_region(&mut self, region: Region) {
        self.regions.push(region);
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
    pub fn regions_in_range(&self, start: u64, end: u64) -> Vec<&Region> {
        self.regions
            .iter()
            .filter(|r| !r.muted && r.timeline_pos < end && r.end_pos() > start)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_regions() {
        let mut track = Track::new_audio("Track 1");
        let r1 = Region::new("file1".into(), 0, 0, 1000);
        let r2 = Region::new("file2".into(), 2000, 0, 500);
        let r1_id = r1.id;

        track.add_region(r1);
        track.add_region(r2);
        assert_eq!(track.regions.len(), 2);

        // Range query
        let in_range = track.regions_in_range(500, 1500);
        assert_eq!(in_range.len(), 1);

        let in_range = track.regions_in_range(0, 3000);
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

        let mut clip = crate::midi::MidiClip::new("Intro", 0, 48000);
        clip.add_note(0, 12000, 60, 100, 0);
        clip.add_note(12000, 12000, 64, 90, 0);
        clip.add_cc(0, 1, 64, 0);

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
}
