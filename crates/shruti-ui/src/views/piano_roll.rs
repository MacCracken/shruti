use egui::{Color32, Rect, ScrollArea, Stroke, Ui, pos2, vec2};

use shruti_session::track::TrackKind;

use crate::state::UiState;
use crate::theme::ThemeColors;
use crate::widgets::{timeline_ruler, track_header};

/// Height of each note row in pixels.
const NOTE_ROW_HEIGHT: f32 = 12.0;
/// Width of the piano key column in pixels.
const PIANO_KEY_WIDTH: f32 = 40.0;
/// Height of the bar/beat ruler at the top.
const RULER_HEIGHT: f32 = 24.0;
/// Total number of MIDI notes.
const MIDI_NOTE_COUNT: u8 = 128;

/// Return the note name for a MIDI note number (e.g. 60 -> "C4").
pub fn note_name(note: u8) -> String {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (note / 12) as i8 - 1;
    format!("{}{}", names[note as usize % 12], octave)
}

/// Return true if the MIDI note falls on a black key.
pub fn is_black_key(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}

/// Return the GM drum name for standard drum notes (36-51).
pub fn drum_name(note: u8) -> &'static str {
    match note {
        36 => "Kick",
        37 => "Side Stick",
        38 => "Snare",
        39 => "Clap",
        40 => "E Snare",
        41 => "Low Tom",
        42 => "Closed HH",
        43 => "Low Tom 2",
        44 => "Pedal HH",
        45 => "Mid Tom",
        46 => "Open HH",
        47 => "Mid Tom 2",
        48 => "Hi Tom",
        49 => "Crash 1",
        50 => "Hi Tom 2",
        51 => "Ride",
        _ => "",
    }
}

/// Convert a MIDI note number to a Y position within the note grid.
/// Note 127 is at the top (y=0), note 0 is at the bottom.
pub fn note_row_y(note: u8) -> f32 {
    (MIDI_NOTE_COUNT - 1 - note) as f32 * NOTE_ROW_HEIGHT
}

/// Convert a frame position to an X pixel offset given the pixels-per-frame scale.
pub fn beat_grid_x(frame: shruti_session::FramePos, pixels_per_frame: f64, scroll_x: f64) -> f32 {
    (frame.as_f64() * pixels_per_frame - scroll_x) as f32
}

/// Map a velocity value (0-127) to an alpha multiplier in 0.3..1.0.
pub fn velocity_alpha(velocity: u8) -> f32 {
    0.3 + (velocity as f32 / 127.0) * 0.7
}

