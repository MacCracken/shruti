use egui::{Ui, vec2};

use crate::theme::ThemeColors;
use crate::widgets::knob::Knob;

/// Number of pads in the drum grid.
const GRID_COLS: usize = 4;
const GRID_ROWS: usize = 4;
const TOTAL_PADS: usize = GRID_COLS * GRID_ROWS;
const STEPS_PER_PAD: usize = 16;

/// Pad size in pixels.
const PAD_SIZE: f32 = 48.0;
/// Step button size in pixels.
const STEP_SIZE: f32 = 24.0;

/// Index into `params` where the selected pad is stored.
const SELECTED_PAD_INDEX: usize = 320;

/// Base index for step sequencer data: params[STEP_BASE + pad * 16 + step].
const STEP_BASE: usize = 64;

/// Number of per-pad parameters (pitch, gain, pan, decay).
const PAD_PARAM_COUNT: usize = 4;

/// Return the GM drum name for a pad index.
fn gm_drum_name(pad: usize) -> &'static str {
    match pad {
        0 => "Kick",
        1 => "Snare",
        2 => "Closed HH",
        3 => "Open HH",
        4 => "Low Tom",
        5 => "Mid Tom",
        6 => "Hi Tom",
        7 => "Crash",
        8 => "Ride",
        9 => "Clap",
        10 => "Rim",
        11 => "Cowbell",
        12 => "Hi Conga",
        13 => "Lo Conga",
        14 => "Tambourine",
        15 => "Shaker",
        _ => "Pad",
    }
}

/// Compute the parameter index for a per-pad knob parameter.
///
/// Layout: `pad * 4 + offset` where offset is 0=pitch, 1=gain, 2=pan, 3=decay.
fn pad_param_index(pad: usize, offset: usize) -> usize {
    pad * PAD_PARAM_COUNT + offset
}

/// Compute the parameter index for a step in the sequencer.
///
/// Layout: `STEP_BASE + pad * 16 + step`.
fn step_param_index(pad: usize, step: usize) -> usize {
    STEP_BASE + pad * STEPS_PER_PAD + step
}

/// Return the label for a pattern bank.
fn pattern_bank_label(bank: usize) -> &'static str {
    match bank {
        0 => "A",
        1 => "B",
        2 => "C",
        3 => "D",
        _ => "?",
    }
}

