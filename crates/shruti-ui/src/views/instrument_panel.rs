use egui::Ui;

use shruti_session::TrackKind;

use crate::state::UiState;
use crate::theme::ThemeColors;
use crate::widgets::knob::Knob;

/// Number of parameter knobs per row in the grid.
const KNOBS_PER_ROW: usize = 4;

/// Returns `true` if the given `TrackKind` is an instrument-type track
/// (Instrument, DrumMachine, Sampler, or AiPlayer).
pub fn is_instrument_track(kind: &TrackKind) -> bool {
    matches!(
        kind,
        TrackKind::Instrument { .. }
            | TrackKind::DrumMachine { .. }
            | TrackKind::Sampler { .. }
            | TrackKind::AiPlayer { .. }
    )
}

/// Compute the number of rows needed to display `param_count` knobs
/// in a grid with `KNOBS_PER_ROW` columns.
pub fn param_grid_rows(param_count: usize) -> usize {
    if param_count == 0 {
        0
    } else {
        param_count.div_ceil(KNOBS_PER_ROW)
    }
}

/// Return a human-readable header string for the given instrument track kind.
pub fn instrument_header(kind: &TrackKind) -> &'static str {
    match kind {
        TrackKind::Instrument { .. } => "Synth",
        TrackKind::DrumMachine { .. } => "Drum Machine",
        TrackKind::Sampler { .. } => "Sampler",
        TrackKind::AiPlayer { .. } => "AI Player",
        _ => "Unknown",
    }
}

