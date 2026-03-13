//! Modulation matrix for assignable modulation routing.
//!
//! Sources (LFO, envelope, velocity, aftertouch, mod wheel, pitch bend) can be
//! routed to any destination parameter with a bipolar amount (-1..+1).
//! Supports both per-voice sources (velocity, envelopes) and global sources
//! (LFOs, mod wheel, aftertouch, pitch bend).

use serde::{Deserialize, Serialize};

/// Maximum number of simultaneous modulation routings.
pub const MAX_ROUTINGS: usize = 16;

/// Number of modulation destinations (used for fixed-size evaluation output).
pub const NUM_DESTINATIONS: usize = 8;

/// A modulation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModSource {
    /// LFO 1 (global, bipolar -1..+1).
    Lfo1,
    /// LFO 2 (global, bipolar -1..+1).
    Lfo2,
    /// Amplitude envelope (per-voice, unipolar 0..1).
    AmpEnvelope,
    /// Filter envelope (per-voice, unipolar 0..1).
    FilterEnvelope,
    /// Note velocity (per-voice, unipolar 0..1).
    Velocity,
    /// MIDI mod wheel CC#1 (global, unipolar 0..1).
    ModWheel,
    /// MIDI channel aftertouch (global, unipolar 0..1).
    Aftertouch,
    /// MIDI pitch bend (global, bipolar -1..+1).
    PitchBend,
}

/// A modulation destination parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModDestination {
    /// Oscillator pitch (in semitones).
    Pitch,
    /// Filter cutoff frequency (in octaves).
    FilterCutoff,
    /// Filter resonance (linear 0..1 range).
    FilterResonance,
    /// Output volume (linear multiplier).
    Volume,
    /// Stereo pan position.
    Pan,
    /// LFO 1 rate modulation.
    Lfo1Rate,
    /// LFO 2 rate modulation.
    Lfo2Rate,
    /// Oscillator detune (in cents).
    OscDetune,
}

impl ModDestination {
    /// Convert destination to its index for use in the evaluation output array.
    #[inline]
    pub fn index(self) -> usize {
        match self {
            ModDestination::Pitch => 0,
            ModDestination::FilterCutoff => 1,
            ModDestination::FilterResonance => 2,
            ModDestination::Volume => 3,
            ModDestination::Pan => 4,
            ModDestination::Lfo1Rate => 5,
            ModDestination::Lfo2Rate => 6,
            ModDestination::OscDetune => 7,
        }
    }

    /// All destination variants, for iteration.
    pub const ALL: [ModDestination; NUM_DESTINATIONS] = [
        ModDestination::Pitch,
        ModDestination::FilterCutoff,
        ModDestination::FilterResonance,
        ModDestination::Volume,
        ModDestination::Pan,
        ModDestination::Lfo1Rate,
        ModDestination::Lfo2Rate,
        ModDestination::OscDetune,
    ];
}

/// A single modulation routing: source → destination with a scaled amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModRouting {
    /// The modulation source signal.
    pub source: ModSource,
    /// The parameter being modulated.
    pub destination: ModDestination,
    /// Bipolar scaling amount (-1.0 to +1.0).
    /// Positive values add the source signal; negative values invert it.
    pub amount: f32,
    /// Whether this routing is active.
    pub enabled: bool,
}

impl ModRouting {
    /// Create a new enabled routing with the given amount (clamped to -1..+1).
    pub fn new(source: ModSource, destination: ModDestination, amount: f32) -> Self {
        Self {
            source,
            destination,
            amount: amount.clamp(-1.0, 1.0),
            enabled: true,
        }
    }
}

/// Current values of all modulation sources for a single voice context.
///
/// Per-voice sources (velocity, amp envelope, filter envelope) vary per voice.
/// Global sources (LFOs, mod wheel, aftertouch, pitch bend) are shared across voices.
#[derive(Debug, Clone, Default)]
pub struct ModSourceValues {
    /// LFO 1 output (bipolar, -1..+1).
    pub lfo1: f32,
    /// LFO 2 output (bipolar, -1..+1).
    pub lfo2: f32,
    /// Amp envelope level (unipolar, 0..1).
    pub amp_envelope: f32,
    /// Filter envelope level (unipolar, 0..1).
    pub filter_envelope: f32,
    /// Note velocity, normalized (unipolar, 0..1).
    pub velocity: f32,
    /// Mod wheel position (unipolar, 0..1).
    pub mod_wheel: f32,
    /// Channel aftertouch (unipolar, 0..1).
    pub aftertouch: f32,
    /// Pitch bend (bipolar, -1..+1).
    pub pitch_bend: f32,
}

