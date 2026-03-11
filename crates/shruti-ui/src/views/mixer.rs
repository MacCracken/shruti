use egui::{ScrollArea, Ui, vec2};

use crate::state::UiState;
use crate::theme::ThemeColors;
use crate::widgets::{fader::Fader, knob::Knob, meter::LevelMeter, track_header::track_color};

const STRIP_WIDTH: f32 = 72.0;

/// Draw the mixer view.
pub fn mixer_view(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    ScrollArea::horizontal()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let track_count = state.session.tracks.len();

                for track_idx in 0..track_count {
                    channel_strip(ui, state, track_idx, colors);

                    // Separator between strips
                    if track_idx < track_count - 1 {
                        ui.separator();
                    }
                }
            });
        });
}

fn channel_strip(ui: &mut Ui, state: &mut UiState, track_idx: usize, colors: &ThemeColors) {
    ui.vertical(|ui| {
        ui.set_width(STRIP_WIDTH);

        let color = track_color(track_idx);

        // Track name with color indicator
        ui.horizontal(|ui| {
            let (indicator_rect, _) = ui.allocate_exact_size(vec2(4.0, 14.0), egui::Sense::hover());
            if ui.is_rect_visible(indicator_rect) {
                ui.painter_at(indicator_rect)
                    .rect_filled(indicator_rect, 1.0, color);
            }

            let name = &state.session.tracks[track_idx].name;
            ui.label(
                egui::RichText::new(name)
                    .size(10.0)
                    .color(colors.text_primary()),
            );
        });

        ui.add_space(4.0);

        // M S buttons
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            let muted = state.session.tracks[track_idx].muted;
            let mute_color = if muted {
                colors.mute_orange()
            } else {
                colors.surface()
            };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("M").size(9.0).color(if muted {
                        egui::Color32::WHITE
                    } else {
                        colors.text_secondary()
                    }))
                    .fill(mute_color)
                    .min_size(vec2(18.0, 14.0)),
                )
                .clicked()
            {
                state.session.tracks[track_idx].muted = !muted;
            }

            let solo = state.session.tracks[track_idx].solo;
            let solo_color = if solo {
                colors.solo_yellow()
            } else {
                colors.surface()
            };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("S").size(9.0).color(if solo {
                        egui::Color32::BLACK
                    } else {
                        colors.text_secondary()
                    }))
                    .fill(solo_color)
                    .min_size(vec2(18.0, 14.0)),
                )
                .clicked()
            {
                state.session.tracks[track_idx].solo = !solo;
            }
        });

        ui.add_space(4.0);

        // Pan knob
        ui.vertical_centered(|ui| {
            let mut pan = state.session.tracks[track_idx].pan;
            ui.add(Knob::pan(&mut pan, colors));
            state.session.tracks[track_idx].pan = pan;
        });

        ui.add_space(4.0);

        // Fader + Meter side by side
        ui.horizontal(|ui| {
            // Fader
            let mut gain = state.session.tracks[track_idx].gain;
            ui.add(Fader::new(&mut gain, colors).height(140.0));
            state.session.tracks[track_idx].gain = gain;

            // Meter
            let meter_data = state
                .meter_levels
                .get(track_idx)
                .copied()
                .unwrap_or(([0.0; 2], [0.0; 2]));
            ui.add(LevelMeter::stereo(meter_data.0, meter_data.1, colors).height(140.0));
        });

        ui.add_space(2.0);

        // dB readout
        ui.vertical_centered(|ui| {
            let db = if state.session.tracks[track_idx].gain < 1e-10 {
                "-inf".to_string()
            } else {
                format!("{:.1}", 20.0 * state.session.tracks[track_idx].gain.log10())
            };
            ui.label(
                egui::RichText::new(db)
                    .monospace()
                    .size(9.0)
                    .color(colors.text_secondary()),
            );
        });
    });
}
