use serde::{Deserialize, Serialize};

/// Voice state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceState {
    /// Voice is not producing sound.
    Idle,
    /// Voice is actively playing.
    Active,
    /// Voice is in its release phase.
    Releasing,
}

/// How to steal voices when max polyphony is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceStealMode {
    /// Steal the oldest voice.
    Oldest,
    /// Steal the quietest voice.
    Quietest,
    /// Steal the lowest-pitched voice.
    Lowest,
    /// Don't steal — ignore new notes.
    None,
}

/// A single voice slot.
#[derive(Debug, Clone)]
pub struct Voice {
    pub state: VoiceState,
    pub note: u8,
    pub velocity: u8,
    pub channel: u8,
    /// Phase accumulator (for oscillators).
    pub phase: f64,
    /// Per-voice amplitude envelope level.
    pub envelope_level: f32,
    /// Age counter — increments each process block while active.
    pub age: u64,
}

impl Voice {
    pub fn new() -> Self {
        Self {
            state: VoiceState::Idle,
            note: 0,
            velocity: 0,
            channel: 0,
            phase: 0.0,
            envelope_level: 0.0,
            age: 0,
        }
    }

    pub fn is_idle(&self) -> bool {
        self.state == VoiceState::Idle
    }

    pub fn is_active(&self) -> bool {
        self.state == VoiceState::Active || self.state == VoiceState::Releasing
    }

    /// Frequency in Hz for this voice's MIDI note.
    pub fn frequency(&self) -> f64 {
        440.0 * 2.0f64.powf((self.note as f64 - 69.0) / 12.0)
    }
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages a pool of voices with configurable polyphony and voice stealing.
pub struct VoiceManager {
    pub voices: Vec<Voice>,
    pub max_voices: usize,
    pub steal_mode: VoiceStealMode,
}

impl VoiceManager {
    pub fn new(max_voices: usize, steal_mode: VoiceStealMode) -> Self {
        let voices = (0..max_voices).map(|_| Voice::new()).collect();
        Self {
            voices,
            max_voices,
            steal_mode,
        }
    }

    /// Allocate a voice for a note-on. Returns the voice index, or None if no voice available.
    pub fn note_on(&mut self, note: u8, velocity: u8, channel: u8) -> Option<usize> {
        // First: look for an idle voice
        if let Some(idx) = self.voices.iter().position(|v| v.is_idle()) {
            self.activate_voice(idx, note, velocity, channel);
            return Some(idx);
        }

        // Second: try voice stealing
        let steal_idx = match self.steal_mode {
            VoiceStealMode::Oldest => self
                .voices
                .iter()
                .enumerate()
                .max_by_key(|(_, v)| v.age)
                .map(|(i, _)| i),
            VoiceStealMode::Quietest => self
                .voices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.envelope_level
                        .partial_cmp(&b.envelope_level)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i),
            VoiceStealMode::Lowest => self
                .voices
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| v.note)
                .map(|(i, _)| i),
            VoiceStealMode::None => None,
        };