impl ModSourceValues {
    /// Get the current value of a specific source.
    #[inline]
    pub fn get(&self, source: ModSource) -> f32 {
        match source {
            ModSource::Lfo1 => self.lfo1,
            ModSource::Lfo2 => self.lfo2,
            ModSource::AmpEnvelope => self.amp_envelope,
            ModSource::FilterEnvelope => self.filter_envelope,
            ModSource::Velocity => self.velocity,
            ModSource::ModWheel => self.mod_wheel,
            ModSource::Aftertouch => self.aftertouch,
            ModSource::PitchBend => self.pitch_bend,
        }
    }
}

/// Per-destination modulation output from evaluation.
///
/// Fixed-size array indexed by `ModDestination::index()`. Each element holds
/// the summed modulation amount for that destination.
#[derive(Debug, Clone)]
pub struct ModOutput {
    /// Summed modulation values per destination.
    pub values: [f32; NUM_DESTINATIONS],
}

impl Default for ModOutput {
    fn default() -> Self {
        Self {
            values: [0.0; NUM_DESTINATIONS],
        }
    }
}

impl ModOutput {
    /// Get the modulation amount for a specific destination.
    #[inline]
    pub fn get(&self, dest: ModDestination) -> f32 {
        self.values[dest.index()]
    }
}

/// Modulation matrix: a collection of source → destination routings.
///
/// Holds up to [`MAX_ROUTINGS`] entries. During evaluation, each enabled
/// routing's source value is multiplied by its amount and summed into the
/// corresponding destination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMatrix {
    routings: Vec<ModRouting>,
}

impl Default for ModMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl ModMatrix {
    /// Create an empty modulation matrix.
    pub fn new() -> Self {
        Self {
            routings: Vec::new(),
        }
    }

    /// Add a routing. Returns `true` if added, `false` if the matrix is full.
    pub fn add_routing(&mut self, routing: ModRouting) -> bool {
        if self.routings.len() >= MAX_ROUTINGS {
            return false;
        }
        self.routings.push(routing);
        true
    }

    /// Remove the routing at the given index. Returns the removed routing,
    /// or `None` if the index is out of bounds.
    pub fn remove_routing(&mut self, index: usize) -> Option<ModRouting> {
        if index < self.routings.len() {
            Some(self.routings.remove(index))
        } else {
            None
        }
    }

    /// Remove all routings.
    pub fn clear(&mut self) {
        self.routings.clear();
    }

    /// Get a slice of all routings.
    pub fn routings(&self) -> &[ModRouting] {
        &self.routings
    }

    /// Get a mutable slice of all routings.
    pub fn routings_mut(&mut self) -> &mut [ModRouting] {
        &mut self.routings
    }

    /// Number of routings currently configured.
    pub fn len(&self) -> usize {
        self.routings.len()
    }

    /// Whether the matrix has no routings.
    pub fn is_empty(&self) -> bool {
        self.routings.is_empty()
    }

