use serde::{Deserialize, Serialize};

/// A single MIDI note event on a track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEvent {
    /// Start position in frames.
    pub position: u64,
    /// Duration in frames.
    pub duration: u64,
    /// MIDI note number (0-127).
    pub note: u8,
    /// Velocity (0-127).
    pub velocity: u8,
    /// MIDI channel (0-15).
    pub channel: u8,
}

/// A MIDI control change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlChange {
    /// Position in frames.
    pub position: u64,
    /// CC number (0-127).
    pub controller: u8,
    /// CC value (0-127).
    pub value: u8,
    /// MIDI channel (0-15).
    pub channel: u8,
}

/// A MIDI clip containing note and CC events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiClip {
    /// Clip name.
    pub name: String,
    /// Note events in this clip.
    pub notes: Vec<NoteEvent>,
    /// Control change events.
    pub control_changes: Vec<ControlChange>,
    /// Position on the timeline in frames.
    pub timeline_pos: u64,
    /// Duration of the clip in frames.
    pub duration: u64,
}

impl MidiClip {
    pub fn new(name: impl Into<String>, timeline_pos: u64, duration: u64) -> Self {
        Self {
            name: name.into(),
            notes: Vec::new(),
            control_changes: Vec::new(),
            timeline_pos,
            duration,
        }
    }

    /// Add a note event, maintaining sorted order by position (frame).
    pub fn add_note(&mut self, position: u64, duration: u64, note: u8, velocity: u8, channel: u8) {
        let event = NoteEvent {
            position,
            duration,
            note,
            velocity,
            channel,
        };
        let idx = self
            .notes
            .binary_search_by_key(&position, |n| n.position)
            .unwrap_or_else(|i| i);
        self.notes.insert(idx, event);
    }

    /// Add a control change event, maintaining sorted order by position (frame).
    pub fn add_cc(&mut self, position: u64, controller: u8, value: u8, channel: u8) {
        let event = ControlChange {
            position,
            controller,
            value,
            channel,
        };
        let idx = self
            .control_changes
            .binary_search_by_key(&position, |cc| cc.position)
            .unwrap_or_else(|i| i);
        self.control_changes.insert(idx, event);
    }

    /// Get the end position on the timeline.
    pub fn end_pos(&self) -> u64 {
        self.timeline_pos + self.duration
    }

    /// Get notes active at a given frame position.
    pub fn notes_at(&self, frame: u64) -> Vec<&NoteEvent> {
        self.notes
            .iter()
            .filter(|n| {
                let abs_pos = self.timeline_pos + n.position;
                frame >= abs_pos && frame < abs_pos + n.duration
            })
            .collect()
    }

    /// Get note-on events at exactly the given frame.
    pub fn note_ons_at(&self, frame: u64) -> Vec<&NoteEvent> {
        self.notes
            .iter()
            .filter(|n| self.timeline_pos + n.position == frame)
            .collect()
    }

    /// Get note-off events at exactly the given frame.
    pub fn note_offs_at(&self, frame: u64) -> Vec<&NoteEvent> {
        self.notes
            .iter()
            .filter(|n| self.timeline_pos + n.position + n.duration == frame)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_clip_creation() {
        let mut clip = MidiClip::new("Chorus", 0, 48000);
        clip.add_note(0, 12000, 60, 100, 0);
        clip.add_note(12000, 12000, 64, 90, 0);
        clip.add_cc(0, 1, 64, 0);

        assert_eq!(clip.name, "Chorus");
        assert_eq!(clip.notes.len(), 2);
        assert_eq!(clip.control_changes.len(), 1);
        assert_eq!(clip.notes[0].note, 60);
        assert_eq!(clip.notes[0].velocity, 100);
        assert_eq!(clip.notes[1].note, 64);
        assert_eq!(clip.control_changes[0].controller, 1);
        assert_eq!(clip.control_changes[0].value, 64);
        assert_eq!(clip.end_pos(), 48000);
    }

    #[test]
    fn test_notes_at() {
        let mut clip = MidiClip::new("Test", 1000, 48000);
        // Note at relative position 0, duration 500 -> absolute 1000..1500
        clip.add_note(0, 500, 60, 100, 0);
        // Note at relative position 200, duration 300 -> absolute 1200..1500
        clip.add_note(200, 300, 64, 80, 0);

        // Frame 1000: only note 60 is active (starts at 1000)
        let active = clip.notes_at(1000);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].note, 60);

        // Frame 1200: both notes active
        let active = clip.notes_at(1200);
        assert_eq!(active.len(), 2);

        // Frame 1500: neither active (end is exclusive)
        let active = clip.notes_at(1500);
        assert_eq!(active.len(), 0);

        // Frame 999: nothing active yet
        let active = clip.notes_at(999);
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_note_ons_offs() {
        let mut clip = MidiClip::new("Test", 0, 48000);
        clip.add_note(100, 400, 60, 100, 0);
        clip.add_note(100, 200, 64, 80, 0);
        clip.add_note(500, 100, 67, 90, 0);

        // Note-ons at frame 100: notes 60 and 64
        let ons = clip.note_ons_at(100);
        assert_eq!(ons.len(), 2);

        // Note-ons at frame 500: note 67
        let ons = clip.note_ons_at(500);
        assert_eq!(ons.len(), 1);
        assert_eq!(ons[0].note, 67);

        // Note-offs at frame 300: note 64 (100 + 200)
        let offs = clip.note_offs_at(300);
        assert_eq!(offs.len(), 1);
        assert_eq!(offs[0].note, 64);

        // Note-offs at frame 500: note 60 (100 + 400)
        let offs = clip.note_offs_at(500);
        assert_eq!(offs.len(), 1);
        assert_eq!(offs[0].note, 60);

        // Note-offs at frame 600: note 67 (500 + 100)
        let offs = clip.note_offs_at(600);
        assert_eq!(offs.len(), 1);
        assert_eq!(offs[0].note, 67);
    }

    #[test]
    fn test_add_note_maintains_sorted_order() {
        let mut clip = MidiClip::new("Test", 0, 48000);
        // Add notes out of order
        clip.add_note(500, 100, 67, 90, 0);
        clip.add_note(100, 400, 60, 100, 0);
        clip.add_note(300, 200, 64, 80, 0);

        let positions: Vec<u64> = clip.notes.iter().map(|n| n.position).collect();
        assert_eq!(positions, vec![100, 300, 500]);
    }

    #[test]
    fn test_add_cc_maintains_sorted_order() {
        let mut clip = MidiClip::new("Test", 0, 48000);
        // Add CCs out of order
        clip.add_cc(400, 7, 100, 0);
        clip.add_cc(100, 1, 64, 0);
        clip.add_cc(200, 11, 80, 0);

        let positions: Vec<u64> = clip.control_changes.iter().map(|cc| cc.position).collect();
        assert_eq!(positions, vec![100, 200, 400]);
    }
}
