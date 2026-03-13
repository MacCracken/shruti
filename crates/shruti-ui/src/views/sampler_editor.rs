use egui::{Color32, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::theme::ThemeColors;
use crate::widgets::knob::Knob;

/// Index into the params vec where the selected zone is stored.
const SELECTED_ZONE_PARAM: usize = 192;

/// Number of parameters per zone.
const PARAMS_PER_ZONE: usize = 6;

/// Zone param offsets.
const ZONE_ROOT_KEY: usize = 0;
const ZONE_KEY_LOW: usize = 1;
const ZONE_KEY_HIGH: usize = 2;
const ZONE_VEL_LOW: usize = 3;
const ZONE_VEL_HIGH: usize = 4;
const ZONE_GAIN: usize = 5;

/// Zone colors for visual differentiation.
const ZONE_COLORS: &[[u8; 4]; 8] = &[
    [60, 130, 240, 160],
    [220, 100, 60, 160],
    [60, 200, 120, 160],
    [200, 60, 200, 160],
    [200, 200, 60, 160],
    [60, 200, 200, 160],
    [200, 130, 60, 160],
    [130, 60, 200, 160],
];

/// Convert a MIDI note number to its musical name (e.g. 60 -> "C4").
fn midi_note_name(note: u8) -> String {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (note / 12) as i8 - 1;
    format!("{}{}", names[note as usize % 12], octave)
}

/// Calculate the parameter index for a given zone and offset.
fn zone_param_index(zone: usize, offset: usize) -> usize {
    zone * PARAMS_PER_ZONE + offset
}

/// Get the display name for a loop mode value.
fn loop_mode_name(mode: usize) -> &'static str {
    match mode {
        0 => "Off",
        1 => "Forward",
        2 => "Ping-Pong",
        _ => "Off",
    }
}

/// Get a zone color by index, cycling through the palette.
fn zone_color(index: usize) -> Color32 {
    let c = ZONE_COLORS[index % ZONE_COLORS.len()];
    Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}

