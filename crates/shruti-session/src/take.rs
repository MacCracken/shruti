//! Take and layer management for loop recording.
//!
//! A `TakeStack` groups multiple audio takes recorded at the same timeline
//! position (typically from loop-based overdub recording). Each take references
//! an audio file in the pool and can be individually muted/soloed.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::region::Region;
use crate::types::FramePos;

/// Unique identifier for a take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TakeId(pub Uuid);

impl TakeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TakeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TakeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single recorded take within a take stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Take {
    /// Unique take identifier.
    pub id: TakeId,
    /// Reference to the audio file in the audio pool.
    pub audio_file_id: String,
    /// Loop iteration number (0-indexed).
    pub iteration: u32,
    /// Whether this take is muted.
    pub muted: bool,
}

impl Take {
    pub fn new(audio_file_id: String, iteration: u32) -> Self {
        Self {
            id: TakeId::new(),
            audio_file_id,
            iteration,
            muted: false,
        }
    }
}

/// A stack of takes recorded at the same timeline position.
///
/// Typically created by loop-aware overdub recording where each loop
/// iteration produces a new take layered on top of previous ones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeStack {
    /// All takes in this stack, ordered by iteration.
    pub takes: Vec<Take>,
    /// Index of the currently active (selected) take for editing.
    pub active_index: usize,
    /// Timeline position where this stack starts.
    pub timeline_pos: FramePos,
    /// Duration of each take (typically one loop length).
    pub duration: FramePos,
}

impl TakeStack {
    /// Create a new empty take stack at the given position.
    pub fn new(timeline_pos: FramePos, duration: FramePos) -> Self {
        Self {
            takes: Vec::new(),
            active_index: 0,
            timeline_pos,
            duration,
        }
    }

    /// Add a new take to the stack. Returns the take's ID.
    pub fn add_take(&mut self, audio_file_id: String, iteration: u32) -> TakeId {
        let take = Take::new(audio_file_id, iteration);
        let id = take.id;
        self.takes.push(take);
        // New take becomes active
        self.active_index = self.takes.len() - 1;
        id
    }

    /// Get the currently active take.
    pub fn active_take(&self) -> Option<&Take> {
        self.takes.get(self.active_index)
    }

    /// Set the active take by index.
    pub fn set_active(&mut self, index: usize) {
        if index < self.takes.len() {
            self.active_index = index;
        }
    }

    /// Mute a specific take by ID. Returns true if found.
    pub fn mute_take(&mut self, id: TakeId) -> bool {
        if let Some(take) = self.takes.iter_mut().find(|t| t.id == id) {
            take.muted = true;
            true
        } else {
            false
        }
    }

    /// Unmute a specific take by ID. Returns true if found.
    pub fn unmute_take(&mut self, id: TakeId) -> bool {
        if let Some(take) = self.takes.iter_mut().find(|t| t.id == id) {
            take.muted = false;
            true
        } else {
            false
        }
    }

    /// Solo a take: unmute it and mute all others.
    pub fn solo_take(&mut self, id: TakeId) -> bool {
        let found = self.takes.iter().any(|t| t.id == id);
        if found {
            for take in &mut self.takes {
                take.muted = take.id != id;
            }
            true
        } else {
            false
        }
    }

    /// Delete a take by ID. Returns the removed take if found.
    pub fn delete_take(&mut self, id: TakeId) -> Option<Take> {
        if let Some(pos) = self.takes.iter().position(|t| t.id == id) {
            let take = self.takes.remove(pos);
            // Adjust active_index if needed
            if self.active_index >= self.takes.len() && !self.takes.is_empty() {
                self.active_index = self.takes.len() - 1;
            }
            Some(take)
        } else {
            None
        }
    }

    /// Get the audible take — the active take if it's not muted.
    /// Falls back to the first non-muted take.
    pub fn audible_take(&self) -> Option<&Take> {
        // Try active take first
        if let Some(active) = self.active_take()
            && !active.muted
        {
            return Some(active);
        }
        // Fall back to first non-muted
        self.takes.iter().find(|t| !t.muted)
    }

    /// Number of takes in this stack.
    pub fn len(&self) -> usize {
        self.takes.len()
    }

    /// Whether this stack is empty.
    pub fn is_empty(&self) -> bool {
        self.takes.is_empty()
    }
}

/// A section of a comp: a time range assigned to a specific take.
///
/// `offset` is relative to the take stack's `timeline_pos` (0 = stack start).
/// `duration` is how long this section lasts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompSection {
    /// Offset from the take stack's start (in frames).
    pub offset: FramePos,
    /// Duration of this section (in frames).
    pub duration: FramePos,
    /// Index of the take to use for this section.
    pub take_index: usize,
}

impl TakeStack {
    /// Build a composite from a list of sections, each assigned to a different take.
    ///
    /// Returns a list of `Region` objects that together form the comp.
    /// Each region points to the audio file of the assigned take, with the
    /// correct `source_offset` and `timeline_pos`.
    ///
    /// Sections must not overlap and should be sorted by offset.
    /// Sections referencing invalid take indices are skipped.
    pub fn build_comp(&self, sections: &[CompSection]) -> Vec<Region> {
        let mut regions = Vec::with_capacity(sections.len());

        for section in sections {
            let take = match self.takes.get(section.take_index) {
                Some(t) => t,
                None => continue,
            };

            let region = Region::new(
                take.audio_file_id.clone(),
                self.timeline_pos + section.offset,
                section.offset,
                section.duration,
            );
            regions.push(region);
        }

        regions
    }

