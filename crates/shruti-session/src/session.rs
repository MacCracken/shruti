use serde::{Deserialize, Serialize};

use crate::audio_pool::AudioPool;
use crate::timeline::Timeline;
use crate::track::{Track, TrackId, TrackKind};
use crate::transport::Transport;

/// A session is the top-level project container.
#[derive(Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub tracks: Vec<Track>,
    pub transport: Transport,
    /// Whether the session has unsaved changes.
    #[serde(skip)]
    pub dirty: bool,
    /// Audio pool (not serialized — audio files are stored on disk).
    #[serde(skip)]
    pub audio_pool: AudioPool,
    /// Timeline renderer (not serialized — runtime only).
    #[serde(skip)]
    pub timeline: Option<Timeline>,
}

impl Session {
    pub fn new(name: impl Into<String>, sample_rate: u32, buffer_size: u32) -> Self {
        let mut session = Self {
            name: name.into(),
            sample_rate,
            buffer_size,
            tracks: Vec::new(),
            transport: Transport::new(sample_rate),
            dirty: false,
            audio_pool: AudioPool::new(),
            timeline: Some(Timeline::new(2, buffer_size)),
        };

        // Every session has a master bus
        session.tracks.push(Track::new_master());
        session
    }

    /// Add a new audio track, returning its ID.
    pub fn add_audio_track(&mut self, name: impl Into<String>) -> TrackId {
        let track = Track::new_audio(name);
        let id = track.id;
        // Insert before master (master is always last)
        let master_idx = self
            .tracks
            .iter()
            .position(|t| t.kind == TrackKind::Master)
            .unwrap_or(self.tracks.len());
        self.tracks.insert(master_idx, track);
        self.dirty = true;
        id
    }

    /// Add a new MIDI track, returning its ID.
    pub fn add_midi_track(&mut self, name: impl Into<String>) -> TrackId {
        let track = Track::new_midi(name);
        let id = track.id;
        let master_idx = self
            .tracks
            .iter()
            .position(|t| t.kind == TrackKind::Master)
            .unwrap_or(self.tracks.len());
        self.tracks.insert(master_idx, track);
        self.dirty = true;
        id
    }

    /// Add a new bus track, returning its ID.
    pub fn add_bus_track(&mut self, name: impl Into<String>) -> TrackId {
        let track = Track::new_bus(name);
        let id = track.id;
        let master_idx = self
            .tracks
            .iter()
            .position(|t| t.kind == TrackKind::Master)
            .unwrap_or(self.tracks.len());
        self.tracks.insert(master_idx, track);
        self.dirty = true;
        id
    }

    /// Remove a track by ID. Cannot remove the master bus.
    pub fn remove_track(&mut self, id: TrackId) -> Option<Track> {
        let pos = self.tracks.iter().position(|t| t.id == id)?;
        if self.tracks[pos].kind == TrackKind::Master {
            return None;
        }
        self.dirty = true;
        Some(self.tracks.remove(pos))
    }

    /// Get a track by ID.
    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    /// Get a mutable track by ID.
    pub fn track_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    /// Get the master track.
    pub fn master(&self) -> Option<&Track> {
        self.tracks.iter().find(|t| t.kind == TrackKind::Master)
    }

    /// Get audio tracks (excludes bus, MIDI, and master).
    pub fn audio_tracks(&self) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Audio)
            .collect()
    }

    /// Get MIDI tracks.
    pub fn midi_tracks(&self) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Midi)
            .collect()
    }

    /// Total number of tracks (including master).
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Find the end position of the last region or MIDI clip across all tracks.
    pub fn session_length(&self) -> u64 {
        let audio_end = self
            .tracks
            .iter()
            .flat_map(|t| t.regions.iter())
            .map(|r| r.end_pos())
            .max()
            .unwrap_or(0);
        let midi_end = self
            .tracks
            .iter()
            .flat_map(|t| t.midi_clips.iter())
            .map(|c| c.end_pos())
            .max()
            .unwrap_or(0);
        audio_end.max(midi_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::Region;

    #[test]
    fn test_session_creation() {
        let session = Session::new("Test Project", 48000, 256);
        assert_eq!(session.name, "Test Project");
        assert_eq!(session.track_count(), 1); // master only
        assert!(session.master().is_some());
    }

    #[test]
    fn test_add_remove_tracks() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Guitar");
        let _t2 = session.add_audio_track("Vocals");
        let _bus = session.add_bus_track("Reverb Bus");

        assert_eq!(session.track_count(), 4); // 2 audio + 1 bus + master
        assert_eq!(session.audio_tracks().len(), 2);

        // Master is always last
        assert_eq!(session.tracks.last().unwrap().kind, TrackKind::Master);

        // Remove a track
        session.remove_track(t1);
        assert_eq!(session.track_count(), 3);

        // Cannot remove master
        let master_id = session.master().unwrap().id;
        assert!(session.remove_track(master_id).is_none());
    }

    #[test]
    fn test_session_length() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Track 1");

        let track = session.track_mut(t1).unwrap();
        track.add_region(Region::new("file1".into(), 0, 0, 48000));
        track.add_region(Region::new("file2".into(), 96000, 0, 48000));

        assert_eq!(session.session_length(), 144000); // 96000 + 48000
    }
}