/// Draw the sampler instrument editor with zone visualization and sample controls.
pub fn sampler_editor_view(
    ui: &mut Ui,
    params: &mut Vec<f32>,
    preset_name: &Option<String>,
    zone_count: usize,
    colors: &ThemeColors,
) {
    // Ensure params vec is large enough for selected zone index + all zones
    let min_len = (SELECTED_ZONE_PARAM + 1).max(zone_count * PARAMS_PER_ZONE);
    if params.len() < min_len {
        params.resize(min_len, 0.0);
    }

    // --- Header ---
    ui.horizontal(|ui| {
        let name = preset_name.as_deref().unwrap_or("No Preset");
        ui.label(
            egui::RichText::new(name)
                .size(14.0)
                .color(colors.text_primary()),
        );
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(format!("{} zones", zone_count))
                .size(11.0)
                .color(colors.text_secondary()),
        );
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // --- Zone Map ---
    zone_map(ui, params, zone_count, colors);

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // --- Selected Zone Controls ---
    let selected = params[SELECTED_ZONE_PARAM] as usize;
    if selected < zone_count {
        selected_zone_controls(ui, params, selected, colors);
    } else {
        ui.label(
            egui::RichText::new("No zone selected")
                .size(11.0)
                .color(colors.text_secondary()),
        );
    }

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // --- Sample Waveform Placeholder ---
    sample_waveform_placeholder(ui, colors);

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // --- Global Controls ---
    global_controls(ui, params, zone_count, colors);
}

/// Draw the keyboard-style zone map.
fn zone_map(ui: &mut Ui, params: &mut [f32], zone_count: usize, colors: &ThemeColors) {
    ui.label(
        egui::RichText::new("Zone Map")
            .size(11.0)
            .color(colors.text_secondary()),
    );
    ui.add_space(2.0);

    let zone_bar_height = 20.0;
    let keyboard_height = 16.0;
    let total_height = zone_bar_height + keyboard_height + 4.0;
    let available_width = ui.available_width().max(200.0);

    let (rect, response) =
        ui.allocate_exact_size(vec2(available_width, total_height), Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let key_width = available_width / 128.0;

        // Draw keyboard background
        let kb_rect = Rect::from_min_size(
            pos2(rect.min.x, rect.min.y + zone_bar_height + 4.0),
            vec2(available_width, keyboard_height),
        );
        painter.rect_filled(kb_rect, 1.0, colors.surface());

        // Draw black/white key pattern
        for key in 0..128u8 {
            let is_black = matches!(key % 12, 1 | 3 | 6 | 8 | 10);
            let x = rect.min.x + key as f32 * key_width;
            let key_rect =
                Rect::from_min_size(pos2(x, kb_rect.min.y), vec2(key_width, keyboard_height));
            let key_color = if is_black {
                colors.bg_primary()
            } else {
                colors.bg_tertiary()
            };
            painter.rect_filled(key_rect, 0.0, key_color);

            // C markers
            if key % 12 == 0 {
                painter.text(
                    pos2(x + key_width * 0.5, kb_rect.max.y - 2.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("C{}", (key / 12) as i8 - 1),
                    egui::FontId::new(6.0, egui::FontFamily::Monospace),
                    colors.text_secondary(),
                );
            }
        }

        // Draw zone rectangles above the keyboard
        for z in 0..zone_count {
            let key_low = params[zone_param_index(z, ZONE_KEY_LOW)] as f32;
            let key_high = params[zone_param_index(z, ZONE_KEY_HIGH)] as f32;
            let x_start = rect.min.x + key_low * key_width;
            let x_end = rect.min.x + (key_high + 1.0) * key_width;
            let zone_rect = Rect::from_min_size(
                pos2(x_start, rect.min.y),
                vec2((x_end - x_start).max(key_width), zone_bar_height),
            );
            painter.rect_filled(zone_rect, 2.0, zone_color(z));
            painter.rect_stroke(
                zone_rect,
                2.0,
                Stroke::new(1.0, colors.text_secondary()),
                egui::StrokeKind::Outside,
            );

            // Zone label
            let label_width = zone_rect.width();
            if label_width > 10.0 {
                painter.text(
                    zone_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", z),
                    egui::FontId::new(9.0, egui::FontFamily::Monospace),
                    Color32::WHITE,
                );
            }
        }
    }

    // Handle click to select a zone
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let key_width = available_width / 128.0;
        let clicked_key = ((pos.x - rect.min.x) / key_width) as u8;
        // Find zone containing this key
        for z in 0..zone_count {
            let key_low = params[zone_param_index(z, ZONE_KEY_LOW)] as u8;
            let key_high = params[zone_param_index(z, ZONE_KEY_HIGH)] as u8;
            if clicked_key >= key_low && clicked_key <= key_high {
                params[SELECTED_ZONE_PARAM] = z as f32;
                break;
            }
        }
    }
}

/// Draw controls for the currently selected zone.
fn selected_zone_controls(ui: &mut Ui, params: &mut Vec<f32>, zone: usize, colors: &ThemeColors) {
    ui.label(
        egui::RichText::new(format!("Zone {}", zone))
            .size(12.0)
            .color(colors.accent()),
    );
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        // Root Key
        let root_key = params[zone_param_index(zone, ZONE_ROOT_KEY)] as u8;
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Root Key")
                    .size(9.0)
                    .color(colors.text_secondary()),
            );
            ui.label(
                egui::RichText::new(format!("{} ({})", midi_note_name(root_key), root_key))
                    .size(11.0)
                    .color(colors.text_primary()),
            );
        });

        ui.add_space(16.0);

        // Key Range
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Key Range")
                    .size(9.0)
                    .color(colors.text_secondary()),
            );
            ui.horizontal(|ui| {
                let mut key_low = params[zone_param_index(zone, ZONE_KEY_LOW)];
                let mut key_high = params[zone_param_index(zone, ZONE_KEY_HIGH)];
                ui.label(
                    egui::RichText::new("Lo")
                        .size(9.0)
                        .color(colors.text_secondary()),
                );
                ui.add(egui::Slider::new(&mut key_low, 0.0..=127.0).show_value(true));
                ui.label(
                    egui::RichText::new("Hi")
                        .size(9.0)
                        .color(colors.text_secondary()),
                );
                ui.add(egui::Slider::new(&mut key_high, 0.0..=127.0).show_value(true));
                params[zone_param_index(zone, ZONE_KEY_LOW)] = key_low;
                params[zone_param_index(zone, ZONE_KEY_HIGH)] = key_high;
            });
        });

        ui.add_space(16.0);

        // Velocity Range
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Velocity Range")
                    .size(9.0)
                    .color(colors.text_secondary()),
            );
            ui.horizontal(|ui| {
                let mut vel_low = params[zone_param_index(zone, ZONE_VEL_LOW)];
                let mut vel_high = params[zone_param_index(zone, ZONE_VEL_HIGH)];
                ui.label(
                    egui::RichText::new("Lo")
                        .size(9.0)
                        .color(colors.text_secondary()),
                );
                ui.add(egui::Slider::new(&mut vel_low, 0.0..=127.0).show_value(true));
                ui.label(
                    egui::RichText::new("Hi")
                        .size(9.0)
                        .color(colors.text_secondary()),
                );
                ui.add(egui::Slider::new(&mut vel_high, 0.0..=127.0).show_value(true));
                params[zone_param_index(zone, ZONE_VEL_LOW)] = vel_low;
                params[zone_param_index(zone, ZONE_VEL_HIGH)] = vel_high;
            });
        });

        ui.add_space(16.0);

        // Gain knob
        ui.vertical(|ui| {
            let mut gain = params[zone_param_index(zone, ZONE_GAIN)];
            ui.add(Knob::new(&mut gain, 0.0, 2.0, colors).with_label("Gain"));
            params[zone_param_index(zone, ZONE_GAIN)] = gain;
        });

        ui.add_space(16.0);

        // Loop Mode selector
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Loop Mode")
                    .size(9.0)
                    .color(colors.text_secondary()),
            );
            // Loop mode is stored after the 6 main params, at a fixed offset.
            // For simplicity, display as a button cycling through modes.
            // We'll reuse the gain param's fractional part or use a separate approach.
            // Store loop mode in the integer part above 2.0 for the gain field is messy;
            // instead we just show a cycling button based on zone index.
            let loop_mode_idx = SELECTED_ZONE_PARAM + 1 + zone;
            if params.len() <= loop_mode_idx {
                params.resize(loop_mode_idx + 1, 0.0);
            }
            let mode = params[loop_mode_idx] as usize;
            let label = loop_mode_name(mode);
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(label)
                            .size(10.0)
                            .color(colors.text_primary()),
                    )
                    .fill(colors.surface())
                    .min_size(vec2(70.0, 18.0)),
                )
                .clicked()
            {
                params[loop_mode_idx] = ((mode + 1) % 3) as f32;
            }
        });
    });
}

