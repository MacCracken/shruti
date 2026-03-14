use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::FramePos;

/// Unique identifier for a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionId(pub Uuid);

impl RegionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RegionId {
    fn default() -> Self {
        Self::new()
    }
}

/// A region is a non-destructive reference to a segment of audio.
///
/// Regions point into the audio pool (by file ID) and define which portion
/// of the source audio to play and where to place it on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: RegionId,
    /// ID of the source audio file in the audio pool.
    pub audio_file_id: String,
    /// Position on the timeline in frames (where the region starts playing).
    pub timeline_pos: FramePos,
    /// Offset into the source audio in frames (where to start reading).
    pub source_offset: FramePos,
    /// Duration of the region in frames.
    pub duration: FramePos,
    /// Gain applied to this region (linear, 1.0 = unity).
    pub gain: f32,
    /// Fade-in duration in frames.
    pub fade_in: FramePos,
    /// Fade-out duration in frames.
    pub fade_out: FramePos,
    /// Whether this region is muted.
    pub muted: bool,
}

impl Region {
    pub fn new(
        audio_file_id: String,
        timeline_pos: impl Into<FramePos>,
        source_offset: impl Into<FramePos>,
        duration: impl Into<FramePos>,
    ) -> Self {
        Self {
            id: RegionId::new(),
            audio_file_id,
            timeline_pos: timeline_pos.into(),
            source_offset: source_offset.into(),
            duration: duration.into(),
            gain: 1.0,
            fade_in: FramePos::ZERO,
            fade_out: FramePos::ZERO,
            muted: false,
        }
    }

    /// The frame where this region ends on the timeline.
    pub fn end_pos(&self) -> FramePos {
        self.timeline_pos + self.duration
    }

    /// Check if a timeline frame falls within this region.
    pub fn contains_frame(&self, frame: FramePos) -> bool {
        frame >= self.timeline_pos && frame < self.end_pos()
    }

    /// Given a timeline frame, return the corresponding source audio frame.
    pub fn source_frame_at(&self, timeline_frame: FramePos) -> Option<FramePos> {
        if !self.contains_frame(timeline_frame) {
            return None;
        }
        Some(self.source_offset + (timeline_frame - self.timeline_pos))
    }

    /// Calculate the fade gain at a given position within the region.
    pub fn fade_gain_at(&self, timeline_frame: FramePos) -> f32 {
        if !self.contains_frame(timeline_frame) {
            return 0.0;
        }

        let local = timeline_frame - self.timeline_pos;
        let mut gain = self.gain;

        // Fade in
        if self.fade_in > FramePos::ZERO && local < self.fade_in {
            gain *= local.as_f32() / self.fade_in.as_f32();
        }

        // Fade out
        if self.fade_out > FramePos::ZERO {
            let remaining = self.duration - local;
            if remaining < self.fade_out {
                gain *= remaining.as_f32() / self.fade_out.as_f32();
            }
        }

        gain
    }

    /// Split this region at the given timeline frame, returning (left, right).
    /// Returns None if the split point is outside the region.
    pub fn split_at(&self, frame: FramePos) -> Option<(Region, Region)> {
        if frame <= self.timeline_pos || frame >= self.end_pos() {
            return None;
        }

        let left_duration = frame - self.timeline_pos;
        let right_duration = self.duration - left_duration;
        let right_source_offset = self.source_offset + left_duration;

        let left = Region {
            id: RegionId::new(),
            audio_file_id: self.audio_file_id.clone(),
            timeline_pos: self.timeline_pos,
            source_offset: self.source_offset,
            duration: left_duration,
            gain: self.gain,
            fade_in: self.fade_in.min(left_duration),
            fade_out: FramePos::ZERO,
            muted: self.muted,
        };

        let right = Region {
            id: RegionId::new(),
            audio_file_id: self.audio_file_id.clone(),
            timeline_pos: frame,
            source_offset: right_source_offset,
            duration: right_duration,
            gain: self.gain,
            fade_in: FramePos::ZERO,
            fade_out: self.fade_out.min(right_duration),
            muted: self.muted,
        };

        Some((left, right))
    }

    /// Trim the start of the region by moving it forward.
    pub fn trim_start(&mut self, new_start: FramePos) {
        if new_start > self.timeline_pos && new_start < self.end_pos() {
            let delta = new_start - self.timeline_pos;
            self.source_offset += delta;
            self.duration -= delta;
            self.timeline_pos = new_start;
        }
    }

    /// Trim the end of the region.
    pub fn trim_end(&mut self, new_end: FramePos) {
        if new_end > self.timeline_pos && new_end < self.end_pos() {
            self.duration = new_end - self.timeline_pos;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_contains() {
        let r = Region::new("file1".into(), FramePos(100), FramePos(0), FramePos(500));
        assert!(!r.contains_frame(FramePos(99)));
        assert!(r.contains_frame(FramePos(100)));
        assert!(r.contains_frame(FramePos(599)));
        assert!(!r.contains_frame(FramePos(600)));
    }

    #[test]
    fn test_region_source_frame() {
        let r = Region::new("file1".into(), FramePos(100), FramePos(50), FramePos(500));
        assert_eq!(r.source_frame_at(FramePos(100)), Some(FramePos(50)));
        assert_eq!(r.source_frame_at(FramePos(200)), Some(FramePos(150)));
        assert_eq!(r.source_frame_at(FramePos(50)), None);
    }

    #[test]
    fn test_region_split() {
        let r = Region::new("file1".into(), FramePos(100), FramePos(0), FramePos(500));
        let (left, right) = r.split_at(FramePos(300)).unwrap();

        assert_eq!(left.timeline_pos, FramePos(100));
        assert_eq!(left.duration, FramePos(200));
        assert_eq!(left.source_offset, FramePos(0));

        assert_eq!(right.timeline_pos, FramePos(300));
        assert_eq!(right.duration, FramePos(300));
        assert_eq!(right.source_offset, FramePos(200));
    }

    #[test]
    fn test_region_trim() {
        let mut r = Region::new("file1".into(), FramePos(100), FramePos(0), FramePos(500));

        r.trim_start(FramePos(200));
        assert_eq!(r.timeline_pos, FramePos(200));
        assert_eq!(r.source_offset, FramePos(100));
        assert_eq!(r.duration, FramePos(400));

        r.trim_end(FramePos(500));
        assert_eq!(r.duration, FramePos(300));
    }

    #[test]
    fn test_fade_gain() {
        let mut r = Region::new("file1".into(), FramePos(0), FramePos(0), FramePos(1000));
        r.fade_in = FramePos(100);
        r.fade_out = FramePos(100);

        assert!((r.fade_gain_at(FramePos(0)) - 0.0).abs() < 1e-6);
        assert!((r.fade_gain_at(FramePos(50)) - 0.5).abs() < 1e-6);
        assert!((r.fade_gain_at(FramePos(100)) - 1.0).abs() < 1e-6);
        assert!((r.fade_gain_at(FramePos(500)) - 1.0).abs() < 1e-6);
        assert!((r.fade_gain_at(FramePos(950)) - 0.5).abs() < 1e-6);
    }
}