/// Draw the instrument rack panel view.
pub fn instrument_panel_view(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    let track_idx = match state.selected_track {
        Some(idx) if idx < state.session.tracks.len() => idx,
        _ => {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new("Select an instrument track")
                        .size(14.0)
                        .color(colors.text_secondary()),
                );
            });
            return;
        }
    };

    if !is_instrument_track(&state.session.tracks[track_idx].kind) {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                egui::RichText::new("Select an instrument track")
                    .size(14.0)
                    .color(colors.text_secondary()),
            );
        });
        return;
    }

    let track_name = state.session.tracks[track_idx].name.clone();
    let kind = state.session.tracks[track_idx].kind.clone();
    let header = instrument_header(&kind);

    // Header: track name + instrument type
    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new(&track_name)
                .size(16.0)
                .color(colors.text_primary()),
        );
        ui.label(
            egui::RichText::new(format!("  [{header}]"))
                .size(12.0)
                .color(colors.text_secondary()),
        );
    });

    ui.separator();

    // Kind-specific info section
    match &kind {
        TrackKind::Instrument { instrument_type } => {
            let type_name = instrument_type.as_deref().unwrap_or("No instrument loaded");
            ui.label(
                egui::RichText::new(format!("Type: {type_name}"))
                    .size(11.0)
                    .color(colors.text_secondary()),
            );
        }
        TrackKind::DrumMachine {
            kit_name,
            pad_count,
        } => {
            let kit = kit_name.as_deref().unwrap_or("No kit loaded");
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Kit: {kit}"))
                        .size(11.0)
                        .color(colors.text_secondary()),
                );
                ui.label(
                    egui::RichText::new(format!("Pads: {pad_count}"))
                        .size(11.0)
                        .color(colors.text_secondary()),
                );
            });
        }
        TrackKind::Sampler {
            preset_name,
            zone_count,
        } => {
            let preset = preset_name.as_deref().unwrap_or("No preset loaded");
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Preset: {preset}"))
                        .size(11.0)
                        .color(colors.text_secondary()),
                );
                ui.label(
                    egui::RichText::new(format!("Zones: {zone_count}"))
                        .size(11.0)
                        .color(colors.text_secondary()),
                );
            });
        }
        TrackKind::AiPlayer {
            model_name,
            style,
            creativity,
        } => {
            let model = model_name.as_deref().unwrap_or("No model loaded");
            let style_str = style.as_deref().unwrap_or("default");
            ui.label(
                egui::RichText::new(format!("Model: {model}"))
                    .size(11.0)
                    .color(colors.text_secondary()),
            );
            ui.label(
                egui::RichText::new(format!("Style: {style_str}"))
                    .size(11.0)
                    .color(colors.text_secondary()),
            );
            // Creativity slider (read-only display from the cloned kind)
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Creativity: {creativity:.0}%",
                        creativity = creativity * 100.0
                    ))
                    .size(11.0)
                    .color(colors.text_secondary()),
                );
            });
        }
        _ => {}
    }

    ui.add_space(8.0);
    ui.separator();

    // Parameter grid
    let param_count = state.session.tracks[track_idx].instrument_params.len();
    if param_count == 0 {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("No parameters")
                .size(11.0)
                .color(colors.text_secondary()),
        );
    } else {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Parameters")
                .size(12.0)
                .color(colors.text_primary()),
        );
        ui.add_space(4.0);

        let rows = param_grid_rows(param_count);
        for row in 0..rows {
            ui.horizontal(|ui| {
                for col in 0..KNOBS_PER_ROW {
                    let idx = row * KNOBS_PER_ROW + col;
                    if idx >= param_count {
                        break;
                    }
                    ui.vertical(|ui| {
                        let mut val = state.session.tracks[track_idx].instrument_params[idx];
                        ui.add(
                            Knob::new(&mut val, 0.0, 1.0, colors)
                                .with_label(&format!("P{}", idx + 1)),
                        );
                        state.session.tracks[track_idx].instrument_params[idx] = val;
                    });
                }
            });
        }
    }

    ui.add_space(12.0);
    ui.separator();

    // Preset section placeholder
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Preset")
            .size(12.0)
            .color(colors.text_primary()),
    );
    ui.label(
        egui::RichText::new("(presets not yet available)")
            .size(10.0)
            .color(colors.text_secondary()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_instrument_track tests ---

    #[test]
    fn instrument_kind_is_instrument_track() {
        let kind = TrackKind::Instrument {
            instrument_type: Some("SubtractiveSynth".to_string()),
        };
        assert!(is_instrument_track(&kind));
    }

    #[test]
    fn drum_machine_kind_is_instrument_track() {
        let kind = TrackKind::DrumMachine {
            kit_name: None,
            pad_count: 16,
        };
        assert!(is_instrument_track(&kind));
    }

    #[test]
    fn sampler_kind_is_instrument_track() {
        let kind = TrackKind::Sampler {
            preset_name: None,
            zone_count: 0,
        };
        assert!(is_instrument_track(&kind));
    }

    #[test]
    fn ai_player_kind_is_instrument_track() {
        let kind = TrackKind::AiPlayer {
            model_name: None,
            style: None,
            creativity: 0.5,
        };
        assert!(is_instrument_track(&kind));
    }

    #[test]
    fn audio_kind_is_not_instrument_track() {
        assert!(!is_instrument_track(&TrackKind::Audio));
    }

    #[test]
    fn bus_kind_is_not_instrument_track() {
        assert!(!is_instrument_track(&TrackKind::Bus));
    }

    #[test]
    fn midi_kind_is_not_instrument_track() {
        assert!(!is_instrument_track(&TrackKind::Midi));
    }

    #[test]
    fn master_kind_is_not_instrument_track() {
        assert!(!is_instrument_track(&TrackKind::Master));
    }

    // --- param_grid_rows tests ---

    #[test]
    fn grid_rows_zero_params() {
        assert_eq!(param_grid_rows(0), 0);
    }

    #[test]
    fn grid_rows_one_param() {
        assert_eq!(param_grid_rows(1), 1);
    }

    #[test]
    fn grid_rows_exact_row() {
        assert_eq!(param_grid_rows(4), 1);
        assert_eq!(param_grid_rows(8), 2);
        assert_eq!(param_grid_rows(12), 3);
    }

    #[test]
    fn grid_rows_partial_row() {
        assert_eq!(param_grid_rows(5), 2);
        assert_eq!(param_grid_rows(7), 2);
        assert_eq!(param_grid_rows(9), 3);
    }

    // --- instrument_header tests ---

    #[test]
    fn header_for_instrument() {
        let kind = TrackKind::Instrument {
            instrument_type: None,
        };
        assert_eq!(instrument_header(&kind), "Synth");
    }

    #[test]
    fn header_for_drum_machine() {
        let kind = TrackKind::DrumMachine {
            kit_name: None,
            pad_count: 16,
        };
        assert_eq!(instrument_header(&kind), "Drum Machine");
    }

    #[test]
    fn header_for_sampler() {
        let kind = TrackKind::Sampler {
            preset_name: None,
            zone_count: 0,
        };
        assert_eq!(instrument_header(&kind), "Sampler");
    }

    #[test]
    fn header_for_ai_player() {
        let kind = TrackKind::AiPlayer {
            model_name: None,
            style: None,
            creativity: 0.5,
        };
        assert_eq!(instrument_header(&kind), "AI Player");
    }

    #[test]
    fn header_for_non_instrument() {
        assert_eq!(instrument_header(&TrackKind::Audio), "Unknown");
    }

    // --- default parameter initialization tests ---

    #[test]
    fn new_instrument_track_has_empty_params() {
        use shruti_session::Track;
        let track = Track::new_instrument("Test Synth", Some("TestSynth".to_string()));
        assert!(track.instrument_params.is_empty());
    }

    #[test]
    fn instrument_params_can_be_populated() {
        use shruti_session::Track;
        let mut track = Track::new_instrument("Test Synth", Some("TestSynth".to_string()));
        track.instrument_params = vec![0.0, 0.5, 1.0, 0.75];
        assert_eq!(track.instrument_params.len(), 4);
        assert!((track.instrument_params[1] - 0.5).abs() < f32::EPSILON);
    }
}
