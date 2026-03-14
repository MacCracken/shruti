use egui::{Color32, CornerRadius, Layout, Ui, vec2};

use crate::state::UiState;
use crate::theme::ThemeColors;
use shruti_session::TransportState;

/// Format frame position as (hours, minutes, seconds).
pub(crate) fn format_time(position_frames: u64, sample_rate: u32) -> (u32, u32, f64) {
    if sample_rate == 0 {
        return (0, 0, 0.0);
    }
    let total_secs = position_frames as f64 / sample_rate as f64;
    let hours = (total_secs / 3600.0) as u32;
    let minutes = ((total_secs % 3600.0) / 60.0) as u32;
    let seconds = total_secs % 60.0;
    (hours, minutes, seconds)
}

/// Format frame position as (bar, beat, tick).
///
/// `time_sig_num` is the number of beats per bar (e.g. 4 for 4/4 time).
pub(crate) fn format_bar_beat(
    position_frames: u64,
    sample_rate: u32,
    bpm: f64,
    time_sig_num: u8,
) -> (u32, u32, u32) {
    if sample_rate == 0 || bpm <= 0.0 || time_sig_num == 0 {
        return (1, 1, 0);
    }
    let frames_per_beat = sample_rate as f64 * 60.0 / bpm;
    let total_beats = position_frames as f64 / frames_per_beat;
    let bar = (total_beats / time_sig_num as f64) as u32 + 1;
    let beat = (total_beats % time_sig_num as f64) as u32 + 1;
    let tick = ((total_beats.fract()) * 960.0) as u32;
    (bar, beat, tick)
}

/// Draw the transport bar at the top of the window.
pub fn transport_bar(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    ui.horizontal(|ui| {
        ui.set_min_height(32.0);
        ui.spacing_mut().item_spacing.x = 4.0;

        // Transport controls
        transport_buttons(ui, state, colors);

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        // Position display
        position_display(ui, state, colors);

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        // Tempo
        tempo_display(ui, state, colors);

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        // Loop toggle
        loop_toggle(ui, state, colors);

        // Right-align session name (with dirty indicator)
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            let title = state.title_bar_text();
            ui.label(
                egui::RichText::new(title)
                    .size(11.0)
                    .color(colors.text_secondary()),
            );
        });
    });
}

fn transport_buttons(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    let btn_size = vec2(28.0, 24.0);
    let is_playing = state.session.transport.state == TransportState::Playing;
    let is_recording = state.recording;

    // Rewind
    if ui
        .add(
            egui::Button::new(egui::RichText::new("⏮").size(14.0))
                .fill(colors.surface())
                .corner_radius(CornerRadius::same(3))
                .min_size(btn_size),
        )
        .clicked()
    {
        state.session.transport.position = shruti_session::FramePos::ZERO;
    }

    // Stop
    if ui
        .add(
            egui::Button::new(egui::RichText::new("⏹").size(14.0))
                .fill(colors.surface())
                .corner_radius(CornerRadius::same(3))
                .min_size(btn_size),
        )
        .clicked()
    {
        state.session.transport.state = TransportState::Stopped;
        state.recording = false;
    }

    // Play/Pause
    let play_icon = if is_playing { "⏸" } else { "▶" };
    let play_color = if is_playing {
        colors.transport_active()
    } else {
        colors.surface()
    };
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new(play_icon)
                    .size(14.0)
                    .color(if is_playing {
                        Color32::WHITE
                    } else {
                        colors.text_primary()
                    }),
            )
            .fill(play_color)
            .corner_radius(CornerRadius::same(3))
            .min_size(btn_size),
        )
        .clicked()
    {
        if is_playing {
            state.session.transport.state = TransportState::Paused;
        } else {
            state.session.transport.state = TransportState::Playing;
        }
    }

    // Record
    let rec_color = if is_recording {
        colors.record_red()
    } else {
        colors.surface()
    };
    if ui
        .add(
            egui::Button::new(egui::RichText::new("⏺").size(14.0).color(if is_recording {
                Color32::WHITE
            } else {
                colors.record_red()
            }))
            .fill(rec_color)
            .corner_radius(CornerRadius::same(3))
            .min_size(btn_size),
        )
        .clicked()
    {
        state.recording = !state.recording;
    }
}

fn position_display(ui: &mut Ui, state: &UiState, colors: &ThemeColors) {
    let pos = state.session.transport.position;
    let sr = state.session.sample_rate;
    let bpm = state.session.transport.bpm;

    // Time display (hh:mm:ss.ms)
    let (hours, minutes, seconds) = format_time(pos.0, sr);
    let time_str = format!("{hours:02}:{minutes:02}:{seconds:06.3}");

    ui.label(
        egui::RichText::new(time_str)
            .monospace()
            .size(14.0)
            .color(colors.text_primary()),
    );

    ui.add_space(8.0);

    // Bar:Beat display
    let time_sig_num = state.session.transport.time_sig_num;
    let (bar, beat, tick) = format_bar_beat(pos.0, sr, bpm, time_sig_num);
    let bar_str = format!("{bar:>3}.{beat}.{tick:03}");

    ui.label(
        egui::RichText::new(bar_str)
            .monospace()
            .size(14.0)
            .color(colors.accent()),
    );
}

