use std::collections::VecDeque;

use crate::edit::EditCommand;
use crate::session::Session;

/// Manages undo/redo history using the command pattern.
///
/// TODO(perf): Each `EditCommand` stores full copies of regions/groups. For
/// large sessions, consider using delta-based or copy-on-write representations
/// (e.g. `Arc<Region>`) to reduce memory pressure when undo history is deep.
pub struct UndoManager {
    undo_stack: VecDeque<EditCommand>,
    redo_stack: Vec<EditCommand>,
    max_history: usize,
}

impl UndoManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Execute a command, pushing it onto the undo stack.
    pub fn execute(&mut self, mut cmd: EditCommand, session: &mut Session) {
        apply_command(&mut cmd, session);
        self.undo_stack.push_back(cmd);
        self.redo_stack.clear();

        if self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front(); // O(1) eviction instead of Vec::remove(0)
        }
    }

    /// Undo the last command.
    pub fn undo(&mut self, session: &mut Session) -> bool {
        if let Some(cmd) = self.undo_stack.pop_back() {
            reverse_command(&cmd, session);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    /// Redo the last undone command.
    pub fn redo(&mut self, session: &mut Session) -> bool {
        if let Some(mut cmd) = self.redo_stack.pop() {
            apply_command(&mut cmd, session);
            self.undo_stack.push_back(cmd);
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(1000)
    }
}

fn apply_command(cmd: &mut EditCommand, session: &mut Session) {
    match cmd {
        EditCommand::AddRegion { track_id, region } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.add_region(region.clone());
            }
        }
        EditCommand::RemoveRegion {
            track_id,
            region_id,
            region,
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                *region = track.remove_region(*region_id);
            }
        }
        EditCommand::MoveRegion {
            track_id,
            region_id,
            new_pos,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.timeline_pos = *new_pos;
            }
        }
        EditCommand::MoveRegionToTrack {
            from_track,
            to_track,
            region_id,
            new_pos,
            region,
            ..
        } => {
            if let Some(track) = session.track_mut(*from_track) {
                *region = track.remove_region(*region_id);
            }
            if let Some(mut r) = region.clone() {
                r.timeline_pos = *new_pos;
                if let Some(track) = session.track_mut(*to_track) {
                    track.add_region(r);
                }
            }
        }
        EditCommand::SplitRegion {
            track_id,
            region_id,
            split_frame,
            original,
            left_id,
            right_id,
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region(*region_id)
            {
                let r_clone = r.clone();
                if let Some((left, right)) = r_clone.split_at(*split_frame) {
                    *left_id = Some(left.id);
                    *right_id = Some(right.id);
                    *original = track.remove_region(*region_id);
                    track.add_region(left);
                    track.add_region(right);
                }
            }
        }
        EditCommand::TrimStart {
            track_id,
            region_id,
            new_start,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.trim_start(*new_start);
            }
        }
        EditCommand::TrimEnd {
            track_id,
            region_id,
            new_end,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.trim_end(*new_end);
            }
        }
        EditCommand::SetFadeIn {
            track_id,
            region_id,
            new_fade,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.fade_in = *new_fade;
            }
        }
        EditCommand::SetFadeOut {
            track_id,
            region_id,
            new_fade,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.fade_out = *new_fade;
            }
        }
        EditCommand::SetRegionGain {
            track_id,
            region_id,
            new_gain,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.gain = *new_gain;
            }
        }
        EditCommand::SetTrackGain {
            track_id, new_gain, ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.gain = *new_gain;
            }
        }
        EditCommand::SetTrackPan {
            track_id, new_pan, ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.pan = *new_pan;
            }
        }
        EditCommand::ToggleTrackMute { track_id } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.muted = !track.muted;
            }
        }
        EditCommand::ToggleTrackSolo { track_id } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.solo = !track.solo;
            }
        }
        EditCommand::MoveTrack {
            from_index,
            to_index,
        } => {
            if *from_index < session.tracks.len() {
                let track = session.tracks.remove(*from_index);
                let actual_to = (*to_index).min(session.tracks.len());
                session.tracks.insert(actual_to, track);
            }
        }
        EditCommand::SetInstrumentParam {
            track_id,
            param_index,
            new_value,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(val) = track.instrument_params.get_mut(*param_index)
            {
                *val = *new_value;
            }
        }
        EditCommand::CreateGroup {
            group_id,
            name,
            group,
        } => {
            let mut new_group = crate::track::TrackGroup::new(name.clone());
            new_group.id = *group_id;
            session.groups.push(new_group.clone());
            *group = Some(new_group);
        }
        EditCommand::RemoveGroup { group_id, group } => {
            if let Some(pos) = session.groups.iter().position(|g| g.id == *group_id) {
                *group = Some(session.groups.remove(pos));
            }
        }
        EditCommand::AddTrackToGroup { group_id, track_id } => {
            session.add_track_to_group(*group_id, *track_id);
        }
        EditCommand::RemoveTrackFromGroup { group_id, track_id } => {
            session.remove_track_from_group(*group_id, *track_id);
        }
        EditCommand::RenameGroup {
            group_id, new_name, ..
        } => {
            session.rename_group(*group_id, new_name.clone());
        }
        EditCommand::ToggleGroupCollapsed { group_id } => {
            session.toggle_group_collapsed(*group_id);
        }
        EditCommand::SetTrackOutput {
            track_id,
            new_output,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.routing.output = *new_output;
            }
        }
        EditCommand::SetSidechainInput {
            track_id,
            new_source,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.routing.sidechain_input = *new_source;
            }
        }
        EditCommand::Compound { commands } => {
            for sub in commands.iter_mut() {
                apply_command(sub, session);
            }
        }
    }
}

