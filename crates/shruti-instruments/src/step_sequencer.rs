use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Step & PadSequence (existing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub active: bool,
    pub velocity: u8,
    pub probability: f32,
    pub accent: bool,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            active: false,
            velocity: 100,
            probability: 1.0,
            accent: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadSequence {
    pub steps: Vec<Step>,
    pub pad_index: usize,
}

impl PadSequence {
    pub fn new(pad_index: usize, step_count: usize) -> Self {
        Self {
            steps: (0..step_count).map(|_| Step::default()).collect(),
            pad_index,
        }
    }
}

// ---------------------------------------------------------------------------
// Pattern system: banks, patterns, chaining
// ---------------------------------------------------------------------------

/// Number of patterns per bank.
pub const PATTERNS_PER_BANK: usize = 16;
/// Number of banks.
pub const NUM_BANKS: usize = 4;

/// Pattern bank identifier (A/B/C/D).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternBank {
    A,
    B,
    C,
    D,
}

impl PatternBank {
    /// Convert bank to a flat index (0..3).
    pub fn index(self) -> usize {
        match self {
            PatternBank::A => 0,
            PatternBank::B => 1,
            PatternBank::C => 2,
            PatternBank::D => 3,
        }
    }

    /// Create from a flat index. Returns `None` for out-of-range values.
    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(PatternBank::A),
            1 => Some(PatternBank::B),
            2 => Some(PatternBank::C),
            3 => Some(PatternBank::D),
            _ => None,
        }
    }
}

/// A named pattern holding one `PadSequence` per pad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Human-readable name.
    pub name: String,
    /// Pad sequences (one per pad, indexed by pad number).
    pub sequences: Vec<PadSequence>,
}

impl Pattern {
    /// Create an empty pattern with default sequences for all pads.
    pub fn new(name: &str, step_count: usize, num_pads: usize) -> Self {
        let sequences = (0..num_pads)
            .map(|i| PadSequence::new(i, step_count))
            .collect();
        Self {
            name: name.to_string(),
            sequences,
        }
    }
}

/// Reference to a specific pattern: (bank, index within bank).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternId {
    pub bank: PatternBank,
    pub index: usize,
}

impl PatternId {
    pub fn new(bank: PatternBank, index: usize) -> Self {
        Self { bank, index }
    }
}

/// An ordered list of pattern references for song-mode playback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternChain {
    /// The ordered sequence of patterns to play.
    pub entries: Vec<PatternId>,
    /// Current position in the chain (used during playback).
    chain_position: usize,
}

impl PatternChain {
    pub fn new(entries: Vec<PatternId>) -> Self {
        Self {
            entries,
            chain_position: 0,
        }
    }

    /// Returns the current pattern in the chain, or `None` if the chain is empty.
    pub fn current(&self) -> Option<PatternId> {
        self.entries.get(self.chain_position).copied()
    }

    /// Advance to the next pattern, wrapping around. Returns the new current pattern.
    pub fn advance(&mut self) -> Option<PatternId> {
        if self.entries.is_empty() {
            return None;
        }
        self.chain_position = (self.chain_position + 1) % self.entries.len();
        self.current()
    }

    /// Reset chain playback to the beginning.
    pub fn reset(&mut self) {
        self.chain_position = 0;
    }

    /// Current position index in the chain.
    pub fn position(&self) -> usize {
        self.chain_position
    }

    /// Number of entries in the chain.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// StepSequencer (extended with pattern support)
// ---------------------------------------------------------------------------

pub struct StepSequencer {
    /// The currently active pad sequences (loaded from the active pattern).
    pub sequences: Vec<PadSequence>,
    pub step_count: usize,
    pub swing: f32,
    current_step: usize,
    samples_per_step: f64,
    sample_counter: f64,
    sample_rate: f32,
    bpm: f64,
    // Simple xorshift RNG state
    rng_state: u32,

    // --- Pattern storage ---
    /// 4 banks x 16 patterns = 64 patterns total.
    patterns: Vec<Vec<Pattern>>,
    /// Currently selected pattern.
    active_pattern: PatternId,
    /// Optional chain for song-mode playback.
    chain: Option<PatternChain>,
}

impl StepSequencer {
    pub fn new(step_count: usize, sample_rate: f32, bpm: f64) -> Self {
        let num_pads = crate::drum_machine::NUM_PADS;
        let sequences = (0..num_pads)
            .map(|i| PadSequence::new(i, step_count))
            .collect();

        let samples_per_step = sample_rate as f64 * 60.0 / bpm / 4.0;

        // Initialize all 64 patterns (4 banks x 16)
        let patterns = (0..NUM_BANKS)
            .map(|bank_idx| {
                let bank_letter = match bank_idx {
                    0 => 'A',
                    1 => 'B',
                    2 => 'C',
                    _ => (b'A' + bank_idx as u8) as char,
                };
                (0..PATTERNS_PER_BANK)
                    .map(|pat_idx| {
                        Pattern::new(
                            &format!("{bank_letter}{}", pat_idx + 1),
                            step_count,
                            num_pads,
                        )
                    })
                    .collect()
            })
            .collect();

        Self {
            sequences,
            step_count,
            swing: 0.0,
            current_step: 0,
            samples_per_step,
            sample_counter: 0.0,
            sample_rate,
            bpm,
            rng_state: 12345,
            patterns,
            active_pattern: PatternId::new(PatternBank::A, 0),
            chain: None,
        }
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm;
        self.samples_per_step = self.sample_rate as f64 * 60.0 / bpm / 4.0;
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
        self.samples_per_step = sr as f64 * 60.0 / self.bpm / 4.0;
    }

