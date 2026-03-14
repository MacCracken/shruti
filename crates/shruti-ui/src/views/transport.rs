use egui::{Color32, CornerRadius, Layout, Ui, vec2};

use crate::state::UiState;
use crate::theme::ThemeColors;
use shruti_session::TransportState;

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
    let total_secs = pos.as_f64() / sr as f64;
    let hours = (total_secs / 3600.0) as u32;
    let minutes = ((total_secs % 3600.0) / 60.0) as u32;
    let seconds = total_secs % 60.0;

    let time_str = format!("{hours:02}:{minutes:02}:{seconds:06.3}");

    ui.label(
        egui::RichText::new(time_str)
            .monospace()
            .size(14.0)
            .color(colors.text_primary()),
    );

    ui.add_space(8.0);

    // Bar:Beat display
    let frames_per_beat = (sr as f64 * 60.0) / bpm;
    let total_beats = pos.as_f64() / frames_per_beat;
    let bar = (total_beats / 4.0).floor() as u32 + 1;
    let beat = (total_beats % 4.0).floor() as u32 + 1;
    let tick = ((total_beats % 1.0) * 960.0) as u32;

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
