use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    /// Audio track with regions on the timeline.
    Audio,
    /// Bus track for routing and grouping.
    Bus,
    /// Master output bus.
    Master,
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
    /// IDs of bus tracks this track sends to.
    pub sends: Vec<TrackId>,
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
}