    /// Evaluate all enabled routings given the current source values.
    ///
    /// Returns a [`ModOutput`] with the summed modulation amount per destination.
    /// Multiple sources routed to the same destination are summed additively.
    /// Disabled routings are skipped.
    pub fn evaluate(&self, sources: &ModSourceValues) -> ModOutput {
        let mut output = ModOutput::default();
        for routing in &self.routings {
            if !routing.enabled {
                continue;
            }
            let source_value = sources.get(routing.source);
            let mod_amount = source_value * routing.amount;
            output.values[routing.destination.index()] += mod_amount;
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_evaluate_single_routing() {
        let mut matrix = ModMatrix::new();
        matrix.add_routing(ModRouting::new(
            ModSource::Lfo1,
            ModDestination::FilterCutoff,
            1.0,
        ));

        let sources = ModSourceValues {
            lfo1: 0.75,
            ..Default::default()
        };

        let output = matrix.evaluate(&sources);
        assert!(
            (output.get(ModDestination::FilterCutoff) - 0.75).abs() < 1e-6,
            "LFO1 at 0.75 with amount 1.0 should produce 0.75 on FilterCutoff"
        );
        // Other destinations should be zero
        assert_eq!(output.get(ModDestination::Pitch), 0.0);
        assert_eq!(output.get(ModDestination::Volume), 0.0);
    }

    #[test]
    fn multiple_sources_to_same_destination_sum() {
        let mut matrix = ModMatrix::new();
        matrix.add_routing(ModRouting::new(ModSource::Lfo1, ModDestination::Pitch, 1.0));
        matrix.add_routing(ModRouting::new(
            ModSource::Velocity,
            ModDestination::Pitch,
            0.5,
        ));

        let sources = ModSourceValues {
            lfo1: 0.4,
            velocity: 0.8,
            ..Default::default()
        };

        let output = matrix.evaluate(&sources);
        // 0.4 * 1.0 + 0.8 * 0.5 = 0.4 + 0.4 = 0.8
        assert!(
            (output.get(ModDestination::Pitch) - 0.8).abs() < 1e-6,
            "Multiple sources should sum: got {}",
            output.get(ModDestination::Pitch)
        );
    }

    #[test]
    fn disabled_routing_is_skipped() {
        let mut matrix = ModMatrix::new();
        let mut routing = ModRouting::new(ModSource::Lfo1, ModDestination::Volume, 1.0);
        routing.enabled = false;
        matrix.add_routing(routing);

        let sources = ModSourceValues {
            lfo1: 1.0,
            ..Default::default()
        };

        let output = matrix.evaluate(&sources);
        assert_eq!(
            output.get(ModDestination::Volume),
            0.0,
            "Disabled routing should not contribute"
        );
    }

    #[test]
    fn amount_scaling() {
        let mut matrix = ModMatrix::new();
        matrix.add_routing(ModRouting::new(
            ModSource::ModWheel,
            ModDestination::FilterCutoff,
            0.5,
        ));

        let sources = ModSourceValues {
            mod_wheel: 1.0,
            ..Default::default()
        };

        let output = matrix.evaluate(&sources);
        assert!(
            (output.get(ModDestination::FilterCutoff) - 0.5).abs() < 1e-6,
            "Amount 0.5 should halve the source value"
        );
    }

    #[test]
    fn negative_amount_inverts() {
        let mut matrix = ModMatrix::new();
        matrix.add_routing(ModRouting::new(
            ModSource::AmpEnvelope,
            ModDestination::Pan,
            -0.8,
        ));

        let sources = ModSourceValues {
            amp_envelope: 1.0,
            ..Default::default()
        };

        let output = matrix.evaluate(&sources);
        assert!(
            (output.get(ModDestination::Pan) - (-0.8)).abs() < 1e-6,
            "Negative amount should invert: got {}",
            output.get(ModDestination::Pan)
        );
    }

    #[test]
    fn max_routings_enforced() {
        let mut matrix = ModMatrix::new();
        for _ in 0..MAX_ROUTINGS {
            assert!(matrix.add_routing(ModRouting::new(
                ModSource::Lfo1,
                ModDestination::Pitch,
                0.1,
            )));
        }
        // 17th should fail
        assert!(
            !matrix.add_routing(ModRouting::new(ModSource::Lfo1, ModDestination::Pitch, 0.1,)),
            "Should reject routing beyond MAX_ROUTINGS"
        );
        assert_eq!(matrix.len(), MAX_ROUTINGS);
    }

    #[test]
    fn remove_routing() {
        let mut matrix = ModMatrix::new();
        matrix.add_routing(ModRouting::new(ModSource::Lfo1, ModDestination::Pitch, 1.0));
        matrix.add_routing(ModRouting::new(
            ModSource::Lfo2,
            ModDestination::Volume,
            0.5,
        ));
        assert_eq!(matrix.len(), 2);

        let removed = matrix.remove_routing(0).unwrap();
        assert_eq!(removed.source, ModSource::Lfo1);
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix.routings()[0].source, ModSource::Lfo2);
    }

    #[test]
    fn remove_out_of_bounds_returns_none() {
        let mut matrix = ModMatrix::new();
        assert!(matrix.remove_routing(0).is_none());
        assert!(matrix.remove_routing(100).is_none());
    }

    #[test]
    fn clear_removes_all() {
        let mut matrix = ModMatrix::new();
        matrix.add_routing(ModRouting::new(ModSource::Lfo1, ModDestination::Pitch, 1.0));
        matrix.add_routing(ModRouting::new(
            ModSource::Lfo2,
            ModDestination::Volume,
            0.5,
        ));
        matrix.clear();
        assert!(matrix.is_empty());
        assert_eq!(matrix.len(), 0);
    }

    #[test]
    fn empty_matrix_produces_zero_output() {
        let matrix = ModMatrix::new();
        let sources = ModSourceValues {
            lfo1: 1.0,
            velocity: 0.9,
            mod_wheel: 0.5,
            ..Default::default()
        };
        let output = matrix.evaluate(&sources);
        for dest in &ModDestination::ALL {
            assert_eq!(output.get(*dest), 0.0);
        }
    }

    #[test]
    fn serde_roundtrip() {
        let mut matrix = ModMatrix::new();
        matrix.add_routing(ModRouting::new(
            ModSource::Lfo1,
            ModDestination::FilterCutoff,
            0.75,
        ));
        matrix.add_routing(ModRouting::new(
            ModSource::Velocity,
            ModDestination::Volume,
            -0.3,
        ));
        let mut disabled = ModRouting::new(ModSource::PitchBend, ModDestination::Pitch, 1.0);
        disabled.enabled = false;
        matrix.add_routing(disabled);

        let json = serde_json::to_string(&matrix).expect("serialize");
        let deserialized: ModMatrix = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.len(), 3);
        assert_eq!(deserialized.routings()[0].source, ModSource::Lfo1);
        assert_eq!(
            deserialized.routings()[0].destination,
            ModDestination::FilterCutoff
        );
        assert!((deserialized.routings()[0].amount - 0.75).abs() < 1e-6);
        assert!(deserialized.routings()[0].enabled);

        assert_eq!(deserialized.routings()[1].source, ModSource::Velocity);
        assert!((deserialized.routings()[1].amount - (-0.3)).abs() < 1e-6);

        assert!(!deserialized.routings()[2].enabled);
    }

