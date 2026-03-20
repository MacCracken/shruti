use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Rem, Sub, SubAssign};

/// Frame position on the timeline (sample-accurate).
///
/// Wraps a `u64` frame count. At 48kHz, this supports timelines up to
/// ~12.2 million years, so overflow is not a practical concern. Standard
/// arithmetic operators (`+`, `-`) use wrapping semantics; use
/// `saturating_sub` for cases where underflow must be avoided.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[repr(transparent)]
#[serde(transparent)]
pub struct FramePos(pub u64);

impl FramePos {
    pub const ZERO: FramePos = FramePos(0);

    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }
    pub fn as_f64(self) -> f64 {
        self.0 as f64
    }

    /// Absolute difference between two frame positions.
    pub fn abs_diff(self, other: FramePos) -> u64 {
        self.0.abs_diff(other.0)
    }

    /// Saturating subtraction (clamps at zero instead of panicking).
    pub fn saturating_sub(self, rhs: FramePos) -> FramePos {
        FramePos(self.0.saturating_sub(rhs.0))
    }

    /// Returns the minimum of two frame positions.
    pub fn min(self, other: FramePos) -> FramePos {
        FramePos(self.0.min(other.0))
    }

    /// Returns the maximum of two frame positions.
    pub fn max(self, other: FramePos) -> FramePos {
        FramePos(self.0.max(other.0))
    }
}

impl fmt::Display for FramePos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Arithmetic ops: FramePos + FramePos, FramePos - FramePos, etc.
impl Add for FramePos {
    type Output = FramePos;
    fn add(self, rhs: FramePos) -> FramePos {
        FramePos(self.0 + rhs.0)
    }
}
impl Sub for FramePos {
    type Output = FramePos;
    fn sub(self, rhs: FramePos) -> FramePos {
        FramePos(self.0 - rhs.0)
    }
}
impl AddAssign for FramePos {
    fn add_assign(&mut self, rhs: FramePos) {
        self.0 += rhs.0;
    }
}
impl SubAssign for FramePos {
    fn sub_assign(&mut self, rhs: FramePos) {
        self.0 -= rhs.0;
    }
}
impl Rem for FramePos {
    type Output = FramePos;
    fn rem(self, rhs: FramePos) -> FramePos {
        FramePos(self.0 % rhs.0)
    }
}

// Conversions
impl From<u64> for FramePos {
    fn from(v: u64) -> Self {
        FramePos(v)
    }
}
impl From<FramePos> for u64 {
    fn from(v: FramePos) -> u64 {
        v.0
    }
}
impl From<u32> for FramePos {
    fn from(v: u32) -> Self {
        FramePos(v as u64)
    }
}

/// A track slot index into the session's track list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct TrackSlot(pub usize);

impl fmt::Display for TrackSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for TrackSlot {
    fn from(v: usize) -> Self {
        TrackSlot(v)
    }
}
impl From<TrackSlot> for usize {
    fn from(v: TrackSlot) -> usize {
        v.0
    }
}
