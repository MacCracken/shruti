use serde::{Deserialize, Serialize};

use crate::types::FramePos;

/// A single MIDI note event on a track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEvent {
    /// Start position in frames.
    pub position: FramePos,
    /// Duration in frames.
    pub duration: FramePos,
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
    pub position: FramePos,
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
    pub timeline_pos: FramePos,
    /// Duration of the clip in frames.
    pub duration: FramePos,
}

impl MidiClip {
    pub fn new(
        name: impl Into<String>,
        timeline_pos: impl Into<FramePos>,
        duration: impl Into<FramePos>,
    ) -> Self {
        Self {
            name: name.into(),
            notes: Vec::new(),
            control_changes: Vec::new(),
            timeline_pos: timeline_pos.into(),
            duration: duration.into(),
        }
    }

    /// Add a note event, maintaining sorted order by position (frame).
    pub fn add_note(
        &mut self,
        position: impl Into<FramePos>,
        duration: impl Into<FramePos>,
        note: u8,
        velocity: u8,
        channel: u8,
    ) {
        let position = position.into();
        let duration = duration.into();
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
    pub fn add_cc(
        &mut self,
        position: impl Into<FramePos>,
        controller: u8,
        value: u8,
        channel: u8,
    ) {
        let position = position.into();
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
    pub fn end_pos(&self) -> FramePos {
        self.timeline_pos + self.duration
    }

    /// Get notes active at a given frame position.
    pub fn notes_at(&self, frame: FramePos) -> Vec<&NoteEvent> {
        self.notes
            .iter()
            .filter(|n| {
                let abs_pos = self.timeline_pos + n.position;
                frame >= abs_pos && frame < abs_pos + n.duration
            })
            .collect()
    }

    /// Get note-on events at exactly the given frame.
    pub fn note_ons_at(&self, frame: FramePos) -> Vec<&NoteEvent> {
        self.notes
            .iter()
            .filter(|n| self.timeline_pos + n.position == frame)
            .collect()
    }

    /// Get note-off events at exactly the given frame.
    pub fn note_offs_at(&self, frame: FramePos) -> Vec<&NoteEvent> {
        self.notes
            .iter()
            .filter(|n| self.timeline_pos + n.position + n.duration == frame)
            .collect()
    }
}

// ── MIDI 2.0 Universal MIDI Packet (UMP) ──────────────────────────────────

/// MIDI 2.0 message type (4-bit field in UMP header).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UmpMessageType {
    /// Utility messages (JR Timestamp, JR Clock).
    Utility,
    /// System Real-Time / System Common (MIDI 1.0 compatible).
    SystemCommon,
    /// MIDI 1.0 Channel Voice (note on/off, CC, etc. — 7-bit).
    Midi1ChannelVoice,
    /// Data messages (SysEx 7-bit).
    Data64,
    /// MIDI 2.0 Channel Voice (note on/off, CC, etc. — 32-bit).
    Midi2ChannelVoice,
    /// Data messages (SysEx 8-bit).
    Data128,
}

/// A MIDI 2.0 Note-On event with 16-bit velocity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteOnV2 {
    pub position: FramePos,
    pub note: u8,
    /// 16-bit velocity (0–65535). 0 = note-off.
    pub velocity: u16,
    pub channel: u8,
    /// Per-note attribute type (0 = none, 1 = manufacturer, 2 = profile, 3 = pitch 7.9).
    pub attribute_type: u8,
    /// Per-note attribute data (16-bit).
    pub attribute_data: u16,
}

/// A MIDI 2.0 Note-Off event with 16-bit velocity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteOffV2 {
    pub position: FramePos,
    pub note: u8,
    /// 16-bit velocity (0–65535).
    pub velocity: u16,
    pub channel: u8,
    pub attribute_type: u8,
    pub attribute_data: u16,
}

/// A MIDI 2.0 Control Change with 32-bit value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlChangeV2 {
    pub position: FramePos,
    /// CC index (0–255 in MIDI 2.0, maps 0-127 to MIDI 1.0 CCs).
    pub controller: u8,
    /// 32-bit value (full resolution).
    pub value: u32,
    pub channel: u8,
}

