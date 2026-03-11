use serde::{Deserialize, Serialize};

/// Identifies a parameter target for automation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AutomationTarget {
    /// Track volume (linear gain).
    TrackGain,
    /// Track pan (-1.0 to 1.0).
    TrackPan,
    /// Plugin parameter by index.
    PluginParam { slot: usize, param_id: u32 },
    /// Send level by send index.
    SendLevel { send_index: usize },
}

/// Interpolation curve between two automation points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurveType {
    /// Linear interpolation.
    Linear,
    /// Step (hold previous value until next point).
    Step,
    /// Smooth S-curve.
    SCurve,
}

/// A single automation point at a specific timeline position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationPoint {
    /// Position in frames on the timeline.
    pub position: u64,
    /// Parameter value at this point.
    pub value: f32,
    /// Curve type for interpolation to the next point.
    pub curve: CurveType,
}

/// An automation lane: a sequence of points for a single parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationLane {
    /// What parameter this lane controls.
    pub target: AutomationTarget,
    /// Automation points sorted by position.
    pub points: Vec<AutomationPoint>,
    /// Whether this lane is active.
    pub enabled: bool,
}

impl AutomationLane {
    pub fn new(target: AutomationTarget) -> Self {
        Self {
            target,
            points: Vec::new(),
            enabled: true,
        }
    }

    /// Add a point, maintaining sorted order by position.
    pub fn add_point(&mut self, point: AutomationPoint) {
        let pos = self
            .points
            .binary_search_by_key(&point.position, |p| p.position)
            .unwrap_or_else(|i| i);
        self.points.insert(pos, point);
    }

    /// Remove the point closest to the given position within tolerance.
    pub fn remove_point_near(&mut self, position: u64, tolerance: u64) -> Option<AutomationPoint> {
        if let Some(idx) = self
            .points
            .iter()
            .position(|p| p.position.abs_diff(position) <= tolerance)
        {
            Some(self.points.remove(idx))
        } else {
            None
        }
    }

    /// Get the interpolated value at a given timeline position.
    pub fn value_at(&self, position: u64) -> Option<f32> {
        if self.points.is_empty() {
            return None;
        }

        // Before first point
        if position <= self.points[0].position {
            return Some(self.points[0].value);
        }

        // After last point
        if position >= self.points.last().unwrap().position {
            return Some(self.points.last().unwrap().value);
        }

        // Find surrounding points
        let right_idx = self
            .points
            .binary_search_by_key(&position, |p| p.position)
            .unwrap_or_else(|i| i);

        if right_idx == 0 {
            return Some(self.points[0].value);
        }

        let left = &self.points[right_idx - 1];
        let right = &self.points[right_idx];

        let t = (position - left.position) as f32 / (right.position - left.position) as f32;

        let value = match left.curve {
            CurveType::Linear => left.value + (right.value - left.value) * t,
            CurveType::Step => left.value,
            CurveType::SCurve => {
                // Hermite-style smooth step
                let t2 = t * t * (3.0 - 2.0 * t);
                left.value + (right.value - left.value) * t2
            }
        };

        Some(value)
    }

    /// Get all points in a range.
    pub fn points_in_range(&self, start: u64, end: u64) -> &[AutomationPoint] {
        let start_idx = self
            .points
            .binary_search_by_key(&start, |p| p.position)
            .unwrap_or_else(|i| i);
        let end_idx = self
            .points
            .binary_search_by_key(&end, |p| p.position)
            .unwrap_or_else(|i| i);
        &self.points[start_idx..end_idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_lane_add_sorted() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: 100,
            value: 0.5,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: 50,
            value: 0.0,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: 200,
            value: 1.0,
            curve: CurveType::Linear,
        });

        assert_eq!(lane.points[0].position, 50);
        assert_eq!(lane.points[1].position, 100);
        assert_eq!(lane.points[2].position, 200);
    }

    #[test]
    fn test_linear_interpolation() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: 0,
            value: 0.0,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: 100,
            value: 1.0,
            curve: CurveType::Linear,
        });

        assert!((lane.value_at(0).unwrap() - 0.0).abs() < 0.001);
        assert!((lane.value_at(50).unwrap() - 0.5).abs() < 0.001);
        assert!((lane.value_at(100).unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_step_interpolation() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackPan);
        lane.add_point(AutomationPoint {
            position: 0,
            value: -1.0,
            curve: CurveType::Step,
        });
        lane.add_point(AutomationPoint {
            position: 100,
            value: 1.0,
            curve: CurveType::Step,
        });

        assert_eq!(lane.value_at(0).unwrap(), -1.0);
        assert_eq!(lane.value_at(50).unwrap(), -1.0);
        assert_eq!(lane.value_at(99).unwrap(), -1.0);
        assert_eq!(lane.value_at(100).unwrap(), 1.0);
    }

    #[test]
    fn test_scurve_interpolation() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: 0,
            value: 0.0,
            curve: CurveType::SCurve,
        });
        lane.add_point(AutomationPoint {
            position: 100,
            value: 1.0,
            curve: CurveType::SCurve,
        });

        let mid = lane.value_at(50).unwrap();
        // S-curve at midpoint should be 0.5
        assert!((mid - 0.5).abs() < 0.01);

        // S-curve should start slow and end slow
        let early = lane.value_at(10).unwrap();
        let late = lane.value_at(90).unwrap();
        // The slope at the edges should be gentler than linear
        assert!(early < 0.1 + 0.01); // Should be < linear (0.1)
        assert!(late > 0.9 - 0.01); // Should be > linear (0.9)
    }

    #[test]
    fn test_value_outside_range() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: 100,
            value: 0.5,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: 200,
            value: 1.0,
            curve: CurveType::Linear,
        });

        // Before first point — hold first value
        assert_eq!(lane.value_at(0).unwrap(), 0.5);
        // After last point — hold last value
        assert_eq!(lane.value_at(300).unwrap(), 1.0);
    }

    #[test]
    fn test_empty_lane() {
        let lane = AutomationLane::new(AutomationTarget::TrackGain);
        assert!(lane.value_at(50).is_none());
    }

    #[test]
    fn test_remove_point() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: 100,
            value: 0.5,
            curve: CurveType::Linear,
        });

        assert!(lane.remove_point_near(105, 10).is_some());
        assert!(lane.points.is_empty());
    }
}