    // --- Pattern API ---

    /// Save the current live sequences into the active pattern slot.
    fn save_active_pattern(&mut self) {
        let id = self.active_pattern;
        self.patterns[id.bank.index()][id.index].sequences = self.sequences.clone();
    }

    /// Load sequences from a pattern slot into the live sequences.
    fn load_pattern(&mut self, id: PatternId) {
        self.sequences = self.patterns[id.bank.index()][id.index].sequences.clone();
    }

    /// Select and load a pattern by bank and index (0..15).
    /// Saves the current pattern first.
    /// Returns `false` if the index is out of range.
    pub fn select_pattern(&mut self, bank: PatternBank, index: usize) -> bool {
        if index >= PATTERNS_PER_BANK {
            return false;
        }
        self.save_active_pattern();
        let id = PatternId::new(bank, index);
        self.load_pattern(id);
        self.active_pattern = id;
        true
    }

    /// Copy a pattern from one slot to another.
    /// Returns `false` if either index is out of range.
    pub fn copy_pattern(
        &mut self,
        from_bank: PatternBank,
        from_idx: usize,
        to_bank: PatternBank,
        to_idx: usize,
    ) -> bool {
        if from_idx >= PATTERNS_PER_BANK || to_idx >= PATTERNS_PER_BANK {
            return false;
        }
        // Save current live data first so it's not stale
        self.save_active_pattern();
        let src = self.patterns[from_bank.index()][from_idx].clone();
        self.patterns[to_bank.index()][to_idx] = src;
        // If the destination is the currently active pattern, reload
        let dest_id = PatternId::new(to_bank, to_idx);
        if self.active_pattern == dest_id {
            self.load_pattern(dest_id);
        }
        true
    }

    /// Get a reference to a pattern.
    pub fn pattern(&self, bank: PatternBank, index: usize) -> Option<&Pattern> {
        if index >= PATTERNS_PER_BANK {
            return None;
        }
        Some(&self.patterns[bank.index()][index])
    }

    /// Get a mutable reference to a pattern.
    pub fn pattern_mut(&mut self, bank: PatternBank, index: usize) -> Option<&mut Pattern> {
        if index >= PATTERNS_PER_BANK {
            return None;
        }
        Some(&mut self.patterns[bank.index()][index])
    }

    /// The currently active pattern id.
    pub fn active_pattern(&self) -> PatternId {
        self.active_pattern
    }

    // --- Chain / Song-mode API ---

    /// Set the pattern chain for song-mode playback.
    /// All entries are validated; returns `false` if any index is out of range.
    pub fn set_chain(&mut self, entries: Vec<PatternId>) -> bool {
        for entry in &entries {
            if entry.index >= PATTERNS_PER_BANK {
                return false;
            }
        }
        self.chain = Some(PatternChain::new(entries));
        true
    }

    /// Get a reference to the current chain, if set.
    pub fn chain(&self) -> Option<&PatternChain> {
        self.chain.as_ref()
    }

    /// Clear the chain (exit song mode).
    pub fn clear_chain(&mut self) {
        self.chain = None;
    }

    /// Advance to the next pattern in the chain and load it.
    /// Returns the new `PatternId`, or `None` if no chain is set or chain is empty.
    pub fn next_pattern_in_chain(&mut self) -> Option<PatternId> {
        self.save_active_pattern();
        let next_id = self.chain.as_mut()?.advance()?;
        self.load_pattern(next_id);
        self.active_pattern = next_id;
        Some(next_id)
    }

    // --- Tick / playback ---

    /// Advance by one sample. Returns list of (pad_index, velocity) triggers.
    pub fn tick(&mut self) -> Vec<(usize, u8)> {
        let mut triggers = Vec::new();

        // Calculate the threshold for this step, accounting for swing
        let threshold = if self.current_step % 2 == 1 {
            // Odd steps (even-indexed in musical terms: the "off-beats") get swing delay
            self.samples_per_step + self.swing as f64 * self.samples_per_step * 0.5
        } else {
            self.samples_per_step
        };

        self.sample_counter += 1.0;

        if self.sample_counter >= threshold {
            self.sample_counter -= threshold;

            let current_step = self.current_step;

            // Check each pad's sequence for this step
            for seq in &self.sequences {
                if current_step < seq.steps.len() {
                    let step = &seq.steps[current_step];
                    if step.active && check_probability(&mut self.rng_state, step.probability) {
                        let vel = if step.accent { 127 } else { step.velocity };
                        triggers.push((seq.pad_index, vel));
                    }
                }
            }

            self.current_step = (current_step + 1) % self.step_count;
        }

        triggers
    }

