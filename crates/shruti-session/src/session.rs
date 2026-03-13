use serde::{Deserialize, Serialize};

use crate::audio_pool::AudioPool;
use crate::error::SessionError;
use crate::timeline::Timeline;
use crate::track::{Send, SendPosition, Track, TrackGroup, TrackGroupId, TrackId, TrackKind};
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
    /// Track groups for organizational purposes.
    #[serde(default)]
    pub groups: Vec<TrackGroup>,
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
            groups: Vec::new(),
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

    /// Add a new instrument track, returning its ID.
    pub fn add_instrument_track(
        &mut self,
        name: impl Into<String>,
        instrument_type: Option<String>,
    ) -> TrackId {
        let track = Track::new_instrument(name, instrument_type);
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

    /// Add a new drum machine track, returning its ID.
    pub fn add_drum_machine_track(
        &mut self,
        name: impl Into<String>,
        kit_name: Option<String>,
    ) -> TrackId {
        let track = Track::new_drum_machine(name, kit_name);
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

    /// Add a new sampler track, returning its ID.
    pub fn add_sampler_track(
        &mut self,
        name: impl Into<String>,
        preset_name: Option<String>,
    ) -> TrackId {
        let track = Track::new_sampler(name, preset_name);
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

    /// Add a new AI player track, returning its ID.
    pub fn add_ai_player_track(
        &mut self,
        name: impl Into<String>,
        model_name: Option<String>,
        style: Option<String>,
    ) -> TrackId {
        let track = Track::new_ai_player(name, model_name, style);
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

    /// Get instrument tracks.
    pub fn instrument_tracks(&self) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| matches!(t.kind, TrackKind::Instrument { .. }))
            .collect()
    }

    /// Remove a track by ID. Cannot remove the master bus.
    /// Also removes the track from any group it belongs to.
    pub fn remove_track(&mut self, id: TrackId) -> Option<Track> {
        let pos = self.tracks.iter().position(|t| t.id == id)?;
        if self.tracks[pos].kind == TrackKind::Master {
            return None;
        }
        // Remove from any group
        for group in &mut self.groups {
            group.remove_track(id);
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
                    .is_some_and(|t| t.kind == TrackKind::Master)
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

    // ---------------------------------------------------------------
    // Track groups
    // ---------------------------------------------------------------

    /// Create a new track group, returning its ID.
    pub fn add_group(&mut self, name: impl Into<String>) -> TrackGroupId {
        let group = TrackGroup::new(name);
        let id = group.id;
        self.groups.push(group);
        self.dirty = true;
        id
    }

    /// Remove a track group by ID, returning it if found.
    pub fn remove_group(&mut self, id: TrackGroupId) -> Option<TrackGroup> {
        if let Some(pos) = self.groups.iter().position(|g| g.id == id) {
            self.dirty = true;
            Some(self.groups.remove(pos))
        } else {
            None
        }
    }

    /// Get a group by ID.
    pub fn group(&self, id: TrackGroupId) -> Option<&TrackGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Get a mutable group by ID.
    pub fn group_mut(&mut self, id: TrackGroupId) -> Option<&mut TrackGroup> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    /// Add a track to a group. Returns false if group not found or track already in group.
    pub fn add_track_to_group(&mut self, group_id: TrackGroupId, track_id: TrackId) -> bool {
        // Verify the track exists and is not master
        let track_exists = self
            .tracks
            .iter()
            .any(|t| t.id == track_id && t.kind != TrackKind::Master);
        if !track_exists {
            return false;
        }
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == group_id)
            && group.add_track(track_id)
        {
            self.dirty = true;
            return true;
        }
        false
    }

    /// Remove a track from a group.
    pub fn remove_track_from_group(&mut self, group_id: TrackGroupId, track_id: TrackId) -> bool {
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == group_id)
            && group.remove_track(track_id)
        {
            self.dirty = true;
            return true;
        }
        false
    }

    /// Find which group a track belongs to, if any.
    pub fn track_group(&self, track_id: TrackId) -> Option<&TrackGroup> {
        self.groups.iter().find(|g| g.contains(track_id))
    }

    /// Rename a group.
    pub fn rename_group(&mut self, id: TrackGroupId, name: impl Into<String>) -> bool {
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == id) {
            group.name = name.into();
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Toggle a group's collapsed state.
    pub fn toggle_group_collapsed(&mut self, id: TrackGroupId) -> bool {
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == id) {
            group.collapsed = !group.collapsed;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    // ---------------------------------------------------------------
    // Output routing
    // ---------------------------------------------------------------

    /// Set the primary output target for a track.
    /// Target must be a Bus or Master track. Pass `None` to route to master (default).
    pub fn set_track_output(
        &mut self,
        track_id: TrackId,
        output: Option<TrackId>,
    ) -> Result<(), SessionError> {
        // Cannot route master track
        if self
            .tracks
            .iter()
            .any(|t| t.id == track_id && t.kind == TrackKind::Master)
        {
            return Err(SessionError::InvalidOperation(
                "cannot set output routing on master track".into(),
            ));
        }

        // Validate target exists and is Bus or Master
        if let Some(target_id) = output {
            if target_id == track_id {
                return Err(SessionError::InvalidOperation(
                    "track cannot route to itself".into(),
                ));
            }
            let target = self
                .track(target_id)
                .ok_or_else(|| SessionError::TrackNotFound("routing target not found".into()))?;
            if target.kind != TrackKind::Bus && target.kind != TrackKind::Master {
                return Err(SessionError::InvalidOperation(
                    "output target must be a Bus or Master track".into(),
                ));
            }
            // Check for routing loops
            if self.would_create_routing_loop(track_id, target_id) {
                return Err(SessionError::InvalidOperation(
                    "routing would create a loop".into(),
                ));
            }
        }

        let track = self
            .track_mut(track_id)
            .ok_or_else(|| SessionError::TrackNotFound("source track not found".into()))?;
        track.routing.output = output;
        self.dirty = true;
        Ok(())
    }

    /// Set the sidechain input source for a track.
    pub fn set_sidechain_input(
        &mut self,
        track_id: TrackId,
        source: Option<TrackId>,
    ) -> Result<(), SessionError> {
        // Validate source track exists
        if let Some(source_id) = source {
            if source_id == track_id {
                return Err(SessionError::InvalidOperation(
                    "track cannot sidechain from itself".into(),
                ));
            }
            if self.track(source_id).is_none() {
                return Err(SessionError::TrackNotFound(
                    "sidechain source not found".into(),
                ));
            }
        }

        let track = self
            .track_mut(track_id)
            .ok_or_else(|| SessionError::TrackNotFound("track not found".into()))?;
        track.routing.sidechain_input = source;
        self.dirty = true;
        Ok(())
    }

    /// Returns the full routing chain from a track to master.
    /// The chain includes the track itself, then each output target, ending at master.
    /// Returns an empty vec if the track is not found.
    pub fn track_output_chain(&self, track_id: TrackId) -> Vec<TrackId> {
        let mut chain = Vec::new();
        let mut current = Some(track_id);
        let mut visited = std::collections::HashSet::new();

        while let Some(id) = current {
            if !visited.insert(id) {
                // Loop detected, stop
                break;
            }
            let track = match self.track(id) {
                Some(t) => t,
                None => break,
            };
            chain.push(id);
            if track.kind == TrackKind::Master {
                break;
            }
            match track.routing.output {
                Some(next) => current = Some(next),
                None => {
                    // Implicit route to master
                    if let Some(master) = self.master() {
                        chain.push(master.id);
                    }
                    break;
                }
            }
        }
        chain
    }

    /// Check if routing `source` to `target` would create a loop.
    fn would_create_routing_loop(&self, source: TrackId, target: TrackId) -> bool {
        // Walk from target following its output chain; if we reach source, it's a loop
        let mut current = Some(target);
        let mut visited = std::collections::HashSet::new();
        while let Some(id) = current {
            if id == source {
                return true;
            }
            if !visited.insert(id) {
                break;
            }
            match self.track(id) {
                Some(t) if t.kind != TrackKind::Master => {
                    current = t.routing.output;
                }
                _ => break,
            }
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

    // ---------------------------------------------------------------
    // Track reorder edge cases
    // ---------------------------------------------------------------

    #[test]
    fn test_move_track_out_of_bounds() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        session.add_audio_track("B");
        // from >= len
        assert!(!session.move_track(10, 0));
        // to >= len
        assert!(!session.move_track(0, 10));
        // both out of bounds
        assert!(!session.move_track(99, 99));
    }

    #[test]
    fn test_move_track_same_index() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        session.add_audio_track("B");
        // Moving to the same index should succeed but be a no-op
        assert!(session.move_track(0, 0));
        assert_eq!(session.tracks[0].name, "A");
        assert_eq!(session.tracks[1].name, "B");
    }

    #[test]
    fn test_move_track_sets_dirty() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        session.add_audio_track("B");
        session.dirty = false;
        assert!(session.move_track(0, 1));
        assert!(session.dirty);
    }

    #[test]
    fn test_swap_tracks_same_index() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        // a == b returns false
        assert!(!session.swap_tracks(0, 0));
    }

    #[test]
    fn test_swap_tracks_out_of_bounds() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        session.add_audio_track("B");
        // index >= len returns false
        assert!(!session.swap_tracks(0, 10));
        assert!(!session.swap_tracks(10, 0));
        assert!(!session.swap_tracks(10, 10));
    }

    // ---------------------------------------------------------------
    // Send routing — more coverage
    // ---------------------------------------------------------------

    #[test]
    fn test_add_send_invalid_source_track() {
        let mut session = Session::new("Test", 48000, 512);
        let bus_id = session.add_bus_track("FX Bus");
        let bogus_id = TrackId::new();
        assert!(!session.add_send(bogus_id, bus_id, 0.5, SendPosition::PostFader));
    }

    #[test]
    fn test_remove_send_out_of_bounds() {
        let mut session = Session::new("Test", 48000, 512);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("FX Bus");
        session.add_send(audio_id, bus_id, 0.5, SendPosition::PostFader);
        // send_index >= sends.len()
        assert!(!session.remove_send(audio_id, 1));
        assert!(!session.remove_send(audio_id, 99));
    }

    #[test]
    fn test_remove_send_invalid_track() {
        let mut session = Session::new("Test", 48000, 512);
        let bogus_id = TrackId::new();
        assert!(!session.remove_send(bogus_id, 0));
    }

    #[test]
    fn test_add_instrument_track() {
        let mut session = Session::new("Test", 48000, 256);
        let id = session.add_instrument_track("Synth", Some("SubtractiveSynth".to_string()));
        assert_eq!(session.track_count(), 2); // instrument + master
        let track = session.track(id).unwrap();
        assert_eq!(track.name, "Synth");
        assert_eq!(
            track.kind,
            TrackKind::Instrument {
                instrument_type: Some("SubtractiveSynth".to_string())
            }
        );
        // Master is still last
        assert_eq!(session.tracks.last().unwrap().kind, TrackKind::Master);
    }

    #[test]
    fn test_instrument_tracks_accessor() {
        let mut session = Session::new("Test", 48000, 256);
        session.add_instrument_track("Synth", Some("SubtractiveSynth".to_string()));
        session.add_instrument_track("Drums", Some("DrumMachine".to_string()));
        session.add_audio_track("Guitar");

        let instruments = session.instrument_tracks();
        assert_eq!(instruments.len(), 2);
        assert!(matches!(instruments[0].kind, TrackKind::Instrument { .. }));
    }

    #[test]
    fn test_instrument_track_with_no_instrument() {
        let mut session = Session::new("Test", 48000, 256);
        let id = session.add_instrument_track("Empty", None);
        let track = session.track(id).unwrap();
        assert_eq!(
            track.kind,
            TrackKind::Instrument {
                instrument_type: None
            }
        );
    }

    #[test]
    fn test_remove_instrument_track() {
        let mut session = Session::new("Test", 48000, 256);
        let id = session.add_instrument_track("Synth", Some("SubtractiveSynth".to_string()));
        assert_eq!(session.track_count(), 2);
        let removed = session.remove_track(id);
        assert!(removed.is_some());
        assert_eq!(session.track_count(), 1); // only master
    }

    #[test]
    fn test_add_multiple_sends() {
        let mut session = Session::new("Test", 48000, 512);
        let audio_id = session.add_audio_track("Guitar");
        let bus1 = session.add_bus_track("Reverb");
        let bus2 = session.add_bus_track("Delay");
        assert!(session.add_send(audio_id, bus1, 0.5, SendPosition::PostFader));
        assert!(session.add_send(audio_id, bus2, 0.3, SendPosition::PreFader));
        let track = session.track(audio_id).unwrap();
        assert_eq!(track.sends.len(), 2);
        assert_eq!(track.sends[0].target, bus1);
        assert_eq!(track.sends[1].target, bus2);
        assert!((track.sends[0].level - 0.5).abs() < f32::EPSILON);
        assert!((track.sends[1].level - 0.3).abs() < f32::EPSILON);
    }

    // ---------------------------------------------------------------
    // Track groups
    // ---------------------------------------------------------------

    #[test]
    fn test_add_and_remove_group() {
        let mut session = Session::new("Test", 48000, 256);
        let gid = session.add_group("Drums");
        assert_eq!(session.groups.len(), 1);
        assert_eq!(session.group(gid).unwrap().name, "Drums");

        let removed = session.remove_group(gid).unwrap();
        assert_eq!(removed.name, "Drums");
        assert!(session.groups.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_group() {
        let mut session = Session::new("Test", 48000, 256);
        let bogus = crate::track::TrackGroupId::new();
        assert!(session.remove_group(bogus).is_none());
    }

    #[test]
    fn test_add_track_to_group() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Kick");
        let t2 = session.add_audio_track("Snare");
        let gid = session.add_group("Drums");

        assert!(session.add_track_to_group(gid, t1));
        assert!(session.add_track_to_group(gid, t2));
        assert_eq!(session.group(gid).unwrap().tracks.len(), 2);

        // Duplicate add fails
        assert!(!session.add_track_to_group(gid, t1));
    }

    #[test]
    fn test_cannot_add_master_to_group() {
        let mut session = Session::new("Test", 48000, 256);
        let gid = session.add_group("Main");
        let master_id = session.master().unwrap().id;
        assert!(!session.add_track_to_group(gid, master_id));
    }

    #[test]
    fn test_remove_track_from_group() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Guitar");
        let gid = session.add_group("Strings");
        session.add_track_to_group(gid, t1);

        assert!(session.remove_track_from_group(gid, t1));
        assert!(session.group(gid).unwrap().tracks.is_empty());

        // Remove non-member fails
        assert!(!session.remove_track_from_group(gid, t1));
    }

    #[test]
    fn test_track_group_lookup() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Kick");
        let t2 = session.add_audio_track("Guitar");
        let gid = session.add_group("Drums");
        session.add_track_to_group(gid, t1);

        assert_eq!(session.track_group(t1).unwrap().id, gid);
        assert!(session.track_group(t2).is_none());
    }

    #[test]
    fn test_remove_track_cleans_up_group() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Kick");
        let gid = session.add_group("Drums");
        session.add_track_to_group(gid, t1);

        session.remove_track(t1);
        assert!(session.group(gid).unwrap().tracks.is_empty());
    }

    #[test]
    fn test_rename_group() {
        let mut session = Session::new("Test", 48000, 256);
        let gid = session.add_group("Old Name");
        assert!(session.rename_group(gid, "New Name"));
        assert_eq!(session.group(gid).unwrap().name, "New Name");
    }

    #[test]
    fn test_toggle_group_collapsed() {
        let mut session = Session::new("Test", 48000, 256);
        let gid = session.add_group("FX");
        assert!(!session.group(gid).unwrap().collapsed);

        assert!(session.toggle_group_collapsed(gid));
        assert!(session.group(gid).unwrap().collapsed);

        assert!(session.toggle_group_collapsed(gid));
        assert!(!session.group(gid).unwrap().collapsed);
    }

    #[test]
    fn test_group_dirty_flag() {
        let mut session = Session::new("Test", 48000, 256);
        session.dirty = false;
        session.add_group("G");
        assert!(session.dirty);
    }

    #[test]
    fn test_add_track_to_nonexistent_group() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Guitar");
        let bogus = crate::track::TrackGroupId::new();
        assert!(!session.add_track_to_group(bogus, t1));
    }

    #[test]
    fn test_add_nonexistent_track_to_group() {
        let mut session = Session::new("Test", 48000, 256);
        let gid = session.add_group("G");
        let bogus = TrackId::new();
        assert!(!session.add_track_to_group(gid, bogus));
    }

    #[test]
    fn test_remove_track_from_nonexistent_group() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Guitar");
        let bogus = crate::track::TrackGroupId::new();
        assert!(!session.remove_track_from_group(bogus, t1));
    }

    #[test]
    fn test_multiple_groups_isolation() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Kick");
        let t2 = session.add_audio_track("Guitar");
        let g1 = session.add_group("Drums");
        let g2 = session.add_group("Strings");
        session.add_track_to_group(g1, t1);
        session.add_track_to_group(g2, t2);

        assert_eq!(session.track_group(t1).unwrap().id, g1);
        assert_eq!(session.track_group(t2).unwrap().id, g2);

        // Removing from wrong group fails
        assert!(!session.remove_track_from_group(g1, t2));
        assert!(!session.remove_track_from_group(g2, t1));
    }

    #[test]
    fn test_rename_nonexistent_group() {
        let mut session = Session::new("Test", 48000, 256);
        let bogus = crate::track::TrackGroupId::new();
        assert!(!session.rename_group(bogus, "Nope"));
    }

    #[test]
    fn test_toggle_collapsed_nonexistent_group() {
        let mut session = Session::new("Test", 48000, 256);
        let bogus = crate::track::TrackGroupId::new();
        assert!(!session.toggle_group_collapsed(bogus));
    }

    #[test]
    fn test_group_mut_accessor() {
        let mut session = Session::new("Test", 48000, 256);
        let gid = session.add_group("Original");
        session.group_mut(gid).unwrap().name = "Modified".into();
        assert_eq!(session.group(gid).unwrap().name, "Modified");

        let bogus = crate::track::TrackGroupId::new();
        assert!(session.group_mut(bogus).is_none());
    }

    #[test]
    fn test_session_with_groups_serde_roundtrip() {
        let mut session = Session::new("Test", 48000, 256);
        let t1 = session.add_audio_track("Kick");
        let t2 = session.add_audio_track("Snare");
        let gid = session.add_group("Drums");
        session.add_track_to_group(gid, t1);
        session.add_track_to_group(gid, t2);
        session.group_mut(gid).unwrap().collapsed = true;

        let json = serde_json::to_string(&session).unwrap();
        let restored: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.groups.len(), 1);
        assert_eq!(restored.groups[0].name, "Drums");
        assert_eq!(restored.groups[0].tracks.len(), 2);
        assert!(restored.groups[0].collapsed);
    }

    #[test]
    fn test_session_without_groups_deserializes() {
        // Serialize a session, strip the "groups" field, then deserialize.
        // This simulates loading an old session file that predates track groups.
        let session = Session::new("Old", 48000, 256);
        let mut json = serde_json::to_string(&session).unwrap();
        // Remove the groups field from the JSON
        json = json.replace(r#","groups":[]"#, "");
        let restored: Session = serde_json::from_str(&json).unwrap();
        assert!(restored.groups.is_empty());
        assert_eq!(restored.name, "Old");
    }

    // ---------------------------------------------------------------
    // Output routing
    // ---------------------------------------------------------------

    #[test]
    fn test_route_track_to_bus() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("Reverb Bus");

        assert!(session.set_track_output(audio_id, Some(bus_id)).is_ok());
        assert_eq!(
            session.track(audio_id).unwrap().routing.output,
            Some(bus_id)
        );
    }

    #[test]
    fn test_route_bus_to_master() {
        let mut session = Session::new("Test", 48000, 256);
        let bus_id = session.add_bus_track("Reverb Bus");
        let master_id = session.master().unwrap().id;

        assert!(session.set_track_output(bus_id, Some(master_id)).is_ok());
        assert_eq!(
            session.track(bus_id).unwrap().routing.output,
            Some(master_id)
        );
    }

    #[test]
    fn test_route_track_to_audio_track_fails() {
        let mut session = Session::new("Test", 48000, 256);
        let a1 = session.add_audio_track("A");
        let a2 = session.add_audio_track("B");

        let result = session.set_track_output(a1, Some(a2));
        assert!(result.is_err());
    }

    #[test]
    fn test_route_track_to_self_fails() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");

        let result = session.set_track_output(audio_id, Some(audio_id));
        assert!(result.is_err());
    }

    #[test]
    fn test_route_master_output_fails() {
        let mut session = Session::new("Test", 48000, 256);
        let bus_id = session.add_bus_track("Bus");
        let master_id = session.master().unwrap().id;

        let result = session.set_track_output(master_id, Some(bus_id));
        assert!(result.is_err());
    }

    #[test]
    fn test_routing_loop_detection() {
        let mut session = Session::new("Test", 48000, 256);
        let bus_a = session.add_bus_track("Bus A");
        let bus_b = session.add_bus_track("Bus B");

        // bus_a -> bus_b (ok)
        assert!(session.set_track_output(bus_a, Some(bus_b)).is_ok());
        // bus_b -> bus_a would create a loop
        let result = session.set_track_output(bus_b, Some(bus_a));
        assert!(result.is_err());
    }

    #[test]
    fn test_routing_loop_detection_three_buses() {
        let mut session = Session::new("Test", 48000, 256);
        let bus_a = session.add_bus_track("Bus A");
        let bus_b = session.add_bus_track("Bus B");
        let bus_c = session.add_bus_track("Bus C");

        // A -> B -> C is fine
        assert!(session.set_track_output(bus_a, Some(bus_b)).is_ok());
        assert!(session.set_track_output(bus_b, Some(bus_c)).is_ok());
        // C -> A would create A -> B -> C -> A loop
        let result = session.set_track_output(bus_c, Some(bus_a));
        assert!(result.is_err());
    }

    #[test]
    fn test_route_to_none_resets_to_master() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("Bus");

        session.set_track_output(audio_id, Some(bus_id)).unwrap();
        session.set_track_output(audio_id, None).unwrap();
        assert!(session.track(audio_id).unwrap().routing.output.is_none());
    }

    #[test]
    fn test_route_nonexistent_track_fails() {
        let mut session = Session::new("Test", 48000, 256);
        let bus_id = session.add_bus_track("Bus");
        let bogus = TrackId::new();

        let result = session.set_track_output(bogus, Some(bus_id));
        assert!(result.is_err());
    }

    #[test]
    fn test_route_to_nonexistent_target_fails() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let bogus = TrackId::new();

        let result = session.set_track_output(audio_id, Some(bogus));
        assert!(result.is_err());
    }

    #[test]
    fn test_sidechain_assignment() {
        let mut session = Session::new("Test", 48000, 256);
        let vocal = session.add_audio_track("Vocal");
        let bass = session.add_audio_track("Bass");

        assert!(session.set_sidechain_input(bass, Some(vocal)).is_ok());
        assert_eq!(
            session.track(bass).unwrap().routing.sidechain_input,
            Some(vocal)
        );
    }

    #[test]
    fn test_sidechain_self_fails() {
        let mut session = Session::new("Test", 48000, 256);
        let vocal = session.add_audio_track("Vocal");

        let result = session.set_sidechain_input(vocal, Some(vocal));
        assert!(result.is_err());
    }

    #[test]
    fn test_sidechain_nonexistent_source_fails() {
        let mut session = Session::new("Test", 48000, 256);
        let vocal = session.add_audio_track("Vocal");
        let bogus = TrackId::new();

        let result = session.set_sidechain_input(vocal, Some(bogus));
        assert!(result.is_err());
    }

    #[test]
    fn test_sidechain_clear() {
        let mut session = Session::new("Test", 48000, 256);
        let vocal = session.add_audio_track("Vocal");
        let bass = session.add_audio_track("Bass");

        session.set_sidechain_input(bass, Some(vocal)).unwrap();
        session.set_sidechain_input(bass, None).unwrap();
        assert!(
            session
                .track(bass)
                .unwrap()
                .routing
                .sidechain_input
                .is_none()
        );
    }

    #[test]
    fn test_track_output_chain_default() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let master_id = session.master().unwrap().id;

        let chain = session.track_output_chain(audio_id);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0], audio_id);
        assert_eq!(chain[1], master_id);
    }

    #[test]
    fn test_track_output_chain_through_bus() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("Bus");
        let master_id = session.master().unwrap().id;

        session.set_track_output(audio_id, Some(bus_id)).unwrap();
        // Bus defaults to master (None output)

        let chain = session.track_output_chain(audio_id);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0], audio_id);
        assert_eq!(chain[1], bus_id);
        assert_eq!(chain[2], master_id);
    }

    #[test]
    fn test_track_output_chain_explicit_master_route() {
        let mut session = Session::new("Test", 48000, 256);
        let bus_id = session.add_bus_track("Bus");
        let master_id = session.master().unwrap().id;

        session.set_track_output(bus_id, Some(master_id)).unwrap();

        let chain = session.track_output_chain(bus_id);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0], bus_id);
        assert_eq!(chain[1], master_id);
    }

    #[test]
    fn test_track_output_chain_master_only() {
        let session = Session::new("Test", 48000, 256);
        let master_id = session.master().unwrap().id;

        let chain = session.track_output_chain(master_id);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0], master_id);
    }

    #[test]
    fn test_track_output_chain_nonexistent() {
        let session = Session::new("Test", 48000, 256);
        let bogus = TrackId::new();

        let chain = session.track_output_chain(bogus);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_routing_dirty_flag() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("Bus");
        session.dirty = false;

        session.set_track_output(audio_id, Some(bus_id)).unwrap();
        assert!(session.dirty);

        session.dirty = false;
        let vocal = session.add_audio_track("Vocal");
        session.dirty = false;
        session.set_sidechain_input(audio_id, Some(vocal)).unwrap();
        assert!(session.dirty);
    }

    #[test]
    fn test_routing_serde_backward_compat() {
        // Simulate an old session without routing fields
        let session = Session::new("Old", 48000, 256);
        let json = serde_json::to_string(&session).unwrap();
        // Remove all routing fields
        let without_routing =
            json.replace(r#","routing":{"output":null,"sidechain_input":null}"#, "");
        let restored: Session = serde_json::from_str(&without_routing).unwrap();
        let master = restored.master().unwrap();
        assert!(master.routing.output.is_none());
        assert!(master.routing.sidechain_input.is_none());
    }

    #[test]
    fn test_routing_serde_roundtrip() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("Bus");
        let vocal = session.add_audio_track("Vocal");

        session.set_track_output(audio_id, Some(bus_id)).unwrap();
        session.set_sidechain_input(audio_id, Some(vocal)).unwrap();

        let json = serde_json::to_string(&session).unwrap();
        let restored: Session = serde_json::from_str(&json).unwrap();

        let track = restored.track(audio_id).unwrap();
        assert_eq!(track.routing.output, Some(bus_id));
        assert_eq!(track.routing.sidechain_input, Some(vocal));
    }

    #[test]
    fn test_add_sampler_track_inserts_before_master() {
        let mut session = Session::new("Test", 48000, 256);
        let id = session.add_sampler_track("Pad Sampler", Some("Grand Piano".to_string()));
        assert_eq!(session.track_count(), 2); // sampler + master
        let track = session.track(id).unwrap();
        assert_eq!(track.name, "Pad Sampler");
        assert_eq!(
            track.kind,
            TrackKind::Sampler {
                preset_name: Some("Grand Piano".to_string()),
                zone_count: 0,
            }
        );
        // Master is still last
        assert_eq!(session.tracks.last().unwrap().kind, TrackKind::Master);
    }

    #[test]
    fn test_add_ai_player_track_inserts_before_master() {
        let mut session = Session::new("Test", 48000, 256);
        let id = session.add_ai_player_track(
            "Jazz AI",
            Some("jazz-model".to_string()),
            Some("bebop".to_string()),
        );
        assert_eq!(session.track_count(), 2); // ai player + master
        let track = session.track(id).unwrap();
        assert_eq!(track.name, "Jazz AI");
        assert_eq!(
            track.kind,
            TrackKind::AiPlayer {
                model_name: Some("jazz-model".to_string()),
                style: Some("bebop".to_string()),
                creativity: 0.5,
            }
        );
        // Master is still last
        assert_eq!(session.tracks.last().unwrap().kind, TrackKind::Master);
    }

    #[test]
    fn test_add_drum_machine_track() {
        let mut session = Session::new("Test", 48000, 256);
        let id = session.add_drum_machine_track("808 Drums", Some("TR-808".to_string()));
        assert_eq!(session.track_count(), 2); // drum machine + master
        let track = session.track(id).unwrap();
        assert_eq!(track.name, "808 Drums");
        assert_eq!(
            track.kind,
            TrackKind::DrumMachine {
                kit_name: Some("TR-808".to_string()),
                pad_count: 16,
            }
        );
        // Master is still last
        assert_eq!(session.tracks.last().unwrap().kind, TrackKind::Master);
        assert!(session.dirty);
    }
}
