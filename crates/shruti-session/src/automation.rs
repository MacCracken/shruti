use serde::{Deserialize, Serialize};

use crate::types::FramePos;

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
    /// Instrument parameter by index (maps to `InstrumentParam` position).
    InstrumentParam { param_index: usize },
}

impl AutomationTarget {
    /// Returns a display label for this target.
    pub fn label(&self) -> String {
        match self {
            AutomationTarget::TrackGain => "Volume".to_string(),
            AutomationTarget::TrackPan => "Pan".to_string(),
            AutomationTarget::PluginParam { slot, param_id } => {
                format!("Plugin {slot} Param {param_id}")
            }
            AutomationTarget::SendLevel { send_index } => format!("Send {}", send_index + 1),
            AutomationTarget::InstrumentParam { param_index } => {
                format!("Instrument Param {param_index}")
            }
        }
    }

    /// Returns all standard automation targets for an instrument track
    /// with the given number of parameters.
    pub fn instrument_targets(param_count: usize) -> Vec<AutomationTarget> {
        let mut targets = vec![AutomationTarget::TrackGain, AutomationTarget::TrackPan];
        for i in 0..param_count {
            targets.push(AutomationTarget::InstrumentParam { param_index: i });
        }
        targets
    }
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
    pub position: FramePos,
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
    pub fn remove_point_near(
        &mut self,
        position: FramePos,
        tolerance: u64,
    ) -> Option<AutomationPoint> {
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
    pub fn value_at(&self, position: FramePos) -> Option<f32> {
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

        // Find surrounding points.
        // After the early returns above, `position` is strictly between the
        // first and last point, so `right_idx` is always >= 1.
        let right_idx = self
            .points
            .binary_search_by_key(&position, |p| p.position)
            .unwrap_or_else(|i| i);
        debug_assert!(
            right_idx >= 1,
            "right_idx should be >= 1 after range guards"
        );

        let left = &self.points[right_idx - 1];
        let right = &self.points[right_idx];

        let t = (position - left.position).as_f32() / (right.position - left.position).as_f32();

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
    pub fn points_in_range(&self, start: FramePos, end: FramePos) -> &[AutomationPoint] {
        let start_idx = self
            .points
            .binary_search_by_key(&start, |p| p.position)
            .unwrap_or_else(|i| i);
        let end_idx = self
            .points
            .binary_search_by_key(&end, |p| p.position)
            .unwrap_or_else(|i| i);
        &self.points[start_idx..end_idx.max(start_idx)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_lane_add_sorted() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 0.5,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(50),
            value: 0.0,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(200),
            value: 1.0,
            curve: CurveType::Linear,
        });

        assert_eq!(lane.points[0].position, FramePos(50));
        assert_eq!(lane.points[1].position, FramePos(100));
        assert_eq!(lane.points[2].position, FramePos(200));
    }

    #[test]
    fn test_linear_interpolation() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: FramePos(0),
            value: 0.0,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 1.0,
            curve: CurveType::Linear,
        });

        assert!((lane.value_at(FramePos(0)).unwrap() - 0.0).abs() < 0.001);
        assert!((lane.value_at(FramePos(50)).unwrap() - 0.5).abs() < 0.001);
        assert!((lane.value_at(FramePos(100)).unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_step_interpolation() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackPan);
        lane.add_point(AutomationPoint {
            position: FramePos(0),
            value: -1.0,
            curve: CurveType::Step,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 1.0,
            curve: CurveType::Step,
        });

        assert_eq!(lane.value_at(FramePos(0)).unwrap(), -1.0);
        assert_eq!(lane.value_at(FramePos(50)).unwrap(), -1.0);
        assert_eq!(lane.value_at(FramePos(99)).unwrap(), -1.0);
        assert_eq!(lane.value_at(FramePos(100)).unwrap(), 1.0);
    }

    #[test]
    fn test_scurve_interpolation() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: FramePos(0),
            value: 0.0,
            curve: CurveType::SCurve,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 1.0,
            curve: CurveType::SCurve,
        });

        let mid = lane.value_at(FramePos(50)).unwrap();
        // S-curve at midpoint should be 0.5
        assert!((mid - 0.5).abs() < 0.01);

        // S-curve should start slow and end slow
        let early = lane.value_at(FramePos(10)).unwrap();
        let late = lane.value_at(FramePos(90)).unwrap();
        // The slope at the edges should be gentler than linear
        assert!(early < 0.1 + 0.01); // Should be < linear (0.1)
        assert!(late > 0.9 - 0.01); // Should be > linear (0.9)
    }

    #[test]
    fn test_value_outside_range() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 0.5,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(200),
            value: 1.0,
            curve: CurveType::Linear,
        });

        // Before first point -- hold first value
        assert_eq!(lane.value_at(FramePos(0)).unwrap(), 0.5);
        // After last point -- hold last value
        assert_eq!(lane.value_at(FramePos(300)).unwrap(), 1.0);
    }

    #[test]
    fn test_empty_lane() {
        let lane = AutomationLane::new(AutomationTarget::TrackGain);
        assert!(lane.value_at(FramePos(50)).is_none());
    }

    #[test]
    fn test_remove_point() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 0.5,
            curve: CurveType::Linear,
        });

        assert!(lane.remove_point_near(FramePos(105), 10).is_some());
        assert!(lane.points.is_empty());
    }

    #[test]
    fn test_step_interpolation_holds_value() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackPan);
        lane.add_point(AutomationPoint {
            position: FramePos(0),
            value: 0.0,
            curve: CurveType::Step,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 1.0,
            curve: CurveType::Step,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(200),
            value: 0.5,
            curve: CurveType::Step,
        });

        // Between first and second, should hold first value
        assert_eq!(lane.value_at(FramePos(1)).unwrap(), 0.0);
        assert_eq!(lane.value_at(FramePos(99)).unwrap(), 0.0);
        // Between second and third, should hold second value
        assert_eq!(lane.value_at(FramePos(150)).unwrap(), 1.0);
        assert_eq!(lane.value_at(FramePos(199)).unwrap(), 1.0);
        // At third point exactly
        assert_eq!(lane.value_at(FramePos(200)).unwrap(), 0.5);
    }

    #[test]
    fn test_scurve_endpoints_and_symmetry() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: FramePos(0),
            value: 0.0,
            curve: CurveType::SCurve,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(1000),
            value: 1.0,
            curve: CurveType::SCurve,
        });

        // Endpoints
        assert!((lane.value_at(FramePos(0)).unwrap() - 0.0).abs() < 0.001);
        assert!((lane.value_at(FramePos(1000)).unwrap() - 1.0).abs() < 0.001);

        // Midpoint should be exactly 0.5
        assert!((lane.value_at(FramePos(500)).unwrap() - 0.5).abs() < 0.001);

        // Symmetry: value at 250 + value at 750 should equal 1.0
        let v250 = lane.value_at(FramePos(250)).unwrap();
        let v750 = lane.value_at(FramePos(750)).unwrap();
        assert!((v250 + v750 - 1.0).abs() < 0.01);

        // S-curve should be slower than linear near edges
        let v100 = lane.value_at(FramePos(100)).unwrap();
        assert!(v100 < 0.1); // linear would be 0.1, s-curve should be less
    }

    #[test]
    fn test_empty_lane_returns_none() {
        let lane = AutomationLane::new(AutomationTarget::TrackGain);
        assert!(lane.value_at(FramePos(0)).is_none());
        assert!(lane.value_at(FramePos(1000)).is_none());
    }

    #[test]
    fn test_single_point_lane() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: FramePos(500),
            value: 0.75,
            curve: CurveType::Linear,
        });

        // Before the point
        assert_eq!(lane.value_at(FramePos(0)).unwrap(), 0.75);
        // At the point
        assert_eq!(lane.value_at(FramePos(500)).unwrap(), 0.75);
        // After the point
        assert_eq!(lane.value_at(FramePos(1000)).unwrap(), 0.75);
    }

    #[test]
    fn test_points_sorted_after_add() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        // Add points in reverse order
        lane.add_point(AutomationPoint {
            position: FramePos(300),
            value: 0.3,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 0.1,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(500),
            value: 0.5,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(200),
            value: 0.2,
            curve: CurveType::Linear,
        });

        // Verify sorted
        for i in 1..lane.points.len() {
            assert!(lane.points[i].position >= lane.points[i - 1].position);
        }
        assert_eq!(lane.points[0].position, FramePos(100));
        assert_eq!(lane.points[1].position, FramePos(200));
        assert_eq!(lane.points[2].position, FramePos(300));
        assert_eq!(lane.points[3].position, FramePos(500));
    }

    #[test]
    fn test_points_in_range() {
        let mut lane = AutomationLane::new(AutomationTarget::TrackGain);
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 0.1,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(200),
            value: 0.2,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(300),
            value: 0.3,
            curve: CurveType::Linear,
        });

        let in_range = lane.points_in_range(FramePos(150), FramePos(250));
        assert_eq!(in_range.len(), 1);
        assert_eq!(in_range[0].position, FramePos(200));

        let all = lane.points_in_range(FramePos(0), FramePos(400));
        assert_eq!(all.len(), 3);

        let none = lane.points_in_range(FramePos(301), FramePos(400));
        assert_eq!(none.len(), 0);
    }

    #[test]
    fn instrument_param_target_works() {
        let target = AutomationTarget::InstrumentParam { param_index: 3 };
        let mut lane = AutomationLane::new(target.clone());
        lane.add_point(AutomationPoint {
            position: FramePos(0),
            value: 0.0,
            curve: CurveType::Linear,
        });
        lane.add_point(AutomationPoint {
            position: FramePos(100),
            value: 1.0,
            curve: CurveType::Linear,
        });
        assert!((lane.value_at(FramePos(50)).unwrap() - 0.5).abs() < 0.01);
        assert_eq!(
            lane.target,
            AutomationTarget::InstrumentParam { param_index: 3 }
        );
    }

    #[test]
    fn instrument_targets_list() {
        let targets = AutomationTarget::instrument_targets(5);
        assert_eq!(targets.len(), 7); // gain + pan + 5 params
        assert_eq!(targets[0], AutomationTarget::TrackGain);
        assert_eq!(targets[1], AutomationTarget::TrackPan);
        assert_eq!(
            targets[2],
            AutomationTarget::InstrumentParam { param_index: 0 }
        );
        assert_eq!(
            targets[6],
            AutomationTarget::InstrumentParam { param_index: 4 }
        );
    }

    #[test]
    fn target_labels() {
        assert_eq!(AutomationTarget::TrackGain.label(), "Volume");
        assert_eq!(AutomationTarget::TrackPan.label(), "Pan");
        assert_eq!(
            AutomationTarget::InstrumentParam { param_index: 2 }.label(),
            "Instrument Param 2"
        );
        assert_eq!(
            AutomationTarget::SendLevel { send_index: 0 }.label(),
            "Send 1"
        );
    }

    #[test]
    fn instrument_param_target_serde_roundtrip() {
        let target = AutomationTarget::InstrumentParam { param_index: 7 };
        let json = serde_json::to_string(&target).unwrap();
        let restored: AutomationTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, target);
    }
}