        if let Some(idx) = steal_idx {
            self.activate_voice(idx, note, velocity, channel);
            Some(idx)
        } else {
            None
        }
    }

    /// Release voice(s) playing a specific note.
    pub fn note_off(&mut self, note: u8, channel: u8) {
        for voice in &mut self.voices {
            if voice.note == note && voice.channel == channel && voice.state == VoiceState::Active {
                voice.state = VoiceState::Releasing;
            }
        }
    }

    /// Mark a voice as idle (called when envelope finishes release).
    pub fn free_voice(&mut self, index: usize) {
        if index < self.voices.len() {
            self.voices[index].state = VoiceState::Idle;
            self.voices[index].age = 0;
        }
    }

    /// Number of active (non-idle) voices.
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| !v.is_idle()).count()
    }

    /// Increment age of all active voices. Call once per process block.
    pub fn tick_age(&mut self) {
        for voice in &mut self.voices {
            if voice.is_active() {
                voice.age += 1;
            }
        }
    }

    /// Reset all voices to idle.
    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            *voice = Voice::new();
        }
    }

    fn activate_voice(&mut self, idx: usize, note: u8, velocity: u8, channel: u8) {
        let voice = &mut self.voices[idx];
        voice.state = VoiceState::Active;
        voice.note = note;
        voice.velocity = velocity;
        voice.channel = channel;
        voice.phase = 0.0;
        voice.envelope_level = 1.0;
        voice.age = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_frequency_a4() {
        let mut v = Voice::new();
        v.note = 69;
        assert!((v.frequency() - 440.0).abs() < 0.01);
    }

    #[test]
    fn voice_frequency_middle_c() {
        let mut v = Voice::new();
        v.note = 60;
        assert!((v.frequency() - 261.63).abs() < 0.1);
    }

    #[test]
    fn voice_manager_allocates_voices() {
        let mut vm = VoiceManager::new(4, VoiceStealMode::Oldest);
        assert_eq!(vm.active_count(), 0);
        vm.note_on(60, 100, 0);
        assert_eq!(vm.active_count(), 1);
        vm.note_on(64, 100, 0);
        assert_eq!(vm.active_count(), 2);
    }

    #[test]
    fn voice_manager_note_off_releases() {
        let mut vm = VoiceManager::new(4, VoiceStealMode::Oldest);
        vm.note_on(60, 100, 0);
        vm.note_off(60, 0);
        // Voice should be Releasing, not Idle
        assert_eq!(vm.voices[0].state, VoiceState::Releasing);
        assert_eq!(vm.active_count(), 1); // Still active (releasing)
    }

    #[test]
    fn voice_manager_free_voice() {
        let mut vm = VoiceManager::new(4, VoiceStealMode::Oldest);
        vm.note_on(60, 100, 0);
        vm.free_voice(0);
        assert_eq!(vm.active_count(), 0);
    }

    #[test]
    fn voice_manager_steal_oldest() {
        let mut vm = VoiceManager::new(2, VoiceStealMode::Oldest);
        vm.note_on(60, 100, 0);
        vm.tick_age(); // voice 0 age = 1
        vm.note_on(64, 100, 0);
        vm.tick_age(); // voice 0 age = 2, voice 1 age = 1
        // All voices full, steal oldest (voice 0)
        let idx = vm.note_on(67, 100, 0);
        assert_eq!(idx, Some(0));
        assert_eq!(vm.voices[0].note, 67);
    }

    #[test]
    fn voice_manager_steal_none_rejects() {
        let mut vm = VoiceManager::new(2, VoiceStealMode::None);
        vm.note_on(60, 100, 0);
        vm.note_on(64, 100, 0);
        let idx = vm.note_on(67, 100, 0);
        assert_eq!(idx, None);
    }

    #[test]
    fn voice_manager_reset() {
        let mut vm = VoiceManager::new(4, VoiceStealMode::Oldest);
        vm.note_on(60, 100, 0);
        vm.note_on(64, 100, 0);
        vm.reset();
        assert_eq!(vm.active_count(), 0);
    }

    #[test]
    fn voice_manager_steal_lowest() {
        let mut vm = VoiceManager::new(2, VoiceStealMode::Lowest);
        vm.note_on(60, 100, 0); // C4
        vm.note_on(72, 100, 0); // C5
        // Steal lowest (note 60)
        let idx = vm.note_on(64, 100, 0);
        assert_eq!(idx, Some(0));
        assert_eq!(vm.voices[0].note, 64);
    }

    // --- Polyphony stress tests ---

    #[test]
    fn max_voices_allocation() {
        let max = 16;
        let mut vm = VoiceManager::new(max, VoiceStealMode::Oldest);
        for i in 0..max {
            let idx = vm.note_on(40 + i as u8, 100, 0);
            assert!(idx.is_some(), "voice {i} should be allocated");
        }
        assert_eq!(vm.active_count(), max);
        // All voice slots should be active
        for voice in &vm.voices {
            assert!(voice.is_active());
        }
    }

    #[test]
    fn voice_stealing_oldest_full() {
        let max = 16;
        let mut vm = VoiceManager::new(max, VoiceStealMode::Oldest);
        // Fill all voices, ticking age between each to establish ordering
        for i in 0..max {
            vm.note_on(40 + i as u8, 100, 0);
            vm.tick_age();
        }
        // Voice 0 is the oldest (age = 16), voice 15 is newest (age = 1)
        let oldest_note = vm.voices[0].note;
        assert_eq!(oldest_note, 40);
        // Allocate one more — should steal voice 0 (oldest)
        let idx = vm.note_on(90, 127, 0);
        assert_eq!(idx, Some(0));
        assert_eq!(vm.voices[0].note, 90);
        assert_eq!(vm.active_count(), max);
    }

    #[test]
    fn voice_stealing_quietest() {
        let max = 4;
        let mut vm = VoiceManager::new(max, VoiceStealMode::Quietest);
        // Fill all voices
        for i in 0..max {
            vm.note_on(60 + i as u8, 100, 0);
        }
        // Manually set envelope levels so voice 2 is quietest
        vm.voices[0].envelope_level = 0.8;
        vm.voices[1].envelope_level = 0.6;
        vm.voices[2].envelope_level = 0.1; // quietest
        vm.voices[3].envelope_level = 0.5;
        // Allocate one more — should steal voice 2 (quietest)
        let idx = vm.note_on(80, 127, 0);
        assert_eq!(idx, Some(2));
        assert_eq!(vm.voices[2].note, 80);
        assert_eq!(vm.active_count(), max);
    }

    #[test]
    fn all_voices_active_stress() {
        let max = 16;
        let mut vm = VoiceManager::new(max, VoiceStealMode::Oldest);
        // Trigger all max voices
        for i in 0..max {
            vm.note_on(40 + i as u8, 100, 0);
        }
        // Process multiple tick_age cycles (simulating audio blocks)
        for _ in 0..100 {
            vm.tick_age();
        }
        assert_eq!(vm.active_count(), max);
        // Verify all voice state is consistent: finite values, no NaN/inf
        for voice in &vm.voices {
            assert!(voice.is_active());
            assert!(voice.phase.is_finite());
            assert!(voice.envelope_level.is_finite());
            assert!(!voice.envelope_level.is_nan());
            assert!(voice.frequency().is_finite());
            assert!(voice.frequency() > 0.0);
        }
    }

    #[test]
    fn rapid_note_on_off() {
        let max = 8;
        let mut vm = VoiceManager::new(max, VoiceStealMode::Oldest);
        // Rapidly trigger and release many notes
        for cycle in 0..200 {
            let note = 30 + (cycle % 88) as u8;
            vm.note_on(note, 100, 0);
            vm.tick_age();
            // Release every other note immediately
            if cycle % 2 == 0 {
                vm.note_off(note, 0);
            }
            // Free releasing voices periodically
            if cycle % 5 == 0 {
                for i in 0..max {
                    if vm.voices[i].state == VoiceState::Releasing {
                        vm.free_voice(i);
                    }
                }
            }
            assert!(
                vm.active_count() <= max,
                "voice count {} exceeded max {} at cycle {}",
                vm.active_count(),
                max,
                cycle
            );
        }
    }

    #[test]
    fn voice_reuse_after_release() {
        let max = 2;
        let mut vm = VoiceManager::new(max, VoiceStealMode::None);
        // Allocate both slots
        let idx0 = vm.note_on(60, 100, 0);
        let idx1 = vm.note_on(64, 100, 0);
        assert_eq!(idx0, Some(0));
        assert_eq!(idx1, Some(1));
        // With steal_mode=None, a third note should be rejected
        assert_eq!(vm.note_on(67, 100, 0), None);
        // Release and free voice 0 (simulating envelope finish)
        vm.note_off(60, 0);
        assert_eq!(vm.voices[0].state, VoiceState::Releasing);
        vm.free_voice(0);
        assert!(vm.voices[0].is_idle());
        // Now the slot should be reusable
        let idx_reused = vm.note_on(67, 100, 0);
        assert_eq!(idx_reused, Some(0));
        assert_eq!(vm.voices[0].note, 67);
        assert_eq!(vm.active_count(), 2);
    }
}