    pub fn reset(&mut self) {
        self.current_step = 0;
        self.sample_counter = 0.0;
    }

    pub fn set_step(&mut self, pad: usize, step: usize, active: bool, velocity: u8) {
        if pad < self.sequences.len() && step < self.sequences[pad].steps.len() {
            self.sequences[pad].steps[step].active = active;
            self.sequences[pad].steps[step].velocity = velocity;
        }
    }

    pub fn current_step(&self) -> usize {
        self.current_step
    }
}

/// Simple xorshift-based probability check.
fn check_probability(rng_state: &mut u32, probability: f32) -> bool {
    if probability >= 1.0 {
        return true;
    }
    if probability <= 0.0 {
        return false;
    }
    // xorshift32
    let mut x = *rng_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *rng_state = x;
    let random = (x as f32) / (u32::MAX as f32);
    random < probability
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_triggers_at_correct_timing() {
        let sample_rate = 44100.0;
        let bpm = 120.0;
        let mut seq = StepSequencer::new(16, sample_rate, bpm);
        // samples_per_step = 44100 * 60 / 120 / 4 = 5512.5

        // Activate step 0 on pad 0
        seq.set_step(0, 0, true, 100);
        // Activate step 1 on pad 0
        seq.set_step(0, 1, true, 80);

        let mut step0_triggered = false;
        let mut step1_triggered = false;
        let mut step0_frame = 0;
        let mut step1_frame = 0;

        // Run through enough samples for 2 steps
        for i in 0..12000 {
            let triggers = seq.tick();
            for &(pad, vel) in &triggers {
                if pad == 0 && vel == 100 && !step0_triggered {
                    step0_triggered = true;
                    step0_frame = i;
                } else if pad == 0 && vel == 80 && !step1_triggered {
                    step1_triggered = true;
                    step1_frame = i;
                }
            }
        }

        assert!(step0_triggered, "step 0 should trigger");
        assert!(step1_triggered, "step 1 should trigger");
        // Step 0 triggers at ~5512 samples, step 1 at ~11025
        let diff = step1_frame - step0_frame;
        assert!(
            (diff as f64 - 5512.5).abs() < 2.0,
            "steps should be ~5512.5 samples apart, got {diff}"
        );
    }

    #[test]
    fn swing_offsets_odd_steps() {
        let sample_rate = 44100.0;
        let bpm = 120.0;

        // Without swing
        let mut seq_no_swing = StepSequencer::new(16, sample_rate, bpm);
        seq_no_swing.set_step(0, 0, true, 100);
        seq_no_swing.set_step(0, 1, true, 100);

        let mut no_swing_frames = Vec::new();
        for i in 0..15000 {
            let triggers = seq_no_swing.tick();
            if triggers.iter().any(|&(p, _)| p == 0) {
                no_swing_frames.push(i);
            }
        }

        // With swing
        let mut seq_swing = StepSequencer::new(16, sample_rate, bpm);
        seq_swing.swing = 0.5;
        seq_swing.set_step(0, 0, true, 100);
        seq_swing.set_step(0, 1, true, 100);

        let mut swing_frames = Vec::new();
        for i in 0..15000 {
            let triggers = seq_swing.tick();
            if triggers.iter().any(|&(p, _)| p == 0) {
                swing_frames.push(i);
            }
        }

        assert!(
            no_swing_frames.len() >= 2,
            "should have at least 2 triggers without swing"
        );
        assert!(
            swing_frames.len() >= 2,
            "should have at least 2 triggers with swing"
        );

        // The second trigger (odd step) should be delayed with swing
        if swing_frames.len() >= 2 && no_swing_frames.len() >= 2 {
            let swing_gap = swing_frames[1] - swing_frames[0];
            let no_swing_gap = no_swing_frames[1] - no_swing_frames[0];
            assert!(
                swing_gap > no_swing_gap,
                "swing should delay the odd step: swing_gap={swing_gap}, no_swing_gap={no_swing_gap}"
            );
        }
    }

    #[test]
    fn probability_zero_never_triggers() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        seq.set_step(0, 0, true, 100);
        seq.sequences[0].steps[0].probability = 0.0;

        let mut triggered = false;
        for _ in 0..50000 {
            let triggers = seq.tick();
            if triggers.iter().any(|&(p, _)| p == 0) {
                triggered = true;
                break;
            }
        }
        assert!(!triggered, "probability=0 should never trigger");
    }

    #[test]
    fn probability_one_always_triggers() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        seq.set_step(0, 0, true, 100);
        seq.sequences[0].steps[0].probability = 1.0;