/// Per-note pitch bend (MIDI 2.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerNotePitchBend {
    pub position: FramePos,
    pub note: u8,
    pub channel: u8,
    /// 32-bit pitch bend value. 0x80000000 = no bend (center).
    pub value: u32,
}

/// Per-note controller (MIDI 2.0 — per-note CC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerNoteController {
    pub position: FramePos,
    pub note: u8,
    pub channel: u8,
    pub controller: u8,
    /// 32-bit value.
    pub value: u32,
}

/// Channel pressure / aftertouch with 32-bit resolution (MIDI 2.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPressureV2 {
    pub position: FramePos,
    pub channel: u8,
    /// 32-bit pressure value.
    pub value: u32,
}

/// Polyphonic key pressure / aftertouch with 32-bit resolution (MIDI 2.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyPressureV2 {
    pub position: FramePos,
    pub note: u8,
    pub channel: u8,
    /// 32-bit pressure value.
    pub value: u32,
}

/// Channel pitch bend with 32-bit resolution (MIDI 2.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchBendV2 {
    pub position: FramePos,
    pub channel: u8,
    /// 32-bit pitch bend. 0x80000000 = center (no bend).
    pub value: u32,
}

/// MIDI 1.0 ↔ 2.0 translation utilities.
pub mod translate {
    use super::*;

    /// Scale a 7-bit MIDI 1.0 velocity to 16-bit MIDI 2.0 velocity.
    pub fn velocity_7_to_16(v: u8) -> u16 {
        if v == 0 {
            return 0;
        }
        // MIDI 2.0 spec: scale by 512 + shift
        (v as u16) << 9 | (v as u16) << 2 | (v as u16) >> 5
    }

    /// Scale a 16-bit MIDI 2.0 velocity to 7-bit MIDI 1.0 velocity.
    pub fn velocity_16_to_7(v: u16) -> u8 {
        (v >> 9) as u8
    }

    /// Scale a 7-bit CC value to 32-bit.
    pub fn cc_7_to_32(v: u8) -> u32 {
        let v32 = v as u32;
        v32 << 25 | v32 << 18 | v32 << 11 | v32 << 4 | v32 >> 3
    }

    /// Scale a 32-bit CC value to 7-bit.
    pub fn cc_32_to_7(v: u32) -> u8 {
        (v >> 25) as u8
    }

    /// Convert a MIDI 1.0 NoteEvent to MIDI 2.0 NoteOnV2.
    pub fn note_event_to_v2(event: &NoteEvent) -> NoteOnV2 {
        NoteOnV2 {
            position: event.position,
            note: event.note,
            velocity: velocity_7_to_16(event.velocity),
            channel: event.channel,
            attribute_type: 0,
            attribute_data: 0,
        }
    }

    /// Convert a MIDI 2.0 NoteOnV2 to MIDI 1.0 NoteEvent (lossy).
    pub fn note_on_v2_to_event(event: &NoteOnV2, duration: FramePos) -> NoteEvent {
        NoteEvent {
            position: event.position,
            duration,
            note: event.note,
            velocity: velocity_16_to_7(event.velocity),
            channel: event.channel,
        }
    }

    /// Convert a MIDI 1.0 ControlChange to MIDI 2.0 ControlChangeV2.
    pub fn cc_to_v2(cc: &ControlChange) -> ControlChangeV2 {
        ControlChangeV2 {
            position: cc.position,
            controller: cc.controller,
            value: cc_7_to_32(cc.value),
            channel: cc.channel,
        }
    }

    /// Convert a MIDI 2.0 ControlChangeV2 to MIDI 1.0 ControlChange (lossy).
    pub fn cc_v2_to_cc(cc: &ControlChangeV2) -> ControlChange {
        ControlChange {
            position: cc.position,
            controller: cc.controller,
            value: cc_32_to_7(cc.value),
            channel: cc.channel,
        }
    }

    /// 32-bit pitch bend center value (no bend).
    pub const PITCH_BEND_CENTER: u32 = 0x80000000;

