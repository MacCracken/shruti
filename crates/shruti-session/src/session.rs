use serde::{Deserialize, Serialize};

use crate::audio_pool::AudioPool;
use crate::timeline::Timeline;
use crate::track::{Send, SendPosition, Track, TrackId, TrackKind};
use crate::transport::Transport;

/// A session is the top-level project container.
#[derive(Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    /// Preferred audio device name (None = system default).
    #[serde(default)]
    pub audio_device_name: Option<String>,
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
            audio_device_name: None,
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

    /// Move a track from one index to another. Master track cannot be moved.
    pub fn move_track(&mut self, from: usize, to: usize) -> bool {
        let len = self.tracks.len();
        if from >= len || to >= len {
            return false;
        }
        // Don't allow moving the master track (always last)
        if self.tracks[from].kind == TrackKind::Master
            || to >= len - 1
                && self
                    .tracks
                    .last()
                    .map(|t| t.kind)
                    .is_some_and(|k| k == TrackKind::Master)
        {
            // Prevent moving anything past master
            let master_idx = self.tracks.iter().position(|t| t.kind == TrackKind::Master);
            if let Some(mi) = master_idx
                && (from == mi || to >= mi)
            {
                return false;
            }
        }
        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);
        self.dirty = true;
        true
    }

    /// Swap two tracks by index. Neither can be the master track.
    pub fn swap_tracks(&mut self, a: usize, b: usize) -> bool {
        let len = self.tracks.len();
        if a >= len || b >= len || a == b {
            return false;
        }
        if self.tracks[a].kind == TrackKind::Master || self.tracks[b].kind == TrackKind::Master {
            return false;
        }
        self.tracks.swap(a, b);
        self.dirty = true;
        true
    }

    /// Add a send from one track to a bus track.
    pub fn add_send(
        &mut self,
        from_track: TrackId,
        to_bus: TrackId,
        level: f32,
        position: SendPosition,
    ) -> bool {
        // Verify target is a bus track
        let is_bus = self
            .tracks
            .iter()
            .any(|t| t.id == to_bus && t.kind == TrackKind::Bus);
        if !is_bus {
            return false;
        }
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == from_track) {
            track.sends.push(Send {
                target: to_bus,
                level,
                position,
                enabled: true,
            });
            self.dirty = true;
            return true;
        }
        false
    }

    /// Remove a send by index from a track.
    pub fn remove_send(&mut self, track_id: TrackId, send_index: usize) -> bool {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id)
            && send_index < track.sends.len()
        {
            track.sends.remove(send_index);
            self.dirty = true;
            return true;
        }
        false
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
    use crate::midi::MidiClip;
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

    #[test]
    fn test_add_midi_track_and_midi_tracks() {
        let mut session = Session::new("MIDI Test", 48000, 256);
        let m1 = session.add_midi_track("Synth");
        let m2 = session.add_midi_track("Piano");
        let _a1 = session.add_audio_track("Guitar");

        let midi = session.midi_tracks();
        assert_eq!(midi.len(), 2);
        assert_eq!(midi[0].kind, TrackKind::Midi);
        assert_eq!(midi[1].kind, TrackKind::Midi);

        // Master still last
        assert_eq!(session.tracks.last().unwrap().kind, TrackKind::Master);

        // Verify track IDs match
        assert_eq!(midi[0].id, m1);
        assert_eq!(midi[1].id, m2);
    }

    #[test]
    fn test_remove_track_returns_removed() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Guitar");

        let removed = session.remove_track(t1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Guitar");
        assert_eq!(session.track_count(), 1); // only master remains
    }

    #[test]
    fn test_remove_nonexistent_track() {
        let mut session = Session::new("Test", 48000, 256);
        let bogus_id = TrackId::new();
        assert!(session.remove_track(bogus_id).is_none());
    }

    #[test]
    fn test_remove_master_returns_none() {
        let mut session = Session::new("Test", 48000, 256);
        let master_id = session.master().unwrap().id;
        assert!(session.remove_track(master_id).is_none());
        // Master should still be there
        assert!(session.master().is_some());
        assert_eq!(session.track_count(), 1);
    }

    #[test]
    fn test_track_and_track_mut_lookup() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Guitar");

        // Immutable lookup
        let track = session.track(t1);
        assert!(track.is_some());
        assert_eq!(track.unwrap().name, "Guitar");

        // Mutable lookup and modify
        let track_mut = session.track_mut(t1).unwrap();
        track_mut.name = "Bass".into();
        assert_eq!(session.track(t1).unwrap().name, "Bass");

        // Non-existent ID
        let bogus_id = TrackId::new();
        assert!(session.track(bogus_id).is_none());
        assert!(session.track_mut(bogus_id).is_none());
    }

    #[test]
    fn test_master_accessor() {
        let session = Session::new("Test", 48000, 256);
        let master = session.master().unwrap();
        assert_eq!(master.kind, TrackKind::Master);
        assert_eq!(master.name, "Master");
    }

    #[test]
    fn test_session_length_with_midi_clips() {
        let mut session = Session::new("Test", 48000, 256);
        let m1 = session.add_midi_track("Synth");

        let track = session.track_mut(m1).unwrap();
        let clip = MidiClip::new("Clip 1", 10000, 50000);
        track.midi_clips.push(clip);

        // MIDI clip end = 10000 + 50000 = 60000
        assert_eq!(session.session_length(), 60000);
    }

    #[test]
    fn test_session_length_midi_and_audio_combined() {
        let mut session = Session::new("Test", 48000, 256);
        let a1 = session.add_audio_track("Audio");
        let m1 = session.add_midi_track("MIDI");

        // Audio region ends at 30000
        session
            .track_mut(a1)
            .unwrap()
            .add_region(Region::new("f".into(), 10000, 0, 20000));

        // MIDI clip ends at 100000 (should be the max)
        let clip = MidiClip::new("C", 50000, 50000);
        session.track_mut(m1).unwrap().midi_clips.push(clip);

        assert_eq!(session.session_length(), 100000);
    }

    #[test]
    fn test_session_length_empty() {
        let session = Session::new("Empty", 48000, 256);
        assert_eq!(session.session_length(), 0);
    }

    #[test]
    fn test_dirty_flag_tracking() {
        let mut session = Session::new("Test", 48000, 256);
        assert!(!session.dirty);

        let t1 = session.add_audio_track("Guitar");
        assert!(session.dirty);

        // Reset dirty and add midi track
        session.dirty = false;
        let _m1 = session.add_midi_track("Synth");
        assert!(session.dirty);

        // Reset dirty and add bus
        session.dirty = false;
        let _b1 = session.add_bus_track("Reverb");
        assert!(session.dirty);

        // Reset dirty and remove track
        session.dirty = false;
        session.remove_track(t1);
        assert!(session.dirty);
    }

    #[test]
    fn test_move_track() {
        let mut session = Session::new("Test", 48000, 512);
        let _t1 = session.add_audio_track("A");
        let _t2 = session.add_audio_track("B");
        let _t3 = session.add_audio_track("C");
        // Order: A, B, C, Master
        assert_eq!(session.tracks[0].name, "A");
        assert!(session.move_track(0, 2));
        // Order: B, C, A, Master
        assert_eq!(session.tracks[0].name, "B");
        assert_eq!(session.tracks[2].name, "A");
    }

    #[test]
    fn test_move_track_cannot_move_master() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        let master_idx = session.tracks.len() - 1;
        assert!(!session.move_track(master_idx, 0));
    }

    #[test]
    fn test_swap_tracks() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        session.add_audio_track("B");
        assert!(session.swap_tracks(0, 1));
        assert_eq!(session.tracks[0].name, "B");
        assert_eq!(session.tracks[1].name, "A");
    }

    #[test]
    fn test_swap_tracks_master_blocked() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        let master_idx = session.tracks.len() - 1;
        assert!(!session.swap_tracks(0, master_idx));
    }

    #[test]
    fn test_add_send_to_bus() {
        let mut session = Session::new("Test", 48000, 512);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("FX Bus");
        assert!(session.add_send(audio_id, bus_id, 0.5, SendPosition::PostFader));
        let track = session.track(audio_id).unwrap();
        assert_eq!(track.sends.len(), 1);
    }

    #[test]
    fn test_add_send_to_non_bus_fails() {
        let mut session = Session::new("Test", 48000, 512);
        let a = session.add_audio_track("A");
        let b = session.add_audio_track("B");
        assert!(!session.add_send(a, b, 0.5, SendPosition::PostFader));
    }

    #[test]
    fn test_remove_send() {
        let mut session = Session::new("Test", 48000, 512);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("FX Bus");
        session.add_send(audio_id, bus_id, 0.5, SendPosition::PostFader);
        assert!(session.remove_send(audio_id, 0));
        let track = session.track(audio_id).unwrap();
        assert!(track.sends.is_empty());
    }
}