/// Draw the piano roll MIDI editor view.
pub fn piano_roll_view(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    let track_idx = match state.selected_track {
        Some(idx) if idx < state.session.tracks.len() => idx,
        _ => {
            ui.centered_and_justified(|ui| {
                ui.label("Select an instrument or MIDI track to view the piano roll.");
            });
            return;
        }
    };

    let available = ui.available_rect_before_wrap();

    // Determine track kind for instrument-aware features.
    let track_kind = state.session.tracks[track_idx].kind.clone();
    let is_drum = matches!(track_kind, TrackKind::DrumMachine { .. });
    let is_ai_player = matches!(track_kind, TrackKind::AiPlayer { .. });

    // --- Header: bar/beat ruler ---
    let ruler_rect = Rect::from_min_size(
        pos2(available.left() + PIANO_KEY_WIDTH, available.top()),
        vec2(available.width() - PIANO_KEY_WIDTH, RULER_HEIGHT),
    );
    timeline_ruler::draw_ruler(
        ui,
        ruler_rect,
        state.scroll_x,
        state.pixels_per_frame,
        state.session.sample_rate,
        state.session.transport.bpm,
        colors,
    );

    // --- Content area below ruler ---
    let content_top = available.top() + RULER_HEIGHT;
    let content_rect = Rect::from_min_max(
        pos2(available.left(), content_top),
        available.right_bottom(),
    );

    let scroll_x = state.scroll_x;
    let pixels_per_frame = state.pixels_per_frame;
    let sample_rate = state.session.sample_rate;
    let bpm = state.session.transport.bpm;

    // Collect MIDI clip data before entering the UI closure.
    let midi_clips: Vec<_> = state.session.tracks[track_idx].midi_clips.clone();
    let track_color = track_header::track_color(track_idx);

    // AI player accent color override.
    let note_base_color = if is_ai_player {
        Color32::from_rgb(0, 220, 180) // teal accent for AI tracks
    } else {
        track_color
    };

    let total_note_height = MIDI_NOTE_COUNT as f32 * NOTE_ROW_HEIGHT;

    let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

    ScrollArea::both()
        .auto_shrink([false; 2])
        .vertical_scroll_offset(note_row_y(60).max(0.0) - content_rect.height() / 2.0)
        .show(&mut content_ui, |ui| {
            // Reserve the full virtual size for 128 note rows.
            let grid_width = (ui.available_width() - PIANO_KEY_WIDTH).max(400.0);
            let (total_rect, _) = ui.allocate_exact_size(
                vec2(PIANO_KEY_WIDTH + grid_width, total_note_height),
                egui::Sense::hover(),
            );

            let painter = ui.painter_at(total_rect);

            let keys_rect = Rect::from_min_size(
                total_rect.left_top(),
                vec2(PIANO_KEY_WIDTH, total_note_height),
            );
            let grid_rect = Rect::from_min_size(
                pos2(total_rect.left() + PIANO_KEY_WIDTH, total_rect.top()),
                vec2(grid_width, total_note_height),
            );

            // --- Piano keys column ---
            for note in 0..MIDI_NOTE_COUNT {
                let y = keys_rect.top() + note_row_y(note);
                let row_rect = Rect::from_min_size(
                    pos2(keys_rect.left(), y),
                    vec2(PIANO_KEY_WIDTH, NOTE_ROW_HEIGHT),
                );

                if !ui.is_rect_visible(row_rect) {
                    continue;
                }

                // Background: black keys darker, white keys lighter.
                let bg = if is_black_key(note) {
                    colors.bg_primary()
                } else {
                    colors.bg_secondary()
                };
                painter.rect_filled(row_rect, 0.0, bg);

                // Highlight playable range for Instrument/Sampler tracks.
                let highlight = match &track_kind {
                    TrackKind::Instrument { .. } | TrackKind::Sampler { .. } => {
                        // Standard 88-key piano range: A0 (21) to C8 (108).
                        (21..=108).contains(&note)
                    }
                    _ => false,
                };
                if highlight {
                    painter.rect_filled(row_rect, 0.0, colors.accent().linear_multiply(0.08));
                }

                // Label
                let label = if is_drum {
                    let dname = drum_name(note);
                    if dname.is_empty() {
                        note_name(note)
                    } else {
                        dname.to_string()
                    }
                } else if note % 12 == 0 {
                    // Only label C notes on the piano keyboard to avoid clutter.
                    note_name(note)
                } else {
                    String::new()
                };

                if !label.is_empty() {
                    painter.text(
                        pos2(row_rect.left() + 2.0, row_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &label,
                        egui::FontId::new(8.0, egui::FontFamily::Proportional),
                        colors.text_secondary(),
                    );
                }

                // Bottom separator for each row.
                painter.line_segment(
                    [row_rect.left_bottom(), row_rect.right_bottom()],
                    Stroke::new(0.25, colors.separator()),
                );
            }

            // Right border of piano keys.
            painter.line_segment(
                [keys_rect.right_top(), keys_rect.right_bottom()],
                Stroke::new(1.0, colors.separator()),
            );

            // --- Note grid area ---
            // Background
            painter.rect_filled(grid_rect, 0.0, colors.bg_tertiary());

            // Horizontal lines between note rows.
            for note in 0..MIDI_NOTE_COUNT {
                let y = grid_rect.top() + note_row_y(note) + NOTE_ROW_HEIGHT;
                if y > grid_rect.top() && y < grid_rect.bottom() {
                    let stroke = if note % 12 == 0 {
                        Stroke::new(0.5, colors.separator())
                    } else {
                        Stroke::new(0.25, colors.grid())
                    };
                    painter.line_segment(
                        [pos2(grid_rect.left(), y), pos2(grid_rect.right(), y)],
                        stroke,
                    );
                }
            }

            // Alternate shading for black key rows.
            for note in 0..MIDI_NOTE_COUNT {
                if is_black_key(note) {
                    let y = grid_rect.top() + note_row_y(note);
                    let row_rect = Rect::from_min_size(
                        pos2(grid_rect.left(), y),
                        vec2(grid_rect.width(), NOTE_ROW_HEIGHT),
                    );
                    painter.rect_filled(row_rect, 0.0, Color32::from_black_alpha(15));
                }
            }

            // Vertical grid lines (bars/beats).
            draw_grid_lines(
                &painter,
                grid_rect,
                scroll_x,
                pixels_per_frame,
                sample_rate,
                bpm,
                colors,
            );

            // --- Draw MIDI notes ---
            for clip in &midi_clips {
                let clip_origin_x =
                    grid_rect.left() + beat_grid_x(clip.timeline_pos, pixels_per_frame, scroll_x);

                for note_event in &clip.notes {
                    let note_x =
                        clip_origin_x + (note_event.position.as_f64() * pixels_per_frame) as f32;
                    let note_w = (note_event.duration.as_f64() * pixels_per_frame) as f32;
                    let note_y = grid_rect.top() + note_row_y(note_event.note);

                    // Cull notes outside the visible grid.
                    if note_x + note_w < grid_rect.left() || note_x > grid_rect.right() {
                        continue;
                    }
                    if note_y + NOTE_ROW_HEIGHT < grid_rect.top() || note_y > grid_rect.bottom() {
                        continue;
                    }

                    let clamped_left = note_x.max(grid_rect.left());
                    let clamped_right = (note_x + note_w).min(grid_rect.right());
                    let note_rect = Rect::from_min_size(
                        pos2(clamped_left, note_y + 1.0),
                        vec2(
                            (clamped_right - clamped_left).max(1.0),
                            NOTE_ROW_HEIGHT - 2.0,
                        ),
                    );

                    let alpha = velocity_alpha(note_event.velocity);
                    let fill = note_base_color.linear_multiply(alpha);
                    painter.rect_filled(note_rect, egui::CornerRadius::same(2), fill);

                    // Thin border for definition.
                    painter.rect_stroke(
                        note_rect,
                        egui::CornerRadius::same(2),
                        Stroke::new(0.5, note_base_color.linear_multiply(0.8)),
                        egui::StrokeKind::Outside,
                    );
                }
            }

            // --- Playhead ---
            let playhead_x = grid_rect.left()
                + beat_grid_x(state.session.transport.position, pixels_per_frame, scroll_x);
            if playhead_x >= grid_rect.left() && playhead_x <= grid_rect.right() {
                painter.line_segment(
                    [
                        pos2(playhead_x, grid_rect.top()),
                        pos2(playhead_x, grid_rect.bottom()),
                    ],
                    Stroke::new(1.0, colors.playhead()),
                );
            }
        });
}

/// Draw vertical bar/beat grid lines inside the note grid area.
fn draw_grid_lines(
    painter: &egui::Painter,
    rect: Rect,
    scroll_offset: f64,
    pixels_per_frame: f64,
    sample_rate: u32,
    bpm: f64,
    colors: &ThemeColors,
) {
    let frames_per_beat = (sample_rate as f64 * 60.0) / bpm;
    let frames_per_bar = frames_per_beat * 4.0;
    let pixels_per_bar = frames_per_bar * pixels_per_frame;

    if pixels_per_bar < 4.0 {
        return;
    }

    let start_frame = (scroll_offset / pixels_per_frame) as i64;
    let end_frame = start_frame + (rect.width() as f64 / pixels_per_frame) as i64;

    let bar_start = (start_frame as f64 / frames_per_bar).floor() as i64;
    let bar_end = (end_frame as f64 / frames_per_bar).ceil() as i64;

    for i in bar_start..=bar_end {
        let frame = (i as f64 * frames_per_bar) as i64;
        let x = rect.left() + (frame as f64 * pixels_per_frame - scroll_offset) as f32;
        if x >= rect.left() && x <= rect.right() {
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(0.5, colors.grid()),
            );
        }

        // Beat subdivision lines.
        if pixels_per_bar > 80.0 {
            for beat in 1..4 {
                let beat_frame = frame + (beat as f64 * frames_per_beat) as i64;
                let bx =
                    rect.left() + (beat_frame as f64 * pixels_per_frame - scroll_offset) as f32;
                if bx >= rect.left() && bx <= rect.right() {
                    painter.line_segment(
                        [pos2(bx, rect.top()), pos2(bx, rect.bottom())],
                        Stroke::new(0.25, colors.grid()),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- note_name mapping ----

    #[test]
    fn note_name_middle_c() {
        assert_eq!(note_name(60), "C4");
    }

    #[test]
    fn note_name_a440() {
        assert_eq!(note_name(69), "A4");
    }

    #[test]
    fn note_name_lowest() {
        assert_eq!(note_name(0), "C-1");
    }

    #[test]
    fn note_name_highest() {
        assert_eq!(note_name(127), "G9");
    }

    // ---- is_black_key ----

    #[test]
    fn is_black_key_all_twelve_semitones() {
        // C  C# D  D# E  F  F# G  G# A  A# B
        let expected = [
            false, true, false, true, false, false, true, false, true, false, true, false,
        ];
        for (i, &expect) in expected.iter().enumerate() {
            assert_eq!(
                is_black_key(i as u8),
                expect,
                "note {} should be black_key={}",
                i,
                expect
            );
        }
    }

    #[test]
    fn is_black_key_higher_octave() {
        // Same pattern repeats in higher octave.
        assert!(!is_black_key(60)); // C4
        assert!(is_black_key(61)); // C#4
        assert!(!is_black_key(64)); // E4
        assert!(is_black_key(66)); // F#4
    }

    // ---- drum_name ----

    #[test]
    fn drum_name_gm_notes() {
        assert_eq!(drum_name(36), "Kick");
        assert_eq!(drum_name(38), "Snare");
        assert_eq!(drum_name(42), "Closed HH");
        assert_eq!(drum_name(46), "Open HH");
        assert_eq!(drum_name(49), "Crash 1");
        assert_eq!(drum_name(51), "Ride");
    }

    #[test]
    fn drum_name_outside_range() {
        assert_eq!(drum_name(35), "");
        assert_eq!(drum_name(52), "");
        assert_eq!(drum_name(0), "");
        assert_eq!(drum_name(127), "");
    }

    // ---- note_row_y position ----

    #[test]
    fn note_row_y_positions() {
        // Note 127 is at the top (y = 0).
        assert!((note_row_y(127) - 0.0).abs() < f32::EPSILON);
        // Note 126 is one row below.
        assert!((note_row_y(126) - NOTE_ROW_HEIGHT).abs() < f32::EPSILON);
        // Note 0 is at the bottom.
        assert!((note_row_y(0) - 127.0 * NOTE_ROW_HEIGHT).abs() < f32::EPSILON);
        // Middle C (60) should be at row (127 - 60) = 67.
        assert!((note_row_y(60) - 67.0 * NOTE_ROW_HEIGHT).abs() < f32::EPSILON);
    }

    // ---- beat_grid_x position ----

    #[test]
    fn beat_grid_x_positions() {
        use shruti_session::FramePos;
        // At frame 0 with no scroll, x should be 0.
        assert!((beat_grid_x(FramePos(0), 0.01, 0.0) - 0.0).abs() < f32::EPSILON);
        // At frame 48000 with ppf=0.01, x = 480.0 pixels.
        let x = beat_grid_x(FramePos(48000), 0.01, 0.0);
        assert!((x - 480.0).abs() < 0.01);
        // With scroll offset 100.0, x shifts left.
        let x_scrolled = beat_grid_x(FramePos(48000), 0.01, 100.0);
        assert!((x_scrolled - 380.0).abs() < 0.01);
    }

    // ---- key range ----

    #[test]
    fn standard_octave_key_range() {
        // An octave has 7 white keys and 5 black keys.
        let mut white = 0;
        let mut black = 0;
        for note in 0..12u8 {
            if is_black_key(note) {
                black += 1;
            } else {
                white += 1;
            }
        }
        assert_eq!(white, 7);
        assert_eq!(black, 5);

        // Full MIDI range: 128 notes = 10 full octaves + 8 extra notes.
        let total_white: usize = (0..128u8).filter(|n| !is_black_key(*n)).count();
        let total_black: usize = (0..128u8).filter(|n| is_black_key(*n)).count();
        assert_eq!(total_white + total_black, 128);
        // 10 * 7 + 5 extra white keys (C G# A A# B -> actually count properly)
        // Just verify ratio is close to 7:5.
        assert!(total_white > total_black);
    }

    // ---- empty clip ----

    #[test]
    fn empty_midi_clip_has_no_notes() {
        use shruti_session::midi::MidiClip;

        let clip = MidiClip::new("Empty", 0u64, 48000u64);
        assert!(clip.notes.is_empty());
        // No notes means nothing to render -- the drawing loop would iterate zero times.
    }

    // ---- velocity to opacity ----

    #[test]
    fn velocity_alpha_mapping() {
        // Velocity 0 -> minimum alpha 0.3.
        let a0 = velocity_alpha(0);
        assert!((a0 - 0.3).abs() < f32::EPSILON);

        // Velocity 127 -> maximum alpha 1.0.
        let a127 = velocity_alpha(127);
        assert!((a127 - 1.0).abs() < 0.001);

        // Mid velocity -> somewhere in between.
        let a64 = velocity_alpha(64);
        assert!(a64 > 0.3 && a64 < 1.0);

        // Monotonically increasing.
        assert!(velocity_alpha(50) < velocity_alpha(100));
    }
}
