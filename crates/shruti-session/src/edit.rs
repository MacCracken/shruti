use serde::{Deserialize, Serialize};

use crate::region::{Region, RegionId};
use crate::track::{TrackGroup, TrackGroupId, TrackId};

/// An editing command that can be applied and undone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditCommand {
    /// Add a region to a track.
    AddRegion { track_id: TrackId, region: Region },
    /// Remove a region from a track.
    RemoveRegion {
        track_id: TrackId,
        region_id: RegionId,
        /// The removed region, stored for undo.
        region: Option<Region>,
    },
    /// Move a region to a new timeline position.
    MoveRegion {
        track_id: TrackId,
        region_id: RegionId,
        old_pos: u64,
        new_pos: u64,
    },
    /// Move a region from one track to another.
    MoveRegionToTrack {
        from_track: TrackId,
        to_track: TrackId,
        region_id: RegionId,
        old_pos: u64,
        new_pos: u64,
        /// Stored for undo.
        region: Option<Region>,
    },
    /// Split a region at a frame position, replacing it with two new regions.
    SplitRegion {
        track_id: TrackId,
        region_id: RegionId,
        split_frame: u64,
        /// The original region before split, stored for undo.
        original: Option<Region>,
        /// The two resulting regions after split.
        left_id: Option<RegionId>,
        right_id: Option<RegionId>,
    },
    /// Trim the start of a region.
    TrimStart {
        track_id: TrackId,
        region_id: RegionId,
        old_start: u64,
        old_offset: u64,
        old_duration: u64,
        new_start: u64,
    },
    /// Trim the end of a region.
    TrimEnd {
        track_id: TrackId,
        region_id: RegionId,
        old_duration: u64,
        new_end: u64,
    },
    /// Set fade-in duration on a region.
    SetFadeIn {
        track_id: TrackId,
        region_id: RegionId,
        old_fade: u64,
        new_fade: u64,
    },
    /// Set fade-out duration on a region.
    SetFadeOut {
        track_id: TrackId,
        region_id: RegionId,
        old_fade: u64,
        new_fade: u64,
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
    MoveTrack { from_index: usize, to_index: usize },
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
        group: Option<TrackGroup>,
    },
    /// Remove a track group.
    RemoveGroup {
        group_id: TrackGroupId,
        /// Stored for undo (the removed group with its members).
        group: Option<TrackGroup>,
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
    /// Compound command (multiple edits as one undoable action).
    Compound { commands: Vec<EditCommand> },
}