    /// Build a simple comp that uses the active take for the entire duration.
    ///
    /// Equivalent to calling `build_comp` with a single section covering
    /// the full stack duration, assigned to the active take.
    pub fn build_comp_from_active(&self) -> Option<Region> {
        let take = self.active_take()?;
        Some(Region::new(
            take.audio_file_id.clone(),
            self.timeline_pos,
            FramePos(0),
            self.duration,
        ))
    }

    /// Build a two-section comp by splitting at a frame offset.
    ///
    /// `split_offset` is relative to the stack start. The left section uses
    /// `left_take_index` and the right section uses `right_take_index`.
    pub fn build_comp_split(
        &self,
        split_offset: FramePos,
        left_take_index: usize,
        right_take_index: usize,
    ) -> Vec<Region> {
        if split_offset >= self.duration || split_offset == FramePos(0) {
            // Degenerate: no actual split, return single region
            let idx = if split_offset == FramePos(0) {
                right_take_index
            } else {
                left_take_index
            };
            let sections = [CompSection {
                offset: FramePos(0),
                duration: self.duration,
                take_index: idx,
            }];
            return self.build_comp(&sections);
        }

        let right_duration = self.duration - split_offset;
        let sections = [
            CompSection {
                offset: FramePos(0),
                duration: split_offset,
                take_index: left_take_index,
            },
            CompSection {
                offset: split_offset,
                duration: right_duration,
                take_index: right_take_index,
            },
        ];
        self.build_comp(&sections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_id_is_unique() {
        let a = TakeId::new();
        let b = TakeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn empty_stack() {
        let stack = TakeStack::new(FramePos(0), FramePos(44100));
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
        assert!(stack.active_take().is_none());
        assert!(stack.audible_take().is_none());
    }

    #[test]
    fn add_take_becomes_active() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        let id1 = stack.add_take("take1.wav".into(), 0);
        assert_eq!(stack.active_index, 0);
        assert_eq!(stack.active_take().unwrap().id, id1);

        let id2 = stack.add_take("take2.wav".into(), 1);
        assert_eq!(stack.active_index, 1);
        assert_eq!(stack.active_take().unwrap().id, id2);
    }

    #[test]
    fn set_active() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        let id2 = stack.add_take("take2.wav".into(), 1);
        stack.add_take("take3.wav".into(), 2);

        stack.set_active(1);
        assert_eq!(stack.active_take().unwrap().id, id2);
    }

    #[test]
    fn mute_and_unmute() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        let id = stack.add_take("take1.wav".into(), 0);

        assert!(!stack.takes[0].muted);
        stack.mute_take(id);
        assert!(stack.takes[0].muted);
        stack.unmute_take(id);
        assert!(!stack.takes[0].muted);
    }

    #[test]
    fn solo_mutes_others() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        let id2 = stack.add_take("take2.wav".into(), 1);
        stack.add_take("take3.wav".into(), 2);