fn reverse_command(cmd: &EditCommand, session: &mut Session) {
    match cmd {
        EditCommand::AddRegion { track_id, region } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.remove_region(region.id);
            }
        }
        EditCommand::RemoveRegion {
            track_id, region, ..
        } => {
            if let Some(r) = region
                && let Some(track) = session.track_mut(*track_id)
            {
                track.add_region(r.clone());
            }
        }
        EditCommand::MoveRegion {
            track_id,
            region_id,
            old_pos,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.timeline_pos = *old_pos;
            }
        }
        EditCommand::MoveRegionToTrack {
            from_track,
            to_track,
            region_id,
            old_pos,
            region,
            ..
        } => {
            if let Some(track) = session.track_mut(*to_track) {
                track.remove_region(*region_id);
            }
            if let Some(mut r) = region.clone() {
                r.timeline_pos = *old_pos;
                if let Some(track) = session.track_mut(*from_track) {
                    track.add_region(r);
                }
            }
        }
        EditCommand::SplitRegion {
            track_id,
            original,
            left_id,
            right_id,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                if let Some(lid) = left_id {
                    track.remove_region(*lid);
                }
                if let Some(rid) = right_id {
                    track.remove_region(*rid);
                }
                if let Some(orig) = original {
                    track.add_region(orig.clone());
                }
            }
        }
        EditCommand::TrimStart {
            track_id,
            region_id,
            old_start,
            old_offset,
            old_duration,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.timeline_pos = *old_start;
                r.source_offset = *old_offset;
                r.duration = *old_duration;
            }
        }
        EditCommand::TrimEnd {
            track_id,
            region_id,
            old_duration,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.duration = *old_duration;
            }
        }
        EditCommand::SetFadeIn {
            track_id,
            region_id,
            old_fade,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.fade_in = *old_fade;
            }
        }
        EditCommand::SetFadeOut {
            track_id,
            region_id,
            old_fade,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.fade_out = *old_fade;
            }
        }
        EditCommand::SetRegionGain {
            track_id,
            region_id,
            old_gain,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(r) = track.region_mut(*region_id)
            {
                r.gain = *old_gain;
            }
        }
        EditCommand::SetTrackGain {
            track_id, old_gain, ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.gain = *old_gain;
            }
        }
        EditCommand::SetTrackPan {
            track_id, old_pan, ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.pan = *old_pan;
            }
        }
        EditCommand::ToggleTrackMute { track_id } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.muted = !track.muted;
            }
        }
        EditCommand::ToggleTrackSolo { track_id } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.solo = !track.solo;
            }
        }
        EditCommand::MoveTrack {
            from_index,
            to_index,
        } => {
            // Reverse: move from to_index back to from_index
            let actual_to = (*to_index).min(session.tracks.len().saturating_sub(1));
            if actual_to < session.tracks.len() {
                let track = session.tracks.remove(actual_to);
                let actual_from = (*from_index).min(session.tracks.len());
                session.tracks.insert(actual_from, track);
            }
        }
        EditCommand::SetInstrumentParam {
            track_id,
            param_index,
            old_value,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id)
                && let Some(val) = track.instrument_params.get_mut(*param_index)
            {
                *val = *old_value;
            }
        }
        EditCommand::CreateGroup { group_id, .. } => {
            // Undo create = remove
            session.remove_group(*group_id);
        }
        EditCommand::RemoveGroup { group, .. } => {
            // Undo remove = re-insert
            if let Some(g) = group {
                session.groups.push(g.clone());
            }
        }
        EditCommand::AddTrackToGroup { group_id, track_id } => {
            // Undo add = remove
            session.remove_track_from_group(*group_id, *track_id);
        }
        EditCommand::RemoveTrackFromGroup { group_id, track_id } => {
            // Undo remove = add
            session.add_track_to_group(*group_id, *track_id);
        }
        EditCommand::RenameGroup {
            group_id, old_name, ..
        } => {
            session.rename_group(*group_id, old_name.clone());
        }
        EditCommand::ToggleGroupCollapsed { group_id } => {
            session.toggle_group_collapsed(*group_id);
        }
        EditCommand::SetTrackOutput {
            track_id,
            old_output,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.routing.output = *old_output;
            }
        }
        EditCommand::SetSidechainInput {
            track_id,
            old_source,
            ..
        } => {
            if let Some(track) = session.track_mut(*track_id) {
                track.routing.sidechain_input = *old_source;
            }
        }
        EditCommand::Compound { commands } => {
            for sub in commands.iter().rev() {
                reverse_command(sub, session);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::EditCommand;
    use crate::region::Region;
    use crate::session::Session;

    fn make_session_with_track() -> (Session, crate::track::TrackId) {
        let mut session = Session::new("Test", 48000, 256);
        let track_id = session.add_audio_track("Track 1");
        (session, track_id)
    }

    fn make_region(pos: u64, duration: u64) -> Region {
        Region::new("audio.wav".into(), pos, 0, duration)
    }

    // ---------------------------------------------------------------
    // 1. UndoManager creation and default state
    // ---------------------------------------------------------------

    #[test]
    fn test_new_undo_manager() {
        let um = UndoManager::new(50);
        assert!(!um.can_undo());
        assert!(!um.can_redo());
        assert_eq!(um.undo_count(), 0);
        assert_eq!(um.redo_count(), 0);
    }

    #[test]
    fn test_default_undo_manager() {
        let um = UndoManager::default();
        assert!(!um.can_undo());
        assert!(!um.can_redo());
        assert_eq!(um.max_history, 1000);
    }

    // ---------------------------------------------------------------
    // 2. Execute a command and undo it
    // ---------------------------------------------------------------

    #[test]
    fn test_execute_add_region_then_undo() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(0, 48000);
        let region_id = region.id;

        um.execute(
            EditCommand::AddRegion {
                track_id,
                region: region.clone(),
            },
            &mut session,
        );

        assert!(um.can_undo());
        assert_eq!(um.undo_count(), 1);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 1);

        // Undo: region should be removed
        assert!(um.undo(&mut session));
        assert!(!um.can_undo());
        assert!(um.can_redo());
        assert!(session.track(track_id).unwrap().region(region_id).is_none());
        assert_eq!(session.track(track_id).unwrap().regions.len(), 0);
    }

    // ---------------------------------------------------------------
    // 3. Execute multiple commands and undo/redo cycle
    // ---------------------------------------------------------------

    #[test]
    fn test_multiple_undo_redo_cycle() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let r1 = make_region(0, 1000);
        let r2 = make_region(2000, 500);
        let r1_id = r1.id;
        let r2_id = r2.id;

        um.execute(
            EditCommand::AddRegion {
                track_id,
                region: r1,
            },
            &mut session,
        );
        um.execute(
            EditCommand::AddRegion {
                track_id,
                region: r2,
            },
            &mut session,
        );

        assert_eq!(um.undo_count(), 2);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 2);

        // Undo second add
        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 1);
        assert!(session.track(track_id).unwrap().region(r2_id).is_none());

        // Undo first add
        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 0);

        // Redo first add
        um.redo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 1);
        assert!(session.track(track_id).unwrap().region(r1_id).is_some());

        // Redo second add
        um.redo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 2);
    }

    // ---------------------------------------------------------------
    // 4. Redo after undo
    // ---------------------------------------------------------------

    #[test]
    fn test_redo_restores_state() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(100, 500);
        let region_id = region.id;

        um.execute(
            EditCommand::AddRegion {
                track_id,
                region: region.clone(),
            },
            &mut session,
        );

        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 0);

        um.redo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 1);
        assert!(session.track(track_id).unwrap().region(region_id).is_some());
    }

    #[test]
    fn test_redo_returns_false_when_empty() {
        let (mut session, _) = make_session_with_track();
        let mut um = UndoManager::new(100);
        assert!(!um.redo(&mut session));
    }

    #[test]
    fn test_undo_returns_false_when_empty() {
        let (mut session, _) = make_session_with_track();
        let mut um = UndoManager::new(100);
        assert!(!um.undo(&mut session));
    }

    // ---------------------------------------------------------------
    // 5. Redo stack cleared after new command
    // ---------------------------------------------------------------

    #[test]
    fn test_redo_stack_cleared_on_new_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let r1 = make_region(0, 1000);
        um.execute(
            EditCommand::AddRegion {
                track_id,
                region: r1,
            },
            &mut session,
        );

        um.undo(&mut session);
        assert!(um.can_redo());

        // Execute a new command -- should clear redo stack
        let r2 = make_region(5000, 1000);
        um.execute(
            EditCommand::AddRegion {
                track_id,
                region: r2,
            },
            &mut session,
        );

        assert!(!um.can_redo());
        assert_eq!(um.redo_count(), 0);
    }

    // ---------------------------------------------------------------
    // 6. EditCommand variants
    // ---------------------------------------------------------------

    #[test]
    fn test_remove_region_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(0, 1000);
        let region_id = region.id;
        session.track_mut(track_id).unwrap().add_region(region);

        um.execute(
            EditCommand::RemoveRegion {
                track_id,
                region_id,
                region: None,
            },
            &mut session,
        );

        assert_eq!(session.track(track_id).unwrap().regions.len(), 0);

        // Undo: region should reappear
        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 1);
        assert!(session.track(track_id).unwrap().region(region_id).is_some());

        // Redo: remove again
        um.redo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 0);
    }

    #[test]
    fn test_move_region_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(100, 500);
        let region_id = region.id;
        session.track_mut(track_id).unwrap().add_region(region);

        um.execute(
            EditCommand::MoveRegion {
                track_id,
                region_id,
                old_pos: 100,
                new_pos: 5000,
            },
            &mut session,
        );

        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .timeline_pos,
            5000
        );

        // Undo
        um.undo(&mut session);
        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .timeline_pos,
            100
        );

        // Redo
        um.redo(&mut session);
        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .timeline_pos,
            5000
        );
    }

    #[test]
    fn test_move_region_to_track_command() {
        let mut session = Session::new("Test", 48000, 256);
        let track_a = session.add_audio_track("Track A");
        let track_b = session.add_audio_track("Track B");
        let mut um = UndoManager::new(100);

        let region = make_region(100, 500);
        let region_id = region.id;
        session.track_mut(track_a).unwrap().add_region(region);

        um.execute(
            EditCommand::MoveRegionToTrack {
                from_track: track_a,
                to_track: track_b,
                region_id,
                old_pos: 100,
                new_pos: 2000,
                region: None,
            },
            &mut session,
        );

        // Region should now be on track B at position 2000
        assert_eq!(session.track(track_a).unwrap().regions.len(), 0);
        assert_eq!(session.track(track_b).unwrap().regions.len(), 1);
        assert_eq!(
            session
                .track(track_b)
                .unwrap()
                .region(region_id)
                .unwrap()
                .timeline_pos,
            2000
        );

        // Undo: region back on track A at original position
        um.undo(&mut session);
        assert_eq!(session.track(track_a).unwrap().regions.len(), 1);
        assert_eq!(session.track(track_b).unwrap().regions.len(), 0);
        assert_eq!(
            session
                .track(track_a)
                .unwrap()
                .region(region_id)
                .unwrap()
                .timeline_pos,
            100
        );

        // Redo
        um.redo(&mut session);
        assert_eq!(session.track(track_a).unwrap().regions.len(), 0);
        assert_eq!(session.track(track_b).unwrap().regions.len(), 1);
    }

    #[test]
    fn test_split_region_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(0, 1000);
        let region_id = region.id;
        session.track_mut(track_id).unwrap().add_region(region);

        um.execute(
            EditCommand::SplitRegion {
                track_id,
                region_id,
                split_frame: 400,
                original: None,
                left_id: None,
                right_id: None,
            },
            &mut session,
        );

        // Original region gone, replaced by two new ones
        let track = session.track(track_id).unwrap();
        assert!(track.region(region_id).is_none());
        assert_eq!(track.regions.len(), 2);

        // Undo: original restored, split pieces removed
        um.undo(&mut session);
        let track = session.track(track_id).unwrap();
        assert_eq!(track.regions.len(), 1);
        assert!(track.region(region_id).is_some());
        assert_eq!(track.region(region_id).unwrap().duration, 1000);

        // Redo: split again
        um.redo(&mut session);
        let track = session.track(track_id).unwrap();
        assert!(track.region(region_id).is_none());
        assert_eq!(track.regions.len(), 2);
    }

    #[test]
    fn test_trim_start_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(100, 1000);
        let region_id = region.id;
        session.track_mut(track_id).unwrap().add_region(region);

        um.execute(
            EditCommand::TrimStart {
                track_id,
                region_id,
                old_start: 100,
                old_offset: 0,
                old_duration: 1000,
                new_start: 300,
            },
            &mut session,
        );

        let r = session.track(track_id).unwrap().region(region_id).unwrap();
        assert_eq!(r.timeline_pos, 300);
        assert_eq!(r.source_offset, 200);
        assert_eq!(r.duration, 800);

        // Undo
        um.undo(&mut session);
        let r = session.track(track_id).unwrap().region(region_id).unwrap();
        assert_eq!(r.timeline_pos, 100);
        assert_eq!(r.source_offset, 0);
        assert_eq!(r.duration, 1000);
    }

    #[test]
    fn test_trim_end_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(0, 1000);
        let region_id = region.id;
        session.track_mut(track_id).unwrap().add_region(region);

        um.execute(
            EditCommand::TrimEnd {
                track_id,
                region_id,
                old_duration: 1000,
                new_end: 600,
            },
            &mut session,
        );

        let r = session.track(track_id).unwrap().region(region_id).unwrap();
        assert_eq!(r.duration, 600);

        // Undo
        um.undo(&mut session);
        let r = session.track(track_id).unwrap().region(region_id).unwrap();
        assert_eq!(r.duration, 1000);
    }

    #[test]
    fn test_set_fade_in_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(0, 1000);
        let region_id = region.id;
        session.track_mut(track_id).unwrap().add_region(region);

        um.execute(
            EditCommand::SetFadeIn {
                track_id,
                region_id,
                old_fade: 0,
                new_fade: 200,
            },
            &mut session,
        );

        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .fade_in,
            200
        );

        um.undo(&mut session);
        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .fade_in,
            0
        );
    }

    #[test]
    fn test_set_fade_out_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(0, 1000);
        let region_id = region.id;
        session.track_mut(track_id).unwrap().add_region(region);

        um.execute(
            EditCommand::SetFadeOut {
                track_id,
                region_id,
                old_fade: 0,
                new_fade: 150,
            },
            &mut session,
        );

        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .fade_out,
            150
        );

        um.undo(&mut session);
        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .fade_out,
            0
        );
    }

    #[test]
    fn test_set_region_gain_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let region = make_region(0, 1000);
        let region_id = region.id;
        session.track_mut(track_id).unwrap().add_region(region);

        um.execute(
            EditCommand::SetRegionGain {
                track_id,
                region_id,
                old_gain: 1.0,
                new_gain: 0.5,
            },
            &mut session,
        );

        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .gain,
            0.5
        );

        um.undo(&mut session);
        assert_eq!(
            session
                .track(track_id)
                .unwrap()
                .region(region_id)
                .unwrap()
                .gain,
            1.0
        );
    }

    #[test]
    fn test_set_track_gain_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        um.execute(
            EditCommand::SetTrackGain {
                track_id,
                old_gain: 1.0,
                new_gain: 0.75,
            },
            &mut session,
        );

        assert_eq!(session.track(track_id).unwrap().gain, 0.75);

        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().gain, 1.0);

        um.redo(&mut session);
        assert_eq!(session.track(track_id).unwrap().gain, 0.75);
    }

    #[test]
    fn test_set_track_pan_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        um.execute(
            EditCommand::SetTrackPan {
                track_id,
                old_pan: 0.0,
                new_pan: -0.5,
            },
            &mut session,
        );

        assert_eq!(session.track(track_id).unwrap().pan, -0.5);

        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().pan, 0.0);

        um.redo(&mut session);
        assert_eq!(session.track(track_id).unwrap().pan, -0.5);
    }

    #[test]
    fn test_toggle_track_mute_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        assert!(!session.track(track_id).unwrap().muted);

        um.execute(EditCommand::ToggleTrackMute { track_id }, &mut session);
        assert!(session.track(track_id).unwrap().muted);

        um.undo(&mut session);
        assert!(!session.track(track_id).unwrap().muted);

        um.redo(&mut session);
        assert!(session.track(track_id).unwrap().muted);
    }

    #[test]
    fn test_toggle_track_solo_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        assert!(!session.track(track_id).unwrap().solo);

        um.execute(EditCommand::ToggleTrackSolo { track_id }, &mut session);
        assert!(session.track(track_id).unwrap().solo);

        um.undo(&mut session);
        assert!(!session.track(track_id).unwrap().solo);

        um.redo(&mut session);
        assert!(session.track(track_id).unwrap().solo);
    }

    #[test]
    fn test_compound_command() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let r1 = make_region(0, 1000);
        let r2 = make_region(2000, 500);
        let r1_id = r1.id;
        let r2_id = r2.id;

        um.execute(
            EditCommand::Compound {
                commands: vec![
                    EditCommand::AddRegion {
                        track_id,
                        region: r1,
                    },
                    EditCommand::AddRegion {
                        track_id,
                        region: r2,
                    },
                ],
            },
            &mut session,
        );

        assert_eq!(session.track(track_id).unwrap().regions.len(), 2);
        assert_eq!(um.undo_count(), 1); // compound counts as one

        // Undo: both removed
        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 0);

        // Redo: both restored
        um.redo(&mut session);
        assert_eq!(session.track(track_id).unwrap().regions.len(), 2);
        assert!(session.track(track_id).unwrap().region(r1_id).is_some());
        assert!(session.track(track_id).unwrap().region(r2_id).is_some());
    }

    // ---------------------------------------------------------------
    // 7. Undo history limit
    // ---------------------------------------------------------------

    #[test]
    fn test_history_limit_enforced() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(3);

        for _ in 0..5 {
            let r = make_region(0, 100);
            um.execute(
                EditCommand::AddRegion {
                    track_id,
                    region: r,
                },
                &mut session,
            );
        }

        // Only 3 commands retained even though we executed 5
        assert_eq!(um.undo_count(), 3);
    }

    #[test]
    fn test_history_limit_oldest_removed() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(2);

        // Execute 3 track-gain commands with distinct values
        um.execute(
            EditCommand::SetTrackGain {
                track_id,
                old_gain: 1.0,
                new_gain: 0.9,
            },
            &mut session,
        );
        um.execute(
            EditCommand::SetTrackGain {
                track_id,
                old_gain: 0.9,
                new_gain: 0.8,
            },
            &mut session,
        );
        um.execute(
            EditCommand::SetTrackGain {
                track_id,
                old_gain: 0.8,
                new_gain: 0.7,
            },
            &mut session,
        );

        assert_eq!(um.undo_count(), 2);

        // Undo twice: should restore to 0.9 (the first command was evicted)
        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().gain, 0.8);
        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().gain, 0.9);

        // No more undos
        assert!(!um.can_undo());
    }

    // ---------------------------------------------------------------
    // 8. can_undo() / can_redo() states
    // ---------------------------------------------------------------

    #[test]
    fn test_can_undo_can_redo_transitions() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        assert!(!um.can_undo());
        assert!(!um.can_redo());

        let r = make_region(0, 100);
        um.execute(
            EditCommand::AddRegion {
                track_id,
                region: r,
            },
            &mut session,
        );
        assert!(um.can_undo());
        assert!(!um.can_redo());

        um.undo(&mut session);
        assert!(!um.can_undo());
        assert!(um.can_redo());

        um.redo(&mut session);
        assert!(um.can_undo());
        assert!(!um.can_redo());
    }

    // ---------------------------------------------------------------
    // Extra: clear()
    // ---------------------------------------------------------------

    #[test]
    fn test_clear() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(100);

        let r = make_region(0, 100);
        um.execute(
            EditCommand::AddRegion {
                track_id,
                region: r,
            },
            &mut session,
        );
        um.undo(&mut session);

        assert!(um.can_redo());
        um.clear();
        assert!(!um.can_undo());
        assert!(!um.can_redo());
        assert_eq!(um.undo_count(), 0);
        assert_eq!(um.redo_count(), 0);
    }

    #[test]
    fn test_move_track_undo_redo() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        session.add_audio_track("B");
        session.add_audio_track("C");
        let mut undo = UndoManager::default();

        undo.execute(
            EditCommand::MoveTrack {
                from_index: 0,
                to_index: 2,
            },
            &mut session,
        );
        assert_eq!(session.tracks[0].name, "B");
        assert_eq!(session.tracks[2].name, "A");

        undo.undo(&mut session);
        assert_eq!(session.tracks[0].name, "A");
        assert_eq!(session.tracks[2].name, "C");

        undo.redo(&mut session);
        assert_eq!(session.tracks[0].name, "B");
    }

    #[test]
    fn test_move_track_undo_restores_exact_order() {
        let mut session = Session::new("Test", 48000, 512);
        session.add_audio_track("A");
        session.add_audio_track("B");
        session.add_audio_track("C");
        session.add_audio_track("D");
        // Order: A(0), B(1), C(2), D(3), Master(4)
        let mut undo = UndoManager::default();

        // Move track from index 0 to index 3
        undo.execute(
            EditCommand::MoveTrack {
                from_index: 0,
                to_index: 3,
            },
            &mut session,
        );
        // After: B(0), C(1), D(2), A(3), Master(4)
        assert_eq!(session.tracks[0].name, "B");
        assert_eq!(session.tracks[1].name, "C");
        assert_eq!(session.tracks[2].name, "D");
        assert_eq!(session.tracks[3].name, "A");

        // Undo: should restore exact original order
        undo.undo(&mut session);
        assert_eq!(session.tracks[0].name, "A");
        assert_eq!(session.tracks[1].name, "B");
        assert_eq!(session.tracks[2].name, "C");
        assert_eq!(session.tracks[3].name, "D");
        assert_eq!(session.tracks[4].name, "Master");
    }

    #[test]
    fn test_set_instrument_param_undo_redo() {
        let mut session = Session::new("Test", 48000, 256);
        let track_id = session.add_instrument_track("Synth", Some("SubtractiveSynth".to_string()));
        // Set up some instrument params on the track
        session.track_mut(track_id).unwrap().instrument_params = vec![0.5, 0.8, 1.0];

        let mut um = UndoManager::new(100);

        um.execute(
            EditCommand::SetInstrumentParam {
                track_id,
                param_index: 1,
                old_value: 0.8,
                new_value: 0.3,
            },
            &mut session,
        );

        assert!((session.track(track_id).unwrap().instrument_params[1] - 0.3).abs() < f32::EPSILON);

        // Undo
        um.undo(&mut session);
        assert!((session.track(track_id).unwrap().instrument_params[1] - 0.8).abs() < f32::EPSILON);

        // Redo
        um.redo(&mut session);
        assert!((session.track(track_id).unwrap().instrument_params[1] - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_instrument_param_out_of_bounds_is_noop() {
        let mut session = Session::new("Test", 48000, 256);
        let track_id = session.add_instrument_track("Synth", Some("SubtractiveSynth".to_string()));
        session.track_mut(track_id).unwrap().instrument_params = vec![0.5];

        let mut um = UndoManager::new(100);

        // param_index 5 is out of bounds — should be a no-op, not panic
        um.execute(
            EditCommand::SetInstrumentParam {
                track_id,
                param_index: 5,
                old_value: 0.0,
                new_value: 1.0,
            },
            &mut session,
        );

        // Original params unchanged
        assert!((session.track(track_id).unwrap().instrument_params[0] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_move_track_compound_with_other_edits() {
        let mut session = Session::new("Test", 48000, 512);
        let track_id = session.add_audio_track("A");
        session.add_audio_track("B");
        session.add_audio_track("C");
        // Order: A(0), B(1), C(2), Master(3)
        let mut undo = UndoManager::default();

        let region = make_region(0, 1000);
        let region_id = region.id;

        // Compound: add a region AND move a track
        undo.execute(
            EditCommand::Compound {
                commands: vec![
                    EditCommand::AddRegion {
                        track_id,
                        region: region.clone(),
                    },
                    EditCommand::MoveTrack {
                        from_index: 0,
                        to_index: 2,
                    },
                ],
            },
            &mut session,
        );

        // After compound: region added, track moved
        // Order: B(0), C(1), A(2), Master(3)
        assert_eq!(session.tracks[0].name, "B");
        assert_eq!(session.tracks[2].name, "A");
        assert_eq!(session.track(track_id).unwrap().regions.len(), 1);

        // Undo the entire compound
        undo.undo(&mut session);

        // Track order restored: A(0), B(1), C(2), Master(3)
        assert_eq!(session.tracks[0].name, "A");
        assert_eq!(session.tracks[1].name, "B");
        assert_eq!(session.tracks[2].name, "C");
        // Region removed
        assert!(session.track(track_id).unwrap().region(region_id).is_none());
        assert_eq!(session.track(track_id).unwrap().regions.len(), 0);
    }

    // ---------------------------------------------------------------
    // Track group undo/redo
    // ---------------------------------------------------------------

    #[test]
    fn test_create_group_undo_redo() {
        let mut session = Session::new("Test", 48000, 256);
        let mut um = UndoManager::new(100);

        let gid = crate::track::TrackGroupId::new();
        um.execute(
            EditCommand::CreateGroup {
                group_id: gid,
                name: "Drums".to_string(),
                group: None,
            },
            &mut session,
        );
        assert_eq!(session.groups.len(), 1);
        assert_eq!(session.group(gid).unwrap().name, "Drums");

        um.undo(&mut session);
        assert!(session.groups.is_empty());

        um.redo(&mut session);
        assert_eq!(session.groups.len(), 1);
        assert_eq!(session.group(gid).unwrap().name, "Drums");
    }

    #[test]
    fn test_remove_group_undo_redo() {
        let mut session = Session::new("Test", 48000, 256);
        let mut um = UndoManager::new(100);

        let gid = session.add_group("Vocals");
        let t1 = session.add_audio_track("Lead");
        session.add_track_to_group(gid, t1);

        um.execute(
            EditCommand::RemoveGroup {
                group_id: gid,
                group: None,
            },
            &mut session,
        );
        assert!(session.groups.is_empty());

        um.undo(&mut session);
        assert_eq!(session.groups.len(), 1);
        assert_eq!(session.group(gid).unwrap().tracks.len(), 1);

        um.redo(&mut session);
        assert!(session.groups.is_empty());
    }

    #[test]
    fn test_add_track_to_group_undo_redo() {
        let mut session = Session::new("Test", 48000, 256);
        let mut um = UndoManager::new(100);

        let gid = session.add_group("FX");
        let t1 = session.add_audio_track("Reverb");

        um.execute(
            EditCommand::AddTrackToGroup {
                group_id: gid,
                track_id: t1,
            },
            &mut session,
        );
        assert_eq!(session.group(gid).unwrap().tracks.len(), 1);

        um.undo(&mut session);
        assert!(session.group(gid).unwrap().tracks.is_empty());

        um.redo(&mut session);
        assert_eq!(session.group(gid).unwrap().tracks.len(), 1);
    }

    #[test]
    fn test_remove_track_from_group_undo_redo() {
        let mut session = Session::new("Test", 48000, 256);
        let mut um = UndoManager::new(100);

        let gid = session.add_group("Drums");
        let t1 = session.add_audio_track("Kick");
        session.add_track_to_group(gid, t1);

        um.execute(
            EditCommand::RemoveTrackFromGroup {
                group_id: gid,
                track_id: t1,
            },
            &mut session,
        );
        assert!(session.group(gid).unwrap().tracks.is_empty());

        um.undo(&mut session);
        assert_eq!(session.group(gid).unwrap().tracks.len(), 1);
    }

    #[test]
    fn test_rename_group_undo_redo() {
        let mut session = Session::new("Test", 48000, 256);
        let mut um = UndoManager::new(100);

        let gid = session.add_group("Old");

        um.execute(
            EditCommand::RenameGroup {
                group_id: gid,
                old_name: "Old".to_string(),
                new_name: "New".to_string(),
            },
            &mut session,
        );
        assert_eq!(session.group(gid).unwrap().name, "New");

        um.undo(&mut session);
        assert_eq!(session.group(gid).unwrap().name, "Old");

        um.redo(&mut session);
        assert_eq!(session.group(gid).unwrap().name, "New");
    }

    #[test]
    fn test_toggle_group_collapsed_undo() {
        let mut session = Session::new("Test", 48000, 256);
        let mut um = UndoManager::new(100);

        let gid = session.add_group("G");
        assert!(!session.group(gid).unwrap().collapsed);

        um.execute(
            EditCommand::ToggleGroupCollapsed { group_id: gid },
            &mut session,
        );
        assert!(session.group(gid).unwrap().collapsed);

        um.undo(&mut session);
        assert!(!session.group(gid).unwrap().collapsed);
    }

    // ---------------------------------------------------------------
    // Output routing undo/redo
    // ---------------------------------------------------------------

    #[test]
    fn test_set_track_output_undo_redo() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("Bus");
        let mut um = UndoManager::new(100);

        um.execute(
            EditCommand::SetTrackOutput {
                track_id: audio_id,
                old_output: None,
                new_output: Some(bus_id),
            },
            &mut session,
        );
        assert_eq!(
            session.track(audio_id).unwrap().routing.output,
            Some(bus_id)
        );

        um.undo(&mut session);
        assert!(session.track(audio_id).unwrap().routing.output.is_none());

        um.redo(&mut session);
        assert_eq!(
            session.track(audio_id).unwrap().routing.output,
            Some(bus_id)
        );
    }

    #[test]
    fn test_set_sidechain_input_undo_redo() {
        let mut session = Session::new("Test", 48000, 256);
        let bass = session.add_audio_track("Bass");
        let vocal = session.add_audio_track("Vocal");
        let mut um = UndoManager::new(100);

        um.execute(
            EditCommand::SetSidechainInput {
                track_id: bass,
                old_source: None,
                new_source: Some(vocal),
            },
            &mut session,
        );
        assert_eq!(
            session.track(bass).unwrap().routing.sidechain_input,
            Some(vocal)
        );

        um.undo(&mut session);
        assert!(
            session
                .track(bass)
                .unwrap()
                .routing
                .sidechain_input
                .is_none()
        );

        um.redo(&mut session);
        assert_eq!(
            session.track(bass).unwrap().routing.sidechain_input,
            Some(vocal)
        );
    }

    #[test]
    fn test_routing_compound_undo() {
        let mut session = Session::new("Test", 48000, 256);
        let audio_id = session.add_audio_track("Guitar");
        let bus_id = session.add_bus_track("Bus");
        let vocal = session.add_audio_track("Vocal");
        let mut um = UndoManager::new(100);

        um.execute(
            EditCommand::Compound {
                commands: vec![
                    EditCommand::SetTrackOutput {
                        track_id: audio_id,
                        old_output: None,
                        new_output: Some(bus_id),
                    },
                    EditCommand::SetSidechainInput {
                        track_id: audio_id,
                        old_source: None,
                        new_source: Some(vocal),
                    },
                ],
            },
            &mut session,
        );

        assert_eq!(
            session.track(audio_id).unwrap().routing.output,
            Some(bus_id)
        );
        assert_eq!(
            session.track(audio_id).unwrap().routing.sidechain_input,
            Some(vocal)
        );

        um.undo(&mut session);
        assert!(session.track(audio_id).unwrap().routing.output.is_none());
        assert!(
            session
                .track(audio_id)
                .unwrap()
                .routing
                .sidechain_input
                .is_none()
        );
    }

    #[test]
    fn test_vecdeque_eviction_oldest_first() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(3); // max 3 entries

        // Execute 5 commands — first 2 should be evicted
        for i in 0..5 {
            um.execute(
                EditCommand::SetTrackGain {
                    track_id,
                    old_gain: i as f32,
                    new_gain: (i + 1) as f32,
                },
                &mut session,
            );
        }

        assert_eq!(um.undo_count(), 3);
        // We should be able to undo exactly 3 times
        assert!(um.undo(&mut session));
        assert!(um.undo(&mut session));
        assert!(um.undo(&mut session));
        assert!(!um.undo(&mut session)); // no more
    }

    #[test]
    fn test_vecdeque_eviction_preserves_order() {
        let (mut session, track_id) = make_session_with_track();
        let mut um = UndoManager::new(2);

        um.execute(
            EditCommand::SetTrackGain {
                track_id,
                old_gain: 1.0,
                new_gain: 2.0,
            },
            &mut session,
        );
        um.execute(
            EditCommand::SetTrackGain {
                track_id,
                old_gain: 2.0,
                new_gain: 3.0,
            },
            &mut session,
        );
        um.execute(
            EditCommand::SetTrackGain {
                track_id,
                old_gain: 3.0,
                new_gain: 4.0,
            },
            &mut session,
        );

        // First command evicted; current gain is 4.0
        assert_eq!(session.track(track_id).unwrap().gain, 4.0);

        // Undo last: 4.0 -> 3.0
        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().gain, 3.0);

        // Undo second-to-last: 3.0 -> 2.0
        um.undo(&mut session);
        assert_eq!(session.track(track_id).unwrap().gain, 2.0);

        // Cannot undo further (first command was evicted)
        assert!(!um.can_undo());
    }
}