    /// Convert a 14-bit MIDI 1.0 pitch bend (0-16383, center=8192) to 32-bit.
    pub fn pitch_bend_14_to_32(v: u16) -> u32 {
        (v as u32) << 18 | (v as u32) << 4 | (v as u32) >> 10
    }

    /// Convert a 32-bit MIDI 2.0 pitch bend to 14-bit.
    pub fn pitch_bend_32_to_14(v: u32) -> u16 {
        (v >> 18) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_clip_creation() {
        let mut clip = MidiClip::new("Chorus", 0u64, 48000u64);
        clip.add_note(0u64, 12000u64, 60, 100, 0);
        clip.add_note(12000u64, 12000u64, 64, 90, 0);
        clip.add_cc(0u64, 1, 64, 0);

        assert_eq!(clip.name, "Chorus");
        assert_eq!(clip.notes.len(), 2);
        assert_eq!(clip.control_changes.len(), 1);
        assert_eq!(clip.notes[0].note, 60);
        assert_eq!(clip.notes[0].velocity, 100);
        assert_eq!(clip.notes[1].note, 64);
        assert_eq!(clip.control_changes[0].controller, 1);
        assert_eq!(clip.control_changes[0].value, 64);
        assert_eq!(clip.end_pos(), FramePos(48000));
    }

    #[test]
    fn test_notes_at() {
        let mut clip = MidiClip::new("Test", 1000u64, 48000u64);
        // Note at relative position 0, duration 500 -> absolute 1000..1500
        clip.add_note(0u64, 500u64, 60, 100, 0);
        // Note at relative position 200, duration 300 -> absolute 1200..1500
        clip.add_note(200u64, 300u64, 64, 80, 0);

        // Frame 1000: only note 60 is active (starts at 1000)
        let active = clip.notes_at(FramePos(1000));
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].note, 60);

        // Frame 1200: both notes active
        let active = clip.notes_at(FramePos(1200));
        assert_eq!(active.len(), 2);

        // Frame 1500: neither active (end is exclusive)
        let active = clip.notes_at(FramePos(1500));
        assert_eq!(active.len(), 0);

        // Frame 999: nothing active yet
        let active = clip.notes_at(FramePos(999));
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_note_ons_offs() {
        let mut clip = MidiClip::new("Test", 0u64, 48000u64);
        clip.add_note(100u64, 400u64, 60, 100, 0);
        clip.add_note(100u64, 200u64, 64, 80, 0);
        clip.add_note(500u64, 100u64, 67, 90, 0);

        // Note-ons at frame 100: notes 60 and 64
        let ons = clip.note_ons_at(FramePos(100));
        assert_eq!(ons.len(), 2);

        // Note-ons at frame 500: note 67
        let ons = clip.note_ons_at(FramePos(500));
        assert_eq!(ons.len(), 1);
        assert_eq!(ons[0].note, 67);

        // Note-offs at frame 300: note 64 (100 + 200)
        let offs = clip.note_offs_at(FramePos(300));
        assert_eq!(offs.len(), 1);
        assert_eq!(offs[0].note, 64);

        // Note-offs at frame 500: note 60 (100 + 400)
        let offs = clip.note_offs_at(FramePos(500));
        assert_eq!(offs.len(), 1);
        assert_eq!(offs[0].note, 60);

        // Note-offs at frame 600: note 67 (500 + 100)
        let offs = clip.note_offs_at(FramePos(600));
        assert_eq!(offs.len(), 1);
        assert_eq!(offs[0].note, 67);
    }

    #[test]
    fn test_add_note_maintains_sorted_order() {
        let mut clip = MidiClip::new("Test", 0u64, 48000u64);
        // Add notes out of order
        clip.add_note(500u64, 100u64, 67, 90, 0);
        clip.add_note(100u64, 400u64, 60, 100, 0);
        clip.add_note(300u64, 200u64, 64, 80, 0);

        let positions: Vec<FramePos> = clip.notes.iter().map(|n| n.position).collect();
        assert_eq!(positions, vec![FramePos(100), FramePos(300), FramePos(500)]);
    }

