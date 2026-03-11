use crate::edit::EditCommand;
use crate::session::Session;

/// Manages undo/redo history using the command pattern.
pub struct UndoManager {
    undo_stack: Vec<EditCommand>,
    redo_stack: Vec<EditCommand>,
    max_history: usize,
}

impl UndoManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Execute a command, pushing it onto the undo stack.
    pub fn execute(&mut self, mut cmd: EditCommand, session: &mut Session) {
        apply_command(&mut cmd, session);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();

        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last command.
    pub fn undo(&mut self, session: &mut Session) -> bool {
        if let Some(cmd) = self.undo_stack.pop() {
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
            self.undo_stack.push(cmd);
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
        EditCommand::Compound { commands } => {
            for sub in commands.iter().rev() {
                reverse_command(sub, session);
            }
        }
    }
}
