use serde::{Deserialize, Serialize};

use crate::region::{Region, RegionId};
use crate::take::{Take, TakeId};
use crate::track::{TrackGroup, TrackGroupId, TrackId};
use crate::types::{FramePos, TrackSlot};

/// An editing command that can be applied and undone.
///
/// Heavy data (Region, TrackGroup) is boxed to keep the enum small.
/// This reduces memory per undo entry — lightweight variants (trims,
/// gain changes, toggles) no longer pay the size cost of the largest variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditCommand {
    /// Add a region to a track.
    AddRegion {
        track_id: TrackId,
        region: Box<Region>,
    },
    /// Remove a region from a track.
    RemoveRegion {
        track_id: TrackId,
        region_id: RegionId,
        /// The removed region, stored for undo.
        region: Option<Box<Region>>,
    },
    /// Move a region to a new timeline position.
    MoveRegion {
        track_id: TrackId,
        region_id: RegionId,
        old_pos: FramePos,
        new_pos: FramePos,
    },
    /// Move a region from one track to another.
    MoveRegionToTrack {
        from_track: TrackId,
        to_track: TrackId,
        region_id: RegionId,
        old_pos: FramePos,
        new_pos: FramePos,
        /// Stored for undo.
        region: Option<Box<Region>>,
    },
    /// Split a region at a frame position, replacing it with two new regions.
    SplitRegion {
        track_id: TrackId,
        region_id: RegionId,
        split_frame: FramePos,
        /// The original region before split, stored for undo.
        original: Option<Box<Region>>,
        /// The two resulting regions after split.
        left_id: Option<RegionId>,
        right_id: Option<RegionId>,
    },
    /// Trim the start of a region.
    TrimStart {
        track_id: TrackId,
        region_id: RegionId,
        old_start: FramePos,
        old_offset: FramePos,
        old_duration: FramePos,
        new_start: FramePos,
    },
    /// Trim the end of a region.
    TrimEnd {
        track_id: TrackId,
        region_id: RegionId,
        old_duration: FramePos,
        new_end: FramePos,
    },
    /// Set fade-in duration on a region.
    SetFadeIn {
        track_id: TrackId,
        region_id: RegionId,
        old_fade: FramePos,
        new_fade: FramePos,
    },
    /// Set fade-out duration on a region.
    SetFadeOut {
        track_id: TrackId,
        region_id: RegionId,
        old_fade: FramePos,
        new_fade: FramePos,
    },
    /// Set region gain.
    SetRegionGain {
        track_id: TrackId,
        region_id: RegionId,
        old_gain: f32,
        new_gain: f32,
    },
    /// Set track gain.
    SetTrackGain {
        track_id: TrackId,
        old_gain: f32,
        new_gain: f32,
    },
    /// Set track pan.
    SetTrackPan {
        track_id: TrackId,
        old_pan: f32,
        new_pan: f32,
    },
    /// Toggle track mute.
    ToggleTrackMute { track_id: TrackId },
    /// Toggle track solo.
    ToggleTrackSolo { track_id: TrackId },
    /// Move a track from one index to another.
    MoveTrack {
        from_index: TrackSlot,
        to_index: TrackSlot,
    },
    /// Set an instrument parameter on a track by index.
    SetInstrumentParam {
        track_id: TrackId,
        param_index: usize,
        old_value: f32,
        new_value: f32,
    },
    /// Create a new track group.
    CreateGroup {
        group_id: TrackGroupId,
        name: String,
        /// Stored for undo (the full group after creation).
        group: Option<Box<TrackGroup>>,
    },
    /// Remove a track group.
    RemoveGroup {
        group_id: TrackGroupId,
        /// Stored for undo (the removed group with its members).
        group: Option<Box<TrackGroup>>,
    },
    /// Add a track to a group.
    AddTrackToGroup {
        group_id: TrackGroupId,
        track_id: TrackId,
    },
    /// Remove a track from a group.
    RemoveTrackFromGroup {
        group_id: TrackGroupId,
        track_id: TrackId,
    },
    /// Rename a track group.
    RenameGroup {
        group_id: TrackGroupId,
        old_name: String,
        new_name: String,
    },
    /// Toggle a group's collapsed state.
    ToggleGroupCollapsed { group_id: TrackGroupId },
    /// Set a track's output routing target.
    SetTrackOutput {
        track_id: TrackId,
        old_output: Option<TrackId>,
        new_output: Option<TrackId>,
    },
    /// Set a track's sidechain input source.
    SetSidechainInput {
        track_id: TrackId,
        old_source: Option<TrackId>,
        new_source: Option<TrackId>,
    },
    /// Set the active take in a take stack.
    SetActiveTake {
        track_id: TrackId,
        stack_index: usize,
        old_active: usize,
        new_active: usize,
    },
    /// Mute or unmute a take in a take stack.
    MuteTake {
        track_id: TrackId,
        stack_index: usize,
        take_id: TakeId,
        was_muted: bool,
    },
    /// Delete a take from a take stack.
    DeleteTake {
        track_id: TrackId,
        stack_index: usize,
        take: Box<Take>,
        was_at_index: usize,
    },
    /// Compound command (multiple edits as one undoable action).
    Compound { commands: Vec<EditCommand> },
}