    #[test]
    fn test_add_cc_maintains_sorted_order() {
        let mut clip = MidiClip::new("Test", 0u64, 48000u64);
        // Add CCs out of order
        clip.add_cc(400u64, 7, 100, 0);
        clip.add_cc(100u64, 1, 64, 0);
        clip.add_cc(200u64, 11, 80, 0);

        let positions: Vec<FramePos> = clip.control_changes.iter().map(|cc| cc.position).collect();
        assert_eq!(positions, vec![FramePos(100), FramePos(200), FramePos(400)]);
    }

    // --- MIDI 2.0 translation tests ---

    #[test]
    fn velocity_7_to_16_zero() {
        assert_eq!(translate::velocity_7_to_16(0), 0);
    }

    #[test]
    fn velocity_7_to_16_max() {
        let v16 = translate::velocity_7_to_16(127);
        assert!(v16 > 65000); // should be near max
    }

    #[test]
    fn velocity_roundtrip() {
        for v in 1..=127u8 {
            let v16 = translate::velocity_7_to_16(v);
            let back = translate::velocity_16_to_7(v16);
            assert_eq!(back, v, "roundtrip failed for velocity {v}");
        }
    }

    #[test]
    fn cc_roundtrip() {
        for v in 0..=127u8 {
            let v32 = translate::cc_7_to_32(v);
            let back = translate::cc_32_to_7(v32);
            assert_eq!(back, v, "roundtrip failed for CC {v}");
        }
    }

    #[test]
    fn note_event_to_v2_preserves_fields() {
        let event = NoteEvent {
            position: FramePos(1000),
            duration: FramePos(500),
            note: 60,
            velocity: 100,
            channel: 0,
        };
        let v2 = translate::note_event_to_v2(&event);
        assert_eq!(v2.note, 60);
        assert_eq!(v2.channel, 0);
        assert_eq!(v2.position, FramePos(1000));
        assert!(v2.velocity > 0);
    }

    #[test]
    fn note_v2_roundtrip() {
        let event = NoteEvent {
            position: FramePos(0),
            duration: FramePos(100),
            note: 64,
            velocity: 80,
            channel: 3,
        };
        let v2 = translate::note_event_to_v2(&event);
        let back = translate::note_on_v2_to_event(&v2, FramePos(100));
        assert_eq!(back.note, 64);
        assert_eq!(back.velocity, 80);
        assert_eq!(back.channel, 3);
    }

    #[test]
    fn cc_to_v2_preserves_fields() {
        let cc = ControlChange {
            position: FramePos(500),
            controller: 7,
            value: 100,
            channel: 0,
        };
        let v2 = translate::cc_to_v2(&cc);
        assert_eq!(v2.controller, 7);
        assert_eq!(v2.channel, 0);
        let back = translate::cc_v2_to_cc(&v2);
        assert_eq!(back.value, 100);
    }

    #[test]
    fn pitch_bend_roundtrip() {
        let center = 8192u16;
        let v32 = translate::pitch_bend_14_to_32(center);
        let back = translate::pitch_bend_32_to_14(v32);
        assert_eq!(back, center);
    }

    #[test]
    fn pitch_bend_center_constant() {
        assert_eq!(translate::PITCH_BEND_CENTER, 0x80000000);
    }

    #[test]
    fn note_on_v2_serializes() {
        let n = NoteOnV2 {
            position: FramePos(100),
            note: 60,
            velocity: 32768,
            channel: 0,
            attribute_type: 0,
            attribute_data: 0,
        };
        let json = serde_json::to_string(&n).unwrap();
        let rt: NoteOnV2 = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.note, 60);
        assert_eq!(rt.velocity, 32768);
    }

    #[test]
    fn control_change_v2_serializes() {
        let cc = ControlChangeV2 {
            position: FramePos(0),
            controller: 74,
            value: 2_000_000_000,
            channel: 5,
        };
        let json = serde_json::to_string(&cc).unwrap();
        let rt: ControlChangeV2 = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.controller, 74);
        assert_eq!(rt.value, 2_000_000_000);
    }
}
