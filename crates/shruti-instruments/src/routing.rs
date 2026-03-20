//! MIDI routing from MIDI tracks to instrument nodes.

use serde::{Deserialize, Serialize};
use shruti_session::midi::NoteEvent;
use uuid::Uuid;

/// How velocity values are transformed before reaching the instrument.
#[derive(Debug, Clone, PartialEq)]
pub enum VelocityCurve {
    /// Linear passthrough (identity).
    Linear,
    /// Gentler response — compresses high velocities.
    Soft,
    /// More aggressive response — expands high velocities.
    Hard,
    /// Always emit a fixed velocity regardless of input.
    Fixed(u8),
}

impl VelocityCurve {
    /// Apply the velocity curve to a raw MIDI velocity value.
    pub fn apply(&self, velocity: u8) -> u8 {
        match self {
            VelocityCurve::Linear => velocity,
            VelocityCurve::Soft => {
                // sqrt curve: compress dynamics
                let normalized = velocity as f64 / 127.0;
                let curved = normalized.sqrt();
                (curved * 127.0).round() as u8
            }
            VelocityCurve::Hard => {
                // square curve: expand dynamics
                let normalized = velocity as f64 / 127.0;
                let curved = normalized * normalized;
                (curved * 127.0).round() as u8
            }
            VelocityCurve::Fixed(v) => *v,
        }
    }
}

/// A route from a MIDI source track to an instrument.
#[derive(Debug, Clone)]
pub struct MidiRoute {
    /// The track that produces MIDI events.
    pub source_track_id: Uuid,
    /// If set, only events on this MIDI channel pass through (0-15). `None` = all channels.
    pub channel_filter: Option<u8>,
    /// Velocity transformation applied to passing events.
    pub velocity_curve: VelocityCurve,
    /// Inclusive note range `(min, max)` — notes outside are filtered out.
    pub note_range: (u8, u8),
}

impl MidiRoute {
    /// Create a new MIDI route that passes all events unmodified.
    pub fn new(source_track_id: Uuid) -> Self {
        Self {
            source_track_id,
            channel_filter: None,
            velocity_curve: VelocityCurve::Linear,
            note_range: (0, 127),
        }
    }

    /// Filter and transform a [`NoteEvent`].
    ///
    /// Returns `None` if the event is rejected by channel or note-range filters.
    /// Otherwise returns a (possibly modified) copy with the velocity curve applied.
    pub fn filter_event(&self, event: &NoteEvent) -> Option<NoteEvent> {
        // Channel filter
        if let Some(ch) = self.channel_filter
            && event.channel != ch
        {
            return None;
        }

        // Note range filter
        if event.note < self.note_range.0 || event.note > self.note_range.1 {
            return None;
        }

        // Apply velocity curve
        let mut out = event.clone();
        out.velocity = self.velocity_curve.apply(event.velocity);
        Some(out)
    }
}

/// Maps a MIDI CC to an instrument parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcMapping {
    /// MIDI CC number (0-127).
    pub cc: u8,
    /// Target parameter index.
    pub param_index: usize,
    /// Minimum parameter value (maps from CC=0).
    pub min_value: f32,
    /// Maximum parameter value (maps from CC=127).
    pub max_value: f32,
}

impl CcMapping {
    pub fn new(cc: u8, param_index: usize, min_value: f32, max_value: f32) -> Self {
        Self {
            cc,
            param_index,
            min_value,
            max_value,
        }
    }

    /// Map a 7-bit CC value to the parameter range.
    pub fn map_value(&self, cc_value: u8) -> f32 {
        let t = cc_value as f32 / 127.0;
        self.min_value + t * (self.max_value - self.min_value)
    }