        let mut trigger_count = 0;
        // Run for enough samples to hit step 0 multiple times (16 steps per cycle)
        // At 120 BPM, 16 steps = ~88200 samples
        for _ in 0..200000 {
            let triggers = seq.tick();
            if triggers.iter().any(|&(p, _)| p == 0) {
                trigger_count += 1;
            }
        }
        // Should trigger at least twice (going through the sequence multiple times)
        assert!(
            trigger_count >= 2,
            "probability=1 should always trigger, got {trigger_count} triggers"
        );
    }

    #[test]
    fn set_step_works() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        assert!(!seq.sequences[0].steps[3].active);
        seq.set_step(0, 3, true, 90);
        assert!(seq.sequences[0].steps[3].active);
        assert_eq!(seq.sequences[0].steps[3].velocity, 90);
    }

    #[test]
    fn reset_goes_to_step_zero() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        seq.set_step(0, 0, true, 100);

        // Advance past step 0
        for _ in 0..10000 {
            seq.tick();
        }
        assert!(seq.current_step() > 0, "should have advanced past step 0");

        seq.reset();
        assert_eq!(seq.current_step(), 0);
    }

    #[test]
    fn bpm_change_updates_timing() {
        let sample_rate = 44100.0;

        // At 120 BPM
        let mut seq_slow = StepSequencer::new(16, sample_rate, 120.0);
        seq_slow.set_step(0, 0, true, 100);
        seq_slow.set_step(0, 1, true, 100);

        let mut slow_frames = Vec::new();
        for i in 0..20000 {
            let triggers = seq_slow.tick();
            if triggers.iter().any(|&(p, _)| p == 0) {
                slow_frames.push(i);
            }
        }

        // At 240 BPM (double tempo)
        let mut seq_fast = StepSequencer::new(16, sample_rate, 120.0);
        seq_fast.set_bpm(240.0);
        seq_fast.set_step(0, 0, true, 100);
        seq_fast.set_step(0, 1, true, 100);

        let mut fast_frames = Vec::new();
        for i in 0..20000 {
            let triggers = seq_fast.tick();
            if triggers.iter().any(|&(p, _)| p == 0) {
                fast_frames.push(i);
            }
        }

        assert!(slow_frames.len() >= 2);
        assert!(fast_frames.len() >= 2);

        let slow_gap = slow_frames[1] - slow_frames[0];
        let fast_gap = fast_frames[1] - fast_frames[0];

        // Double BPM should halve the gap
        let ratio = slow_gap as f64 / fast_gap as f64;
        assert!(
            (ratio - 2.0).abs() < 0.1,
            "double BPM should halve step gap: ratio={ratio}"
        );
    }

    #[test]
    fn step_count_32_works() {
        let mut seq = StepSequencer::new(32, 44100.0, 120.0);
        assert_eq!(seq.step_count, 32);
        assert_eq!(seq.sequences[0].steps.len(), 32);

        seq.set_step(0, 31, true, 100);
        assert!(seq.sequences[0].steps[31].active);
    }

    #[test]
    fn step_count_64_works() {
        let mut seq = StepSequencer::new(64, 44100.0, 120.0);
        assert_eq!(seq.step_count, 64);
        assert_eq!(seq.sequences[0].steps.len(), 64);

        seq.set_step(0, 63, true, 100);
        assert!(seq.sequences[0].steps[63].active);
    }

    #[test]
    fn step_default_values() {
        let step = Step::default();
        assert!(!step.active);
        assert_eq!(step.velocity, 100);
        assert!((step.probability - 1.0).abs() < f32::EPSILON);
        assert!(!step.accent);
    }

    #[test]
    fn accent_overrides_velocity() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        seq.set_step(0, 0, true, 50);
        seq.sequences[0].steps[0].accent = true;

        let mut triggered_velocity = None;
        for _ in 0..10000 {
            let triggers = seq.tick();
            for &(pad, vel) in &triggers {
                if pad == 0 {
                    triggered_velocity = Some(vel);
                    break;
                }
            }
            if triggered_velocity.is_some() {
                break;
            }
        }
        assert_eq!(
            triggered_velocity,
            Some(127),
            "accent should override velocity to 127"
        );
    }

    // -----------------------------------------------------------------------
    // Pattern system tests
    // -----------------------------------------------------------------------

    #[test]
    fn pattern_bank_index_roundtrip() {
        for i in 0..4 {
            let bank = PatternBank::from_index(i).unwrap();
            assert_eq!(bank.index(), i);
        }
        assert!(PatternBank::from_index(4).is_none());
        assert!(PatternBank::from_index(100).is_none());
    }

    #[test]
    fn initial_active_pattern_is_a0() {
        let seq = StepSequencer::new(16, 44100.0, 120.0);
        let id = seq.active_pattern();
        assert_eq!(id.bank, PatternBank::A);
        assert_eq!(id.index, 0);
    }

    #[test]
    fn select_pattern_switches_data() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);

        // Write a step in current pattern (A0)
        seq.set_step(0, 0, true, 100);
        assert!(seq.sequences[0].steps[0].active);

        // Switch to A1
        assert!(seq.select_pattern(PatternBank::A, 1));
        assert_eq!(seq.active_pattern().index, 1);
        // A1 should be empty
        assert!(!seq.sequences[0].steps[0].active);

        // Switch back to A0 — should still have data
        assert!(seq.select_pattern(PatternBank::A, 0));
        assert!(seq.sequences[0].steps[0].active);
        assert_eq!(seq.sequences[0].steps[0].velocity, 100);
    }

    #[test]
    fn select_pattern_invalid_index_returns_false() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        assert!(!seq.select_pattern(PatternBank::A, 16));
        assert!(!seq.select_pattern(PatternBank::D, 99));
        // Active pattern should be unchanged
        assert_eq!(seq.active_pattern(), PatternId::new(PatternBank::A, 0));
    }

    #[test]
    fn select_pattern_across_banks() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);

        seq.set_step(2, 5, true, 80);
        assert!(seq.select_pattern(PatternBank::C, 3));
        assert_eq!(seq.active_pattern().bank, PatternBank::C);
        assert!(!seq.sequences[2].steps[5].active);

        // Go back
        assert!(seq.select_pattern(PatternBank::A, 0));
        assert!(seq.sequences[2].steps[5].active);
    }

    #[test]
    fn copy_pattern_duplicates_data() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);

        // Set up some data in A0
        seq.set_step(0, 0, true, 110);
        seq.set_step(1, 3, true, 70);

        // Copy A0 -> B5
        assert!(seq.copy_pattern(PatternBank::A, 0, PatternBank::B, 5));

        // Switch to B5 and verify
        assert!(seq.select_pattern(PatternBank::B, 5));
        assert!(seq.sequences[0].steps[0].active);
        assert_eq!(seq.sequences[0].steps[0].velocity, 110);
        assert!(seq.sequences[1].steps[3].active);
        assert_eq!(seq.sequences[1].steps[3].velocity, 70);
    }

    #[test]
    fn copy_pattern_invalid_index_returns_false() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        assert!(!seq.copy_pattern(PatternBank::A, 16, PatternBank::B, 0));
        assert!(!seq.copy_pattern(PatternBank::A, 0, PatternBank::B, 16));
    }

    #[test]
    fn copy_pattern_to_active_reloads() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);

        // Put data in B0
        assert!(seq.select_pattern(PatternBank::B, 0));
        seq.set_step(3, 7, true, 90);

        // Go back to A0 (empty)
        assert!(seq.select_pattern(PatternBank::A, 0));
        assert!(!seq.sequences[3].steps[7].active);

        // Copy B0 -> A0 (active) — should reload live sequences
        assert!(seq.copy_pattern(PatternBank::B, 0, PatternBank::A, 0));
        assert!(seq.sequences[3].steps[7].active);
        assert_eq!(seq.sequences[3].steps[7].velocity, 90);
    }

    #[test]
    fn pattern_names_are_generated() {
        let seq = StepSequencer::new(16, 44100.0, 120.0);
        assert_eq!(seq.pattern(PatternBank::A, 0).unwrap().name, "A1");
        assert_eq!(seq.pattern(PatternBank::A, 15).unwrap().name, "A16");
        assert_eq!(seq.pattern(PatternBank::D, 0).unwrap().name, "D1");
        assert!(seq.pattern(PatternBank::A, 16).is_none());
    }

    #[test]
    fn chain_creation_and_traversal() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);

        let entries = vec![
            PatternId::new(PatternBank::A, 0),
            PatternId::new(PatternBank::A, 1),
            PatternId::new(PatternBank::B, 3),
        ];
        assert!(seq.set_chain(entries.clone()));
        assert!(seq.chain().is_some());

        let chain = seq.chain().unwrap();
        assert_eq!(chain.len(), 3);
        assert!(!chain.is_empty());
        assert_eq!(chain.position(), 0);
        assert_eq!(chain.current(), Some(entries[0]));
    }

    #[test]
    fn chain_advance_cycles() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);

        // Set different data in each pattern we'll chain
        seq.set_step(0, 0, true, 100); // A0
        seq.select_pattern(PatternBank::A, 1);
        seq.set_step(0, 1, true, 80); // A1

        seq.select_pattern(PatternBank::A, 0);

        let entries = vec![
            PatternId::new(PatternBank::A, 0),
            PatternId::new(PatternBank::A, 1),
        ];
        assert!(seq.set_chain(entries));

        // Advance: A0 -> A1
        let next = seq.next_pattern_in_chain().unwrap();
        assert_eq!(next, PatternId::new(PatternBank::A, 1));
        assert!(seq.sequences[0].steps[1].active); // A1 data loaded

        // Advance: A1 -> A0 (wraps)
        let next = seq.next_pattern_in_chain().unwrap();
        assert_eq!(next, PatternId::new(PatternBank::A, 0));
        assert!(seq.sequences[0].steps[0].active); // A0 data loaded
    }

    #[test]
    fn chain_empty_returns_none() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        assert!(seq.set_chain(vec![]));
        assert_eq!(seq.next_pattern_in_chain(), None);
        assert_eq!(seq.chain().unwrap().current(), None);
    }

    #[test]
    fn chain_invalid_index_rejected() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        let entries = vec![
            PatternId::new(PatternBank::A, 0),
            PatternId::new(PatternBank::B, 16), // invalid
        ];
        assert!(!seq.set_chain(entries));
        assert!(seq.chain().is_none());
    }

    #[test]
    fn clear_chain_removes_chain() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        let entries = vec![PatternId::new(PatternBank::A, 0)];
        assert!(seq.set_chain(entries));
        assert!(seq.chain().is_some());
        seq.clear_chain();
        assert!(seq.chain().is_none());
    }

    #[test]
    fn no_chain_next_returns_none() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        assert_eq!(seq.next_pattern_in_chain(), None);
    }

    #[test]
    fn pattern_chain_reset() {
        let mut chain = PatternChain::new(vec![
            PatternId::new(PatternBank::A, 0),
            PatternId::new(PatternBank::A, 1),
            PatternId::new(PatternBank::A, 2),
        ]);
        chain.advance();
        chain.advance();
        assert_eq!(chain.position(), 2);
        chain.reset();
        assert_eq!(chain.position(), 0);
        assert_eq!(chain.current(), Some(PatternId::new(PatternBank::A, 0)));
    }

    #[test]
    fn pattern_and_chain_serde() {
        let pattern = Pattern::new("Test", 16, 8);
        let json = serde_json::to_string(&pattern).unwrap();
        let deser: Pattern = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.name, "Test");
        assert_eq!(deser.sequences.len(), 8);

        let chain = PatternChain::new(vec![
            PatternId::new(PatternBank::B, 5),
            PatternId::new(PatternBank::D, 15),
        ]);
        let json = serde_json::to_string(&chain).unwrap();
        let deser: PatternChain = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.entries.len(), 2);
        assert_eq!(deser.entries[0].bank, PatternBank::B);
        assert_eq!(deser.entries[1].index, 15);
    }

    // ── Timing accuracy at various BPMs ──────────────────────────────

    fn collect_trigger_frames(seq: &mut StepSequencer, num_samples: usize) -> Vec<usize> {
        let mut frames = Vec::new();
        for i in 0..num_samples {
            let triggers = seq.tick();
            if triggers.iter().any(|&(p, _)| p == 0) {
                frames.push(i);
            }
        }
        frames
    }

    #[test]
    fn timing_accuracy_60_bpm() {
        let sr = 44100.0_f32;
        let bpm = 60.0;
        let expected_sps = sr as f64 * 60.0 / bpm / 4.0;

        let mut seq = StepSequencer::new(16, sr, bpm);
        for s in 0..4 {
            seq.set_step(0, s, true, 100);
        }

        let frames = collect_trigger_frames(&mut seq, 50000);
        assert!(frames.len() >= 4);
        for w in frames.windows(2) {
            let gap = w[1] - w[0];
            assert!(
                (gap as f64 - expected_sps).abs() < 2.0,
                "60 BPM: expected step gap ~{expected_sps}, got {gap}"
            );
        }
    }

    #[test]
    fn timing_accuracy_120_bpm() {
        let sr = 44100.0_f32;
        let bpm = 120.0;
        let expected_sps = sr as f64 * 60.0 / bpm / 4.0;

        let mut seq = StepSequencer::new(16, sr, bpm);
        for s in 0..4 {
            seq.set_step(0, s, true, 100);
        }

        let frames = collect_trigger_frames(&mut seq, 25000);
        assert!(frames.len() >= 4);
        for w in frames.windows(2) {
            let gap = w[1] - w[0];
            assert!(
                (gap as f64 - expected_sps).abs() < 2.0,
                "120 BPM: expected step gap ~{expected_sps}, got {gap}"
            );
        }
    }

    #[test]
    fn timing_accuracy_180_bpm() {
        let sr = 44100.0_f32;
        let bpm = 180.0;
        let expected_sps = sr as f64 * 60.0 / bpm / 4.0;

        let mut seq = StepSequencer::new(16, sr, bpm);
        for s in 0..4 {
            seq.set_step(0, s, true, 100);
        }

        let frames = collect_trigger_frames(&mut seq, 20000);
        assert!(frames.len() >= 4);
        for w in frames.windows(2) {
            let gap = w[1] - w[0];
            assert!(
                (gap as f64 - expected_sps).abs() < 2.0,
                "180 BPM: expected step gap ~{expected_sps}, got {gap}"
            );
        }
    }

    #[test]
    fn swing_50_percent_is_straight() {
        let sr = 44100.0_f32;
        let bpm = 120.0;
        let expected_sps = sr as f64 * 60.0 / bpm / 4.0;

        let mut seq = StepSequencer::new(16, sr, bpm);
        seq.swing = 0.0;
        for s in 0..4 {
            seq.set_step(0, s, true, 100);
        }

        let frames = collect_trigger_frames(&mut seq, 25000);
        assert!(frames.len() >= 3);
        for w in frames.windows(2) {
            let gap = w[1] - w[0];
            assert!(
                (gap as f64 - expected_sps).abs() < 2.0,
                "straight swing: all gaps should be ~{expected_sps}, got {gap}"
            );
        }
    }

    #[test]
    fn swing_67_percent_triplet_feel() {
        let sr = 44100.0_f32;
        let bpm = 120.0;
        let sps = sr as f64 * 60.0 / bpm / 4.0;
        let swing = 0.34_f32;

        let mut seq = StepSequencer::new(16, sr, bpm);
        seq.swing = swing;
        for s in 0..6 {
            seq.set_step(0, s, true, 100);
        }

        let frames = collect_trigger_frames(&mut seq, 40000);
        assert!(frames.len() >= 4);

        let gap_0_to_1 = frames[1] - frames[0];
        let expected_odd_threshold = sps + swing as f64 * sps * 0.5;
        assert!(
            (gap_0_to_1 as f64 - expected_odd_threshold).abs() < 2.0,
            "swing gap even->odd: expected ~{expected_odd_threshold}, got {gap_0_to_1}"
        );

        let gap_1_to_2 = frames[2] - frames[1];
        assert!(
            (gap_1_to_2 as f64 - sps).abs() < 2.0,
            "swing gap odd->even: expected ~{sps}, got {gap_1_to_2}"
        );
    }

    #[test]
    fn swing_full_maximum_delay() {
        let sr = 44100.0_f32;
        let bpm = 120.0;
        let sps = sr as f64 * 60.0 / bpm / 4.0;
        let expected_odd = sps * 1.5;

        let mut seq = StepSequencer::new(16, sr, bpm);
        seq.swing = 1.0;
        for s in 0..4 {
            seq.set_step(0, s, true, 100);
        }

        let frames = collect_trigger_frames(&mut seq, 30000);
        assert!(frames.len() >= 3);

        let gap_even_to_odd = frames[1] - frames[0];
        assert!(
            (gap_even_to_odd as f64 - expected_odd).abs() < 2.0,
            "max swing even->odd: expected ~{expected_odd}, got {gap_even_to_odd}"
        );
    }

    #[test]
    fn probability_half_fires_roughly_50_percent() {
        let sr = 44100.0_f32;
        let bpm = 120.0;
        let sps = (sr as f64 * 60.0 / bpm / 4.0) as usize;

        let mut fire_count = 0_usize;
        let iterations = 1000;

        for i in 0..iterations {
            let mut seq = StepSequencer::new(16, sr, bpm);
            seq.set_step(0, 0, true, 100);
            seq.sequences[0].steps[0].probability = 0.5;
            seq.rng_state = 12345_u32.wrapping_add(i as u32 * 7919);

            let mut fired = false;
            for _ in 0..(sps + 2) {
                let triggers = seq.tick();
                if triggers.iter().any(|&(p, _)| p == 0) {
                    fired = true;
                    break;
                }
            }
            if fired {
                fire_count += 1;
            }
        }

        let ratio = fire_count as f64 / iterations as f64;
        assert!(
            (0.40..=0.60).contains(&ratio),
            "probability=0.5 should fire ~50%, got {:.1}% ({fire_count}/{iterations})",
            ratio * 100.0
        );
    }

    #[test]
    fn disabled_step_does_not_fire() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        seq.set_step(0, 0, true, 100);
        seq.set_step(0, 1, false, 100);

        let mut step1_fired = false;
        let mut past_step1 = false;

        for _ in 0..20000 {
            let triggers = seq.tick();
            if seq.current_step() > 2 {
                past_step1 = true;
            }
            for &(pad, _vel) in &triggers {
                if pad == 0 && seq.current_step() == 2 {
                    step1_fired = true;
                }
            }
            if past_step1 {
                break;
            }
        }
        assert!(!step1_fired, "disabled step 1 should not fire");
    }

    #[test]
    fn all_steps_disabled_by_default() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        let mut any_trigger = false;
        for _ in 0..100000 {
            let triggers = seq.tick();
            if !triggers.is_empty() {
                any_trigger = true;
                break;
            }
        }
        assert!(!any_trigger, "no triggers when all steps disabled");
    }

    #[test]
    fn disable_previously_active_step() {
        let sr = 44100.0_f32;
        let sps = (sr as f64 * 60.0 / 120.0 / 4.0) as usize;

        let mut seq = StepSequencer::new(16, sr, 120.0);
        seq.set_step(0, 0, true, 100);

        let mut fired = false;
        for _ in 0..(sps + 2) {
            if !seq.tick().is_empty() {
                fired = true;
                break;
            }
        }
        assert!(fired);

        seq.set_step(0, 0, false, 100);
        seq.reset();

        let mut fired_after = false;
        for _ in 0..(sps + 2) {
            if !seq.tick().is_empty() {
                fired_after = true;
                break;
            }
        }
        assert!(!fired_after, "disabled step should not fire");
    }

    #[test]
    fn accent_produces_velocity_127_regardless_of_set_velocity() {
        for base_vel in [1_u8, 50, 80, 100, 126] {
            let mut seq = StepSequencer::new(16, 44100.0, 120.0);
            seq.set_step(0, 0, true, base_vel);
            seq.sequences[0].steps[0].accent = true;

            for _ in 0..10000 {
                let triggers = seq.tick();
                for &(pad, vel) in &triggers {
                    if pad == 0 {
                        assert_eq!(vel, 127, "accent vel should be 127 for base {base_vel}");
                        break;
                    }
                }
            }
        }
    }

    #[test]
    fn non_accent_uses_set_velocity() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        seq.set_step(0, 0, true, 73);

        let mut found_vel = None;
        for _ in 0..10000 {
            for &(pad, vel) in &seq.tick() {
                if pad == 0 {
                    found_vel = Some(vel);
                    break;
                }
            }
            if found_vel.is_some() {
                break;
            }
        }
        assert_eq!(found_vel, Some(73));
    }

    #[test]
    fn accent_step_has_higher_velocity_than_non_accent() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        seq.set_step(0, 0, true, 60);
        seq.set_step(0, 1, true, 60);
        seq.sequences[0].steps[1].accent = true;

        let mut vel_step0 = None;
        let mut vel_step1 = None;

        for _ in 0..15000 {
            for &(pad, vel) in &seq.tick() {
                if pad == 0 {
                    if vel_step0.is_none() {
                        vel_step0 = Some(vel);
                    } else if vel_step1.is_none() {
                        vel_step1 = Some(vel);
                    }
                }
            }
            if vel_step0.is_some() && vel_step1.is_some() {
                break;
            }
        }

        assert_eq!(vel_step0.unwrap(), 60);
        assert_eq!(vel_step1.unwrap(), 127);
    }

    #[test]
    fn pattern_16_wraps_correctly() {
        let sr = 44100.0_f32;
        let sps = sr as f64 * 60.0 / 120.0 / 4.0;

        let mut seq = StepSequencer::new(16, sr, 120.0);
        seq.set_step(0, 0, true, 100);

        let total = (sps * 33.0) as usize;
        let frames = collect_trigger_frames(&mut seq, total);
        assert!(frames.len() >= 2);

        let cycle_gap = frames[1] - frames[0];
        let expected = sps * 16.0;
        assert!((cycle_gap as f64 - expected).abs() < 20.0);
    }

    #[test]
    fn pattern_32_wraps_correctly() {
        let sr = 44100.0_f32;
        let sps = sr as f64 * 60.0 / 120.0 / 4.0;

        let mut seq = StepSequencer::new(32, sr, 120.0);
        seq.set_step(0, 0, true, 100);

        let total = (sps * 65.0) as usize;
        let frames = collect_trigger_frames(&mut seq, total);
        assert!(frames.len() >= 2);

        let cycle_gap = frames[1] - frames[0];
        let expected = sps * 32.0;
        assert!((cycle_gap as f64 - expected).abs() < 20.0);
    }

    #[test]
    fn pattern_64_wraps_correctly() {
        let sr = 44100.0_f32;
        let sps = sr as f64 * 60.0 / 120.0 / 4.0;

        let mut seq = StepSequencer::new(64, sr, 120.0);
        seq.set_step(0, 0, true, 100);

        let total = (sps * 129.0) as usize;
        let frames = collect_trigger_frames(&mut seq, total);
        assert!(frames.len() >= 2);

        let cycle_gap = frames[1] - frames[0];
        let expected = sps * 64.0;
        assert!((cycle_gap as f64 - expected).abs() < 40.0);
    }

    #[test]
    fn pattern_wraps_step_counter() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        for s in 0..16 {
            seq.set_step(0, s, true, 100);
        }

        let sps = (44100.0_f64 * 60.0 / 120.0 / 4.0) as usize;
        for _ in 0..(sps * 17 + 100) {
            seq.tick();
        }

        assert!(seq.current_step() < 16);
    }

    #[test]
    fn reset_clears_sample_counter() {
        let sr = 44100.0_f32;
        let sps = sr as f64 * 60.0 / 120.0 / 4.0;

        let mut seq = StepSequencer::new(16, sr, 120.0);
        seq.set_step(0, 0, true, 100);

        let partial = (sps / 2.0) as usize;
        for _ in 0..partial {
            seq.tick();
        }

        seq.reset();
        assert_eq!(seq.current_step(), 0);

        let frames = collect_trigger_frames(&mut seq, (sps + 2.0) as usize);
        assert!(!frames.is_empty());
        assert!((frames[0] as f64 - sps).abs() < 2.0);
    }

    #[test]
    fn reset_after_full_cycle_returns_to_step_0() {
        let sps = (44100.0_f64 * 60.0 / 120.0 / 4.0) as usize;

        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        seq.set_step(0, 0, true, 100);

        for _ in 0..(sps * 16 + 100) {
            seq.tick();
        }

        seq.reset();
        assert_eq!(seq.current_step(), 0);

        let frames = collect_trigger_frames(&mut seq, sps + 2);
        assert!(!frames.is_empty());
    }

    #[test]
    fn multiple_resets_are_idempotent() {
        let mut seq = StepSequencer::new(16, 44100.0, 120.0);
        for _ in 0..5000 {
            seq.tick();
        }
        seq.reset();
        seq.reset();
        seq.reset();
        assert_eq!(seq.current_step(), 0);
    }
}