        stack.solo_take(id2);
        assert!(stack.takes[0].muted);
        assert!(!stack.takes[1].muted);
        assert!(stack.takes[2].muted);
    }

    #[test]
    fn delete_take() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        let id1 = stack.add_take("take1.wav".into(), 0);
        stack.add_take("take2.wav".into(), 1);

        let deleted = stack.delete_take(id1);
        assert!(deleted.is_some());
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.takes[0].audio_file_id, "take2.wav");
    }

    #[test]
    fn delete_adjusts_active_index() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        let id2 = stack.add_take("take2.wav".into(), 1);
        // active_index is 1

        stack.delete_take(id2);
        assert_eq!(stack.active_index, 0);
    }

    #[test]
    fn audible_take_returns_active_if_unmuted() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        let id = stack.add_take("take1.wav".into(), 0);
        assert_eq!(stack.audible_take().unwrap().id, id);
    }

    #[test]
    fn audible_take_skips_muted_active() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        let id1 = stack.add_take("take1.wav".into(), 0);
        let id2 = stack.add_take("take2.wav".into(), 1);

        // Active is take2, mute it
        stack.mute_take(id2);
        // Should fall back to take1
        assert_eq!(stack.audible_take().unwrap().id, id1);
    }

    #[test]
    fn audible_take_all_muted() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        let id1 = stack.add_take("take1.wav".into(), 0);
        let id2 = stack.add_take("take2.wav".into(), 1);
        stack.mute_take(id1);
        stack.mute_take(id2);
        assert!(stack.audible_take().is_none());
    }

    #[test]
    fn delete_nonexistent_returns_none() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        let fake_id = TakeId::new();
        assert!(stack.delete_take(fake_id).is_none());
    }

    #[test]
    fn take_stack_serializes() {
        let mut stack = TakeStack::new(FramePos(1000), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        stack.add_take("take2.wav".into(), 1);

        let json = serde_json::to_string(&stack).unwrap();
        let roundtrip: TakeStack = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.len(), 2);
        assert_eq!(roundtrip.takes[0].audio_file_id, "take1.wav");
        assert_eq!(roundtrip.takes[1].iteration, 1);
    }

    // --- Comp editing tests ---

    #[test]
    fn build_comp_single_section() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        stack.add_take("take2.wav".into(), 1);

        let sections = vec![CompSection {
            offset: FramePos(0),
            duration: FramePos(44100),
            take_index: 1,
        }];
        let regions = stack.build_comp(&sections);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].audio_file_id, "take2.wav");
        assert_eq!(regions[0].timeline_pos, FramePos(0));
        assert_eq!(regions[0].duration, FramePos(44100));
    }

    #[test]
    fn build_comp_multiple_sections() {
        let mut stack = TakeStack::new(FramePos(1000), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        stack.add_take("take2.wav".into(), 1);
        stack.add_take("take3.wav".into(), 2);

        let sections = vec![
            CompSection {
                offset: FramePos(0),
                duration: FramePos(20000),
                take_index: 0,
            },
            CompSection {
                offset: FramePos(20000),
                duration: FramePos(10000),
                take_index: 2,
            },
            CompSection {
                offset: FramePos(30000),
                duration: FramePos(14100),
                take_index: 1,
            },
        ];
        let regions = stack.build_comp(&sections);
        assert_eq!(regions.len(), 3);

        // First section: take1 at timeline_pos 1000
        assert_eq!(regions[0].audio_file_id, "take1.wav");
        assert_eq!(regions[0].timeline_pos, FramePos(1000));
        assert_eq!(regions[0].source_offset, FramePos(0));
        assert_eq!(regions[0].duration, FramePos(20000));

        // Second section: take3 at timeline_pos 21000
        assert_eq!(regions[1].audio_file_id, "take3.wav");
        assert_eq!(regions[1].timeline_pos, FramePos(21000));
        assert_eq!(regions[1].source_offset, FramePos(20000));

        // Third section: take2 at timeline_pos 31000
        assert_eq!(regions[2].audio_file_id, "take2.wav");
        assert_eq!(regions[2].timeline_pos, FramePos(31000));
    }

    #[test]
    fn build_comp_skips_invalid_take_index() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);

        let sections = vec![
            CompSection {
                offset: FramePos(0),
                duration: FramePos(22050),
                take_index: 0,
            },
            CompSection {
                offset: FramePos(22050),
                duration: FramePos(22050),
                take_index: 99, // invalid
            },
        ];
        let regions = stack.build_comp(&sections);
        assert_eq!(regions.len(), 1); // second section skipped
    }

    #[test]
    fn build_comp_from_active() {
        let mut stack = TakeStack::new(FramePos(500), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        stack.add_take("take2.wav".into(), 1);
        // active is take2 (index 1)

        let region = stack.build_comp_from_active().unwrap();
        assert_eq!(region.audio_file_id, "take2.wav");
        assert_eq!(region.timeline_pos, FramePos(500));
        assert_eq!(region.duration, FramePos(44100));
    }

    #[test]
    fn build_comp_from_active_empty_stack() {
        let stack = TakeStack::new(FramePos(0), FramePos(44100));
        assert!(stack.build_comp_from_active().is_none());
    }

    #[test]
    fn build_comp_split() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        stack.add_take("take2.wav".into(), 1);

        let regions = stack.build_comp_split(FramePos(22050), 0, 1);
        assert_eq!(regions.len(), 2);

        // Left half: take1
        assert_eq!(regions[0].audio_file_id, "take1.wav");
        assert_eq!(regions[0].timeline_pos, FramePos(0));
        assert_eq!(regions[0].duration, FramePos(22050));

        // Right half: take2
        assert_eq!(regions[1].audio_file_id, "take2.wav");
        assert_eq!(regions[1].timeline_pos, FramePos(22050));
        assert_eq!(regions[1].duration, FramePos(22050));
    }

    #[test]
    fn build_comp_split_at_zero() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        stack.add_take("take2.wav".into(), 1);

        // Split at 0 = degenerate, whole thing uses right take
        let regions = stack.build_comp_split(FramePos(0), 0, 1);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].audio_file_id, "take2.wav");
    }

    #[test]
    fn build_comp_split_at_end() {
        let mut stack = TakeStack::new(FramePos(0), FramePos(44100));
        stack.add_take("take1.wav".into(), 0);
        stack.add_take("take2.wav".into(), 1);

        // Split at or past duration = degenerate, whole thing uses left take
        let regions = stack.build_comp_split(FramePos(44100), 0, 1);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].audio_file_id, "take1.wav");
    }

    #[test]
    fn comp_section_serializes() {
        let section = CompSection {
            offset: FramePos(1000),
            duration: FramePos(5000),
            take_index: 2,
        };
        let json = serde_json::to_string(&section).unwrap();
        let rt: CompSection = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.offset, FramePos(1000));
        assert_eq!(rt.take_index, 2);
    }
}
