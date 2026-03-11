use serde::{Deserialize, Serialize};

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

pub struct StepSequencer {
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
}

impl StepSequencer {
    pub fn new(step_count: usize, sample_rate: f32, bpm: f64) -> Self {
        let sequences = (0..crate::drum_machine::NUM_PADS)
            .map(|i| PadSequence::new(i, step_count))
            .collect();

        let samples_per_step = sample_rate as f64 * 60.0 / bpm / 4.0;

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
}