/// Draw the drum machine grid view.
///
/// `params` must be large enough to hold all pad parameters, step data, and the
/// selected-pad index (at least 321 entries: 64 pad params + 256 step params +
/// selected pad at index 320).
pub fn drum_grid_view(
    ui: &mut Ui,
    params: &mut Vec<f32>,
    kit_name: &Option<String>,
    pad_count: u8,
    colors: &ThemeColors,
) {
    // Ensure params is large enough.
    let min_len = SELECTED_PAD_INDEX + 1;
    if params.len() < min_len {
        params.resize(min_len, 0.0);
    }

    let selected_pad = (params[SELECTED_PAD_INDEX] as usize).min(TOTAL_PADS - 1);

    // --- Header ---
    ui.horizontal(|ui| {
        let name = kit_name.as_deref().unwrap_or("Default Kit");
        ui.label(
            egui::RichText::new(name)
                .size(14.0)
                .color(colors.text_primary()),
        );
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(format!("{pad_count} pads"))
                .size(10.0)
                .color(colors.text_secondary()),
        );
    });

    ui.add_space(6.0);

    // --- Pad Grid (4x4) ---
    ui.horizontal(|ui| {
        // Left side: pad grid
        ui.vertical(|ui| {
            egui::Grid::new("drum_pad_grid")
                .spacing(vec2(4.0, 4.0))
                .show(ui, |ui| {
                    for row in 0..GRID_ROWS {
                        for col in 0..GRID_COLS {
                            let pad = row * GRID_COLS + col;
                            let is_selected = pad == selected_pad;

                            let fill = if is_selected {
                                colors.accent()
                            } else {
                                colors.surface()
                            };
                            let text_color = if is_selected {
                                egui::Color32::WHITE
                            } else {
                                colors.text_primary()
                            };

                            let label = format!("{}\n{}", pad + 1, gm_drum_name(pad));
                            let btn = egui::Button::new(
                                egui::RichText::new(label).size(9.0).color(text_color),
                            )
                            .fill(fill)
                            .min_size(vec2(PAD_SIZE, PAD_SIZE));

                            if ui.add(btn).clicked() {
                                params[SELECTED_PAD_INDEX] = pad as f32;
                            }
                        }
                        ui.end_row();
                    }
                });
        });

        ui.add_space(12.0);

        // Right side: selected pad controls
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Pad {} - {}",
                    selected_pad + 1,
                    gm_drum_name(selected_pad)
                ))
                .size(11.0)
                .color(colors.text_primary()),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                // Pitch
                let idx = pad_param_index(selected_pad, 0);
                ui.vertical_centered(|ui| {
                    ui.add(Knob::new(&mut params[idx], -24.0, 24.0, colors).with_label("Pitch"));
                });

                // Gain
                let idx = pad_param_index(selected_pad, 1);
                ui.vertical_centered(|ui| {
                    ui.add(Knob::new(&mut params[idx], 0.0, 1.0, colors).with_label("Gain"));
                });

                // Pan
                let idx = pad_param_index(selected_pad, 2);
                ui.vertical_centered(|ui| {
                    ui.add(Knob::pan(&mut params[idx], colors));
                });

                // Decay
                let idx = pad_param_index(selected_pad, 3);
                ui.vertical_centered(|ui| {
                    ui.add(Knob::new(&mut params[idx], 0.01, 2.0, colors).with_label("Decay"));
                });
            });
        });
    });

    ui.add_space(8.0);

    // --- Step Sequencer ---
    ui.label(
        egui::RichText::new("Steps")
            .size(10.0)
            .color(colors.text_secondary()),
    );
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for step in 0..STEPS_PER_PAD {
            let idx = step_param_index(selected_pad, step);
            let active = params[idx] > 0.5;

            let fill = if active {
                colors.accent()
            } else {
                colors.surface()
            };
            let text_color = if active {
                egui::Color32::WHITE
            } else {
                colors.text_secondary()
            };

            let btn = egui::Button::new(
                egui::RichText::new(format!("{}", step + 1))
                    .size(8.0)
                    .color(text_color),
            )
            .fill(fill)
            .min_size(vec2(STEP_SIZE, STEP_SIZE));

            if ui.add(btn).clicked() {
                params[idx] = if active { 0.0 } else { 1.0 };
            }
        }
    });

    ui.add_space(8.0);

    // --- Transport: pattern selector, swing, BPM ---
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Pattern")
                .size(10.0)
                .color(colors.text_secondary()),
        );

        // Pattern bank stored at params[321] (just after selected_pad).
        let bank_idx = SELECTED_PAD_INDEX + 1;
        if params.len() <= bank_idx + 1 {
            params.resize(bank_idx + 2, 0.0);
        }
        let current_bank = params[bank_idx] as usize;

        for bank in 0..4 {
            let label = pattern_bank_label(bank);
            let is_active = bank == current_bank;
            let fill = if is_active {
                colors.accent()
            } else {
                colors.surface()
            };
            let text_color = if is_active {
                egui::Color32::WHITE
            } else {
                colors.text_secondary()
            };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(label).size(10.0).color(text_color))
                        .fill(fill)
                        .min_size(vec2(24.0, 18.0)),
                )
                .clicked()
            {
                params[bank_idx] = bank as f32;
            }
        }

        ui.add_space(12.0);

        // Swing knob (params[322])
        let swing_idx = bank_idx + 1;
        ui.add(Knob::new(&mut params[swing_idx], 0.0, 100.0, colors).with_label("Swing"));

        ui.add_space(12.0);

        // BPM display (read-only label; actual BPM lives in engine/session)
        ui.label(
            egui::RichText::new("BPM")
                .size(10.0)
                .color(colors.text_secondary()),
        );
        ui.label(
            egui::RichText::new("120")
                .monospace()
                .size(12.0)
                .color(colors.text_primary()),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gm_drum_names_all_16() {
        let expected = [
            "Kick",
            "Snare",
            "Closed HH",
            "Open HH",
            "Low Tom",
            "Mid Tom",
            "Hi Tom",
            "Crash",
            "Ride",
            "Clap",
            "Rim",
            "Cowbell",
            "Hi Conga",
            "Lo Conga",
            "Tambourine",
            "Shaker",
        ];
        for (i, name) in expected.iter().enumerate() {
            assert_eq!(gm_drum_name(i), *name, "mismatch at pad {i}");
        }
    }

    #[test]
    fn gm_drum_name_out_of_range() {
        assert_eq!(gm_drum_name(16), "Pad");
        assert_eq!(gm_drum_name(255), "Pad");
    }

    #[test]
    fn pad_param_index_calculation() {
        // pad 0: pitch=0, gain=1, pan=2, decay=3
        assert_eq!(pad_param_index(0, 0), 0);
        assert_eq!(pad_param_index(0, 1), 1);
        assert_eq!(pad_param_index(0, 2), 2);
        assert_eq!(pad_param_index(0, 3), 3);
        // pad 1: pitch=4, gain=5, pan=6, decay=7
        assert_eq!(pad_param_index(1, 0), 4);
        assert_eq!(pad_param_index(1, 3), 7);
        // pad 15: pitch=60, gain=61, pan=62, decay=63
        assert_eq!(pad_param_index(15, 0), 60);
        assert_eq!(pad_param_index(15, 3), 63);
    }

    #[test]
    fn step_param_index_calculation() {
        // pad 0, step 0 => 64
        assert_eq!(step_param_index(0, 0), STEP_BASE);
        // pad 0, step 15 => 79
        assert_eq!(step_param_index(0, 15), STEP_BASE + 15);
        // pad 1, step 0 => 80
        assert_eq!(step_param_index(1, 0), STEP_BASE + 16);
        // pad 15, step 15 => 64 + 15*16 + 15 = 64 + 240 + 15 = 319
        assert_eq!(step_param_index(15, 15), STEP_BASE + 255);
    }

    #[test]
    fn grid_dimensions() {
        assert_eq!(GRID_COLS * GRID_ROWS, 16);
        assert_eq!(TOTAL_PADS, 16);
        assert_eq!(GRID_COLS, 4);
        assert_eq!(GRID_ROWS, 4);
    }

    #[test]
    fn pattern_bank_labels() {
        assert_eq!(pattern_bank_label(0), "A");
        assert_eq!(pattern_bank_label(1), "B");
        assert_eq!(pattern_bank_label(2), "C");
        assert_eq!(pattern_bank_label(3), "D");
        assert_eq!(pattern_bank_label(4), "?");
    }

    #[test]
    fn selected_pad_bounds() {
        // Selected pad is clamped to 0..15
        let max_valid = TOTAL_PADS - 1;
        assert_eq!(max_valid, 15);
        // If params[320] is set beyond range, it should be clamped
        let too_high: usize = 999;
        let clamped = too_high.min(TOTAL_PADS - 1);
        assert_eq!(clamped, 15);
        let zero: usize = 0;
        let clamped = zero.min(TOTAL_PADS - 1);
        assert_eq!(clamped, 0);
    }

    #[test]
    fn step_data_does_not_overlap_pad_params() {
        // Pad params occupy indices 0..63 (16 pads * 4 params)
        let last_pad_param = pad_param_index(15, 3);
        let first_step = step_param_index(0, 0);
        assert_eq!(last_pad_param, 63);
        assert_eq!(first_step, 64);
        assert!(first_step > last_pad_param);
    }

    #[test]
    fn selected_pad_index_after_step_data() {
        // Step data occupies 64..319 (256 entries), selected pad at 320
        let last_step = step_param_index(15, 15);
        assert_eq!(last_step, 319);
        assert!(SELECTED_PAD_INDEX > last_step);
    }
}