    /// Map a 32-bit CC value (MIDI 2.0) to the parameter range.
    pub fn map_value_32(&self, cc_value: u32) -> f32 {
        let t = cc_value as f64 / u32::MAX as f64;
        self.min_value + t as f32 * (self.max_value - self.min_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(note: u8, velocity: u8, channel: u8) -> NoteEvent {
        NoteEvent {
            position: shruti_session::FramePos(0),
            duration: shruti_session::FramePos(100),
            note,
            velocity,
            channel,
        }
    }

    #[test]
    fn passthrough_all_events() {
        let route = MidiRoute::new(Uuid::new_v4());
        let event = make_event(60, 100, 0);
        let result = route.filter_event(&event).unwrap();
        assert_eq!(result.note, 60);
        assert_eq!(result.velocity, 100);
        assert_eq!(result.channel, 0);
    }

    #[test]
    fn channel_filter_passes_matching() {
        let mut route = MidiRoute::new(Uuid::new_v4());
        route.channel_filter = Some(5);
        let event = make_event(60, 100, 5);
        assert!(route.filter_event(&event).is_some());
    }

    #[test]
    fn channel_filter_rejects_non_matching() {
        let mut route = MidiRoute::new(Uuid::new_v4());
        route.channel_filter = Some(5);
        let event = make_event(60, 100, 3);
        assert!(route.filter_event(&event).is_none());
    }

    #[test]
    fn note_range_passes_within() {
        let mut route = MidiRoute::new(Uuid::new_v4());
        route.note_range = (36, 72);
        assert!(route.filter_event(&make_event(36, 100, 0)).is_some());
        assert!(route.filter_event(&make_event(72, 100, 0)).is_some());
        assert!(route.filter_event(&make_event(54, 100, 0)).is_some());
    }

    #[test]
    fn note_range_rejects_outside() {
        let mut route = MidiRoute::new(Uuid::new_v4());
        route.note_range = (36, 72);
        assert!(route.filter_event(&make_event(35, 100, 0)).is_none());
        assert!(route.filter_event(&make_event(73, 100, 0)).is_none());
        assert!(route.filter_event(&make_event(0, 100, 0)).is_none());
        assert!(route.filter_event(&make_event(127, 100, 0)).is_none());
    }

    #[test]
    fn velocity_curve_linear() {
        let curve = VelocityCurve::Linear;
        assert_eq!(curve.apply(0), 0);
        assert_eq!(curve.apply(64), 64);
        assert_eq!(curve.apply(127), 127);
    }

    #[test]
    fn velocity_curve_soft() {
        let curve = VelocityCurve::Soft;
        // Soft curve: output >= input for sub-max values (sqrt expands low values)
        assert_eq!(curve.apply(0), 0);
        assert_eq!(curve.apply(127), 127);
        // A mid-range value should be boosted
        let mid = curve.apply(32);
        assert!(
            mid > 32,
            "soft curve should boost low velocities: got {mid}"
        );
    }

    #[test]
    fn velocity_curve_hard() {
        let curve = VelocityCurve::Hard;
        assert_eq!(curve.apply(0), 0);
        assert_eq!(curve.apply(127), 127);
        // A mid-range value should be reduced
        let mid = curve.apply(90);
        assert!(
            mid < 90,
            "hard curve should reduce mid velocities: got {mid}"
        );
    }

    #[test]
    fn velocity_curve_fixed() {
        let curve = VelocityCurve::Fixed(100);
        assert_eq!(curve.apply(0), 100);
        assert_eq!(curve.apply(50), 100);
        assert_eq!(curve.apply(127), 100);
    }

    #[test]
    fn combined_channel_and_note_range_filter() {
        let mut route = MidiRoute::new(Uuid::new_v4());
        route.channel_filter = Some(0);
        route.note_range = (48, 72);
        route.velocity_curve = VelocityCurve::Fixed(80);

        // Passes: right channel, right note range
        let result = route.filter_event(&make_event(60, 127, 0)).unwrap();
        assert_eq!(result.velocity, 80); // fixed velocity

        // Fails: wrong channel
        assert!(route.filter_event(&make_event(60, 127, 1)).is_none());

        // Fails: note out of range
        assert!(route.filter_event(&make_event(47, 127, 0)).is_none());
    }

    #[test]
    fn cc_mapping_maps_value() {
        let mapping = CcMapping::new(7, 0, 0.0, 1.0);
        assert!((mapping.map_value(0) - 0.0).abs() < 1e-6);
        assert!((mapping.map_value(127) - 1.0).abs() < 1e-6);
        assert!((mapping.map_value(64) - 0.50394).abs() < 0.01);
    }

    #[test]
    fn cc_mapping_custom_range() {
        let mapping = CcMapping::new(74, 5, 20.0, 20000.0);
        let val = mapping.map_value(127);
        assert!((val - 20000.0).abs() < 1.0);
        let val = mapping.map_value(0);
        assert!((val - 20.0).abs() < 1.0);
    }

    #[test]
    fn cc_mapping_32bit() {
        let mapping = CcMapping::new(1, 0, 0.0, 1.0);
        let val = mapping.map_value_32(u32::MAX);
        assert!((val - 1.0).abs() < 0.01);
        let val = mapping.map_value_32(0);
        assert!((val - 0.0).abs() < 0.01);
    }
}