fn tempo_display(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    ui.label(
        egui::RichText::new("BPM")
            .size(9.0)
            .color(colors.text_secondary()),
    );

    let mut bpm = state.session.transport.bpm as f32;
    let response = ui.add(
        egui::DragValue::new(&mut bpm)
            .range(20.0..=999.0)
            .speed(0.5)
            .fixed_decimals(1),
    );
    if response.changed() {
        state.session.transport.bpm = bpm as f64;
    }

    // Time signature
    ui.label(
        egui::RichText::new(format!(
            "{}/{}",
            state.session.transport.time_sig_num, state.session.transport.time_sig_den
        ))
        .monospace()
        .size(11.0)
        .color(colors.text_secondary()),
    );
}

fn loop_toggle(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    let loop_active = state.session.transport.loop_enabled;
    let loop_color = if loop_active {
        colors.accent()
    } else {
        colors.surface()
    };

    if ui
        .add(
            egui::Button::new(egui::RichText::new("⟳").size(14.0).color(if loop_active {
                Color32::WHITE
            } else {
                colors.text_secondary()
            }))
            .fill(loop_color)
            .corner_radius(CornerRadius::same(3))
            .min_size(vec2(28.0, 24.0)),
        )
        .clicked()
    {
        state.session.transport.loop_enabled = !state.session.transport.loop_enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- format_time tests ----

    #[test]
    fn format_time_zero() {
        let (h, m, s) = format_time(0, 48000);
        assert_eq!(h, 0);
        assert_eq!(m, 0);
        assert!(s.abs() < 1e-9);
    }

    #[test]
    fn format_time_one_second() {
        let (h, m, s) = format_time(48000, 48000);
        assert_eq!(h, 0);
        assert_eq!(m, 0);
        assert!((s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn format_time_one_minute() {
        let (h, m, s) = format_time(48000 * 60, 48000);
        assert_eq!(h, 0);
        assert_eq!(m, 1);
        assert!(s.abs() < 1e-9);
    }

    #[test]
    fn format_time_one_hour() {
        let (h, m, s) = format_time(48000 * 3600, 48000);
        assert_eq!(h, 1);
        assert_eq!(m, 0);
        assert!(s.abs() < 1e-9);
    }

    #[test]
    fn format_time_complex() {
        // 1 hour, 23 minutes, 45 seconds = 5025 seconds
        let frames = 48000u64 * 5025;
        let (h, m, s) = format_time(frames, 48000);
        assert_eq!(h, 1);
        assert_eq!(m, 23);
        assert!((s - 45.0).abs() < 1e-9);
    }

    #[test]
    fn format_time_44100_sample_rate() {
        let (h, m, s) = format_time(44100, 44100);
        assert_eq!(h, 0);
        assert_eq!(m, 0);
        assert!((s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn format_time_zero_sample_rate() {
        let (h, m, s) = format_time(48000, 0);
        assert_eq!(h, 0);
        assert_eq!(m, 0);
        assert!(s.abs() < 1e-9);
    }

    // ---- format_bar_beat tests ----

    #[test]
    fn format_bar_beat_at_zero() {
        let (bar, beat, tick) = format_bar_beat(0, 48000, 120.0, 4);
        assert_eq!(bar, 1);
        assert_eq!(beat, 1);
        assert_eq!(tick, 0);
    }

    #[test]
    fn format_bar_beat_one_beat() {
        // 120 BPM, 48kHz => 24000 frames/beat
        let (bar, beat, tick) = format_bar_beat(24000, 48000, 120.0, 4);
        assert_eq!(bar, 1);
        assert_eq!(beat, 2);
        assert_eq!(tick, 0);
    }

    #[test]
    fn format_bar_beat_one_bar() {
        // 120 BPM, 4/4 time => 4 beats/bar => 96000 frames/bar
        let (bar, beat, tick) = format_bar_beat(96000, 48000, 120.0, 4);
        assert_eq!(bar, 2);
        assert_eq!(beat, 1);
        assert_eq!(tick, 0);
    }

    #[test]
    fn format_bar_beat_three_four_time() {
        // 120 BPM, 3/4 time => 3 beats/bar => 72000 frames/bar
        let (bar, beat, tick) = format_bar_beat(72000, 48000, 120.0, 3);
        assert_eq!(bar, 2);
        assert_eq!(beat, 1);
        assert_eq!(tick, 0);
    }

    #[test]
    fn format_bar_beat_half_beat_tick() {
        // Half a beat at 120 BPM = 12000 frames
        let (bar, beat, tick) = format_bar_beat(12000, 48000, 120.0, 4);
        assert_eq!(bar, 1);
        assert_eq!(beat, 1);
        assert_eq!(tick, 480); // half beat = 480 ticks
    }

    #[test]
    fn format_bar_beat_60_bpm() {
        // 60 BPM, 48kHz => 48000 frames/beat
        let (bar, beat, tick) = format_bar_beat(48000, 48000, 60.0, 4);
        assert_eq!(bar, 1);
        assert_eq!(beat, 2);
        assert_eq!(tick, 0);
    }

    #[test]
    fn format_bar_beat_zero_bpm() {
        let (bar, beat, tick) = format_bar_beat(48000, 48000, 0.0, 4);
        assert_eq!(bar, 1);
        assert_eq!(beat, 1);
        assert_eq!(tick, 0);
    }

    #[test]
    fn format_bar_beat_zero_sample_rate() {
        let (bar, beat, tick) = format_bar_beat(48000, 0, 120.0, 4);
        assert_eq!(bar, 1);
        assert_eq!(beat, 1);
        assert_eq!(tick, 0);
    }

    #[test]
    fn format_bar_beat_zero_time_sig() {
        let (bar, beat, tick) = format_bar_beat(48000, 48000, 120.0, 0);
        assert_eq!(bar, 1);
        assert_eq!(beat, 1);
        assert_eq!(tick, 0);
    }
}
