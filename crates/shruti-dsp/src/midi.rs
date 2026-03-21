//! MIDI types — re-exported from dhvani.

pub use dhvani::midi::voice::{Voice, VoiceManager, VoiceState, VoiceStealMode};
pub use dhvani::midi::{ControlChange, FramePos, MidiClip, MidiEvent, NoteEvent};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_event_construction() {
        let note = NoteEvent {
            position: 0,
            duration: 48000,
            note: 60,
            velocity: 100,
            channel: 0,
        };
        assert_eq!(note.note, 60);
        assert_eq!(note.velocity, 100);
    }

    #[test]
    fn midi_clip_basic() {
        let mut clip = MidiClip::new("test", 0, 96000);
        clip.add_note(0, 48000, 60, 100, 0);
        clip.add_note(48000, 48000, 64, 90, 0);
        assert_eq!(clip.event_count(), 2);
    }

    #[test]
    fn voice_manager_note_on() {
        let mut vm = VoiceManager::new(8, VoiceStealMode::Oldest);
        let slot = vm.note_on(60, 100, 0);
        assert!(slot.is_some());
        assert_eq!(vm.active_count(), 1);
    }

    #[test]
    fn voice_manager_note_off() {
        let mut vm = VoiceManager::new(8, VoiceStealMode::Oldest);
        vm.note_on(60, 100, 0);
        vm.note_off(60, 0);
        // Voice enters releasing state (not immediately idle)
        let voice = vm.voice(0).unwrap();
        assert_eq!(voice.state, VoiceState::Releasing);
    }

    #[test]
    fn voice_frequency() {
        let mut vm = VoiceManager::new(4, VoiceStealMode::Oldest);
        let slot = vm.note_on(69, 100, 0).unwrap(); // A4
        let voice = vm.voice(slot).unwrap();
        assert!((voice.frequency() - 440.0).abs() < 0.1);
    }
}