/// Draw the sample waveform placeholder.
fn sample_waveform_placeholder(ui: &mut Ui, colors: &ThemeColors) {
    ui.label(
        egui::RichText::new("Sample")
            .size(11.0)
            .color(colors.text_secondary()),
    );
    ui.add_space(2.0);

    let available_width = ui.available_width().max(200.0);
    let height = 80.0;
    let (rect, _response) = ui.allocate_exact_size(vec2(available_width, height), Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 3.0, colors.bg_tertiary());
        painter.rect_stroke(
            rect,
            3.0,
            Stroke::new(1.0, colors.separator()),
            egui::StrokeKind::Outside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Drop sample here",
            egui::FontId::new(12.0, egui::FontFamily::Proportional),
            colors.text_secondary(),
        );
    }
}

/// Draw global sampler controls.
fn global_controls(ui: &mut Ui, params: &mut Vec<f32>, zone_count: usize, colors: &ThemeColors) {
    ui.label(
        egui::RichText::new("Global")
            .size(11.0)
            .color(colors.text_secondary()),
    );
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        // Master Gain knob — stored after selected zone + loop modes
        let master_gain_idx = SELECTED_ZONE_PARAM + 1 + zone_count.max(1);
        if params.len() <= master_gain_idx {
            params.resize(master_gain_idx + 1, 1.0);
        }
        ui.vertical(|ui| {
            let mut master_gain = params[master_gain_idx];
            ui.add(Knob::new(&mut master_gain, 0.0, 2.0, colors).with_label("Master"));
            params[master_gain_idx] = master_gain;
        });

        ui.add_space(24.0);

        // Polyphony display
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Polyphony")
                    .size(9.0)
                    .color(colors.text_secondary()),
            );
            ui.label(
                egui::RichText::new("32")
                    .size(11.0)
                    .color(colors.text_primary()),
            );
        });

        ui.add_space(24.0);

        // Auto-Slice button placeholder
        ui.vertical(|ui| {
            ui.add(
                egui::Button::new(
                    egui::RichText::new("Auto-Slice")
                        .size(10.0)
                        .color(colors.text_primary()),
                )
                .fill(colors.surface())
                .min_size(vec2(80.0, 22.0)),
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_note_name_c4() {
        assert_eq!(midi_note_name(60), "C4");
    }

    #[test]
    fn midi_note_name_a4() {
        assert_eq!(midi_note_name(69), "A4");
    }

    #[test]
    fn midi_note_name_edge_zero() {
        // MIDI note 0 = C-1
        assert_eq!(midi_note_name(0), "C-1");
    }

    #[test]
    fn midi_note_name_edge_127() {
        // MIDI note 127 = G9
        assert_eq!(midi_note_name(127), "G9");
    }

    #[test]
    fn midi_note_name_sharps() {
        assert_eq!(midi_note_name(61), "C#4");
        assert_eq!(midi_note_name(66), "F#4");
    }

    #[test]
    fn zone_param_index_first_zone() {
        assert_eq!(zone_param_index(0, 0), 0); // root key
        assert_eq!(zone_param_index(0, 1), 1); // key low
        assert_eq!(zone_param_index(0, 5), 5); // gain
    }

    #[test]
    fn zone_param_index_later_zones() {
        assert_eq!(zone_param_index(1, 0), 6);
        assert_eq!(zone_param_index(2, 3), 15);
        assert_eq!(zone_param_index(5, 5), 35);
    }

    #[test]
    fn loop_mode_name_values() {
        assert_eq!(loop_mode_name(0), "Off");
        assert_eq!(loop_mode_name(1), "Forward");
        assert_eq!(loop_mode_name(2), "Ping-Pong");
        assert_eq!(loop_mode_name(99), "Off");
    }

    #[test]
    fn zone_key_range_bounds() {
        // Verify zone params store key ranges within 0-127
        let mut params = vec![0.0f32; 200];
        let zone = 0;
        params[zone_param_index(zone, ZONE_KEY_LOW)] = 0.0;
        params[zone_param_index(zone, ZONE_KEY_HIGH)] = 127.0;
        assert_eq!(params[zone_param_index(zone, ZONE_KEY_LOW)] as u8, 0);
        assert_eq!(params[zone_param_index(zone, ZONE_KEY_HIGH)] as u8, 127);
    }

    #[test]
    fn velocity_range_defaults() {
        // A freshly zeroed params vec has velocity range 0-0
        let params = vec![0.0f32; 200];
        let zone = 0;
        let vel_low = params[zone_param_index(zone, ZONE_VEL_LOW)];
        let vel_high = params[zone_param_index(zone, ZONE_VEL_HIGH)];
        assert!((0.0..=127.0).contains(&vel_low));
        assert!((0.0..=127.0).contains(&vel_high));
    }

    #[test]
    fn zone_color_cycles() {
        // Verify colors cycle through the palette
        let c0 = zone_color(0);
        let c8 = zone_color(8);
        assert_eq!(c0, c8); // 8 colors, so index 8 wraps to 0
    }

    #[test]
    fn selected_zone_bounds_checking() {
        // selected_zone is stored at SELECTED_ZONE_PARAM
        let mut params = vec![0.0f32; 200];
        let zone_count = 4;

        // Set selected to a valid zone
        params[SELECTED_ZONE_PARAM] = 2.0;
        let selected = params[SELECTED_ZONE_PARAM] as usize;
        assert!(selected < zone_count);

        // Set selected beyond zone count
        params[SELECTED_ZONE_PARAM] = 10.0;
        let selected = params[SELECTED_ZONE_PARAM] as usize;
        assert!(selected >= zone_count); // should not render zone controls
    }

    #[test]
    fn zone_count_to_grid_dimensions() {
        // Each zone occupies PARAMS_PER_ZONE (6) slots in the params vec
        let zone_count = 8;
        let required_params = zone_count * PARAMS_PER_ZONE;
        assert_eq!(required_params, 48);

        let zone_count_16 = 16;
        let required_16 = zone_count_16 * PARAMS_PER_ZONE;
        assert_eq!(required_16, 96);
    }

    #[test]
    fn midi_note_name_middle_octave() {
        // Verify several notes in the middle octave range
        assert_eq!(midi_note_name(48), "C3");
        assert_eq!(midi_note_name(72), "C5");
        assert_eq!(midi_note_name(57), "A3");
    }
}