    #[test]
    fn amount_clamped_on_construction() {
        let routing = ModRouting::new(ModSource::Lfo1, ModDestination::Pitch, 5.0);
        assert!(
            (routing.amount - 1.0).abs() < 1e-6,
            "Amount should clamp to 1.0"
        );

        let routing = ModRouting::new(ModSource::Lfo1, ModDestination::Pitch, -3.0);
        assert!(
            (routing.amount - (-1.0)).abs() < 1e-6,
            "Amount should clamp to -1.0"
        );
    }

    #[test]
    fn all_sources_read_correctly() {
        let sources = ModSourceValues {
            lfo1: 0.1,
            lfo2: 0.2,
            amp_envelope: 0.3,
            filter_envelope: 0.4,
            velocity: 0.5,
            mod_wheel: 0.6,
            aftertouch: 0.7,
            pitch_bend: -0.8,
        };

        assert!((sources.get(ModSource::Lfo1) - 0.1).abs() < 1e-6);
        assert!((sources.get(ModSource::Lfo2) - 0.2).abs() < 1e-6);
        assert!((sources.get(ModSource::AmpEnvelope) - 0.3).abs() < 1e-6);
        assert!((sources.get(ModSource::FilterEnvelope) - 0.4).abs() < 1e-6);
        assert!((sources.get(ModSource::Velocity) - 0.5).abs() < 1e-6);
        assert!((sources.get(ModSource::ModWheel) - 0.6).abs() < 1e-6);
        assert!((sources.get(ModSource::Aftertouch) - 0.7).abs() < 1e-6);
        assert!((sources.get(ModSource::PitchBend) - (-0.8)).abs() < 1e-6);
    }

    #[test]
    fn destination_indices_are_unique() {
        let mut seen = [false; NUM_DESTINATIONS];
        for dest in &ModDestination::ALL {
            let idx = dest.index();
            assert!(idx < NUM_DESTINATIONS, "Index out of range: {idx}");
            assert!(!seen[idx], "Duplicate index: {idx}");
            seen[idx] = true;
        }
    }

    #[test]
    fn complex_routing_scenario() {
        // Simulate a typical synth patch: velocity→volume, LFO1→cutoff, filter env→cutoff
        let mut matrix = ModMatrix::new();
        matrix.add_routing(ModRouting::new(
            ModSource::Velocity,
            ModDestination::Volume,
            1.0,
        ));
        matrix.add_routing(ModRouting::new(
            ModSource::Lfo1,
            ModDestination::FilterCutoff,
            0.3,
        ));
        matrix.add_routing(ModRouting::new(
            ModSource::FilterEnvelope,
            ModDestination::FilterCutoff,
            0.7,
        ));

        let sources = ModSourceValues {
            lfo1: 0.5,
            filter_envelope: 0.8,
            velocity: 0.9,
            ..Default::default()
        };

        let output = matrix.evaluate(&sources);

        // Volume: 0.9 * 1.0 = 0.9
        assert!(
            (output.get(ModDestination::Volume) - 0.9).abs() < 1e-6,
            "Volume: got {}",
            output.get(ModDestination::Volume)
        );

        // FilterCutoff: 0.5 * 0.3 + 0.8 * 0.7 = 0.15 + 0.56 = 0.71
        assert!(
            (output.get(ModDestination::FilterCutoff) - 0.71).abs() < 1e-6,
            "FilterCutoff: got {}",
            output.get(ModDestination::FilterCutoff)
        );
    }
}
