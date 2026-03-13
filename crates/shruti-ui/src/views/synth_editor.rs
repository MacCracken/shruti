use egui::Ui;

use crate::theme::ThemeColors;
use crate::widgets::knob::Knob;

// --- Parameter indices (mirroring shruti-instruments/src/synth.rs) ---

const PARAM_WAVEFORM: usize = 0;
const PARAM_ATTACK: usize = 1;
const PARAM_DECAY: usize = 2;
const PARAM_SUSTAIN: usize = 3;
const PARAM_RELEASE: usize = 4;
const PARAM_VOLUME: usize = 5;
const PARAM_DETUNE: usize = 6;
const PARAM_FILTER_CUTOFF: usize = 7;
const PARAM_FILTER_RESONANCE: usize = 8;
const PARAM_FILTER_MODE: usize = 9;
const PARAM_FILTER_ENV_ATTACK: usize = 10;
const PARAM_FILTER_ENV_DECAY: usize = 11;
const PARAM_FILTER_ENV_SUSTAIN: usize = 12;
const PARAM_FILTER_ENV_RELEASE: usize = 13;
const PARAM_FILTER_ENV_DEPTH: usize = 14;
const PARAM_LFO1_RATE: usize = 15;
const PARAM_LFO1_DEPTH: usize = 16;
const PARAM_LFO1_TARGET: usize = 17;
const PARAM_LFO1_SHAPE: usize = 18;
const PARAM_LFO2_RATE: usize = 19;
const PARAM_LFO2_DEPTH: usize = 20;
const PARAM_LFO2_TARGET: usize = 21;
const PARAM_LFO2_SHAPE: usize = 22;
const PARAM_OSC2_ENABLE: usize = 23;
const PARAM_OSC2_WAVEFORM: usize = 24;
const PARAM_OSC2_DETUNE: usize = 25;
const PARAM_OSC2_LEVEL: usize = 26;
const PARAM_OSC3_ENABLE: usize = 27;
const PARAM_OSC3_WAVEFORM: usize = 28;
const PARAM_OSC3_DETUNE: usize = 29;
const PARAM_OSC3_LEVEL: usize = 30;
const PARAM_HARD_SYNC: usize = 31;
const PARAM_RING_MOD: usize = 32;
const PARAM_FM_AMOUNT: usize = 33;

/// Minimum number of parameters the synth editor expects.
const MIN_PARAM_COUNT: usize = 34;

// --- Section labels ---

const SECTION_OSCILLATORS: &str = "Oscillators";
const SECTION_AMP_ENVELOPE: &str = "Amplitude Envelope";
const SECTION_FILTER: &str = "Filter";
const SECTION_FILTER_ENVELOPE: &str = "Filter Envelope";
const SECTION_LFO1: &str = "LFO 1";
const SECTION_LFO2: &str = "LFO 2";

// --- Selector name tables ---

const WAVEFORM_NAMES: &[&str] = &["Sine", "Saw", "Square", "Triangle", "Noise"];
const FILTER_MODE_NAMES: &[&str] = &["LowPass", "HighPass", "BandPass", "Notch"];
const LFO_TARGET_NAMES: &[&str] = &["None", "Cutoff", "Pitch", "Volume"];
const LFO_SHAPE_NAMES: &[&str] = &["Sine", "Triangle", "Square", "SawUp", "SawDown", "S&H"];

// --- Helpers ---

/// Convert a float parameter value to a selector index, clamped to valid range.
fn float_to_index(value: f32, count: usize) -> usize {
    let idx = value.round() as i32;
    idx.clamp(0, (count as i32) - 1) as usize
}

/// Convert a selector index back to the float parameter value.
fn index_to_float(index: usize) -> f32 {
    index as f32
}

/// Look up a name from a table by float parameter value.
#[cfg(test)]
fn name_from_float<'a>(value: f32, names: &'a [&'a str]) -> &'a str {
    let idx = float_to_index(value, names.len());
    names[idx]
}

/// Ensure the params vec has at least `MIN_PARAM_COUNT` entries, padding with defaults.
fn ensure_param_count(params: &mut Vec<f32>) {
    if params.len() < MIN_PARAM_COUNT {
        params.resize(MIN_PARAM_COUNT, 0.0);
    }
    // Set sensible non-zero defaults for newly padded entries only when they are
    // exactly zero and likely uninitialised. We avoid overwriting intentional zeros
    // by only defaulting the sustain and volume params (users rarely want those at 0).
}

/// Draw a ComboBox selector that maps a float param to a list of named options.
fn selector(ui: &mut Ui, id: &str, label: &str, param: &mut f32, names: &[&str]) {
    let mut idx = float_to_index(*param, names.len());
    let current = names[idx];
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(10.0)
                .color(egui::Color32::from_rgb(140, 140, 150)),
        );
        egui::ComboBox::from_id_salt(id)
            .selected_text(current)
            .width(90.0)
            .show_ui(ui, |ui| {
                for (i, name) in names.iter().enumerate() {
                    if ui.selectable_label(i == idx, *name).clicked() {
                        idx = i;
                    }
                }
            });
    });
    *param = index_to_float(idx);
}

/// Draw a toggle (checkbox-style) for a boolean float param (0.0 = off, 1.0 = on).
fn toggle(ui: &mut Ui, label: &str, param: &mut f32) {
    let mut on = *param >= 0.5;
    ui.checkbox(&mut on, label);
    *param = if on { 1.0 } else { 0.0 };
}

// --- Main view ---

/// Render the subtractive synth parameter editor.
///
/// `params` is the instrument parameter vector from the track. The caller is
/// responsible for extracting it (e.g. `&mut track.instrument_params`).
pub fn synth_editor_view(ui: &mut Ui, params: &mut Vec<f32>, colors: &ThemeColors) {
    ensure_param_count(params);

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;

            // --- Oscillators ---
            ui.collapsing(SECTION_OSCILLATORS, |ui| {
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("Osc 1")
                            .size(11.0)
                            .color(colors.text_primary()),
                    );
                    ui.horizontal(|ui| {
                        selector(
                            ui,
                            "osc1_waveform",
                            "Waveform",
                            &mut params[PARAM_WAVEFORM],
                            WAVEFORM_NAMES,
                        );
                        let mut detune = params[PARAM_DETUNE];
                        ui.add(Knob::new(&mut detune, -1.0, 1.0, colors).with_label("Detune"));
                        params[PARAM_DETUNE] = detune;
                    });
                });

                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("Osc 2")
                            .size(11.0)
                            .color(colors.text_primary()),
                    );
                    toggle(ui, "Enable", &mut params[PARAM_OSC2_ENABLE]);
                    ui.horizontal(|ui| {
                        selector(
                            ui,
                            "osc2_waveform",
                            "Waveform",
                            &mut params[PARAM_OSC2_WAVEFORM],
                            WAVEFORM_NAMES,
                        );
                        let mut detune = params[PARAM_OSC2_DETUNE];
                        ui.add(Knob::new(&mut detune, -1.0, 1.0, colors).with_label("Detune"));
                        params[PARAM_OSC2_DETUNE] = detune;

                        let mut level = params[PARAM_OSC2_LEVEL];
                        ui.add(Knob::new(&mut level, 0.0, 1.0, colors).with_label("Level"));
                        params[PARAM_OSC2_LEVEL] = level;
                    });
                });

                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("Osc 3")
                            .size(11.0)
                            .color(colors.text_primary()),
                    );
                    toggle(ui, "Enable", &mut params[PARAM_OSC3_ENABLE]);
                    ui.horizontal(|ui| {
                        selector(
                            ui,
                            "osc3_waveform",
                            "Waveform",
                            &mut params[PARAM_OSC3_WAVEFORM],
                            WAVEFORM_NAMES,
                        );
                        let mut detune = params[PARAM_OSC3_DETUNE];
                        ui.add(Knob::new(&mut detune, -1.0, 1.0, colors).with_label("Detune"));
                        params[PARAM_OSC3_DETUNE] = detune;

                        let mut level = params[PARAM_OSC3_LEVEL];
                        ui.add(Knob::new(&mut level, 0.0, 1.0, colors).with_label("Level"));
                        params[PARAM_OSC3_LEVEL] = level;
                    });
                });

                ui.add_space(4.0);

                // Inter-oscillator modulation
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("Modulation")
                            .size(11.0)
                            .color(colors.text_primary()),
                    );
                    ui.horizontal(|ui| {
                        toggle(ui, "Hard Sync", &mut params[PARAM_HARD_SYNC]);

                        let mut ring = params[PARAM_RING_MOD];
                        ui.add(Knob::new(&mut ring, 0.0, 1.0, colors).with_label("Ring Mod"));
                        params[PARAM_RING_MOD] = ring;

                        let mut fm = params[PARAM_FM_AMOUNT];
                        ui.add(Knob::new(&mut fm, 0.0, 1.0, colors).with_label("FM"));
                        params[PARAM_FM_AMOUNT] = fm;
                    });
                });
            });

            // --- Amplitude Envelope ---
            ui.collapsing(SECTION_AMP_ENVELOPE, |ui| {
                ui.horizontal(|ui| {
                    let mut attack = params[PARAM_ATTACK];
                    ui.add(Knob::new(&mut attack, 0.0, 5.0, colors).with_label("Attack"));
                    params[PARAM_ATTACK] = attack;

                    let mut decay = params[PARAM_DECAY];
                    ui.add(Knob::new(&mut decay, 0.0, 5.0, colors).with_label("Decay"));
                    params[PARAM_DECAY] = decay;

                    let mut sustain = params[PARAM_SUSTAIN];
                    ui.add(Knob::new(&mut sustain, 0.0, 1.0, colors).with_label("Sustain"));
                    params[PARAM_SUSTAIN] = sustain;

                    let mut release = params[PARAM_RELEASE];
                    ui.add(Knob::new(&mut release, 0.0, 10.0, colors).with_label("Release"));
                    params[PARAM_RELEASE] = release;

                    let mut volume = params[PARAM_VOLUME];
                    ui.add(Knob::new(&mut volume, 0.0, 1.0, colors).with_label("Volume"));
                    params[PARAM_VOLUME] = volume;
                });
            });

            // --- Filter ---
            ui.collapsing(SECTION_FILTER, |ui| {
                ui.horizontal(|ui| {
                    let mut cutoff = params[PARAM_FILTER_CUTOFF];
                    ui.add(Knob::new(&mut cutoff, 20.0, 20000.0, colors).with_label("Cutoff"));
                    params[PARAM_FILTER_CUTOFF] = cutoff;

                    let mut resonance = params[PARAM_FILTER_RESONANCE];
                    ui.add(Knob::new(&mut resonance, 0.0, 1.0, colors).with_label("Resonance"));
                    params[PARAM_FILTER_RESONANCE] = resonance;
                });

                selector(
                    ui,
                    "filter_mode",
                    "Mode",
                    &mut params[PARAM_FILTER_MODE],
                    FILTER_MODE_NAMES,
                );
            });

            // --- Filter Envelope ---
            ui.collapsing(SECTION_FILTER_ENVELOPE, |ui| {
                ui.horizontal(|ui| {
                    let mut attack = params[PARAM_FILTER_ENV_ATTACK];
                    ui.add(Knob::new(&mut attack, 0.0, 5.0, colors).with_label("Attack"));
                    params[PARAM_FILTER_ENV_ATTACK] = attack;

                    let mut decay = params[PARAM_FILTER_ENV_DECAY];
                    ui.add(Knob::new(&mut decay, 0.0, 5.0, colors).with_label("Decay"));
                    params[PARAM_FILTER_ENV_DECAY] = decay;

                    let mut sustain = params[PARAM_FILTER_ENV_SUSTAIN];
                    ui.add(Knob::new(&mut sustain, 0.0, 1.0, colors).with_label("Sustain"));
                    params[PARAM_FILTER_ENV_SUSTAIN] = sustain;

                    let mut release = params[PARAM_FILTER_ENV_RELEASE];
                    ui.add(Knob::new(&mut release, 0.0, 10.0, colors).with_label("Release"));
                    params[PARAM_FILTER_ENV_RELEASE] = release;

                    let mut depth = params[PARAM_FILTER_ENV_DEPTH];
                    ui.add(Knob::new(&mut depth, -1.0, 1.0, colors).with_label("Depth"));
                    params[PARAM_FILTER_ENV_DEPTH] = depth;
                });
            });

            // --- LFO 1 ---
            ui.collapsing(SECTION_LFO1, |ui| {
                ui.horizontal(|ui| {
                    let mut rate = params[PARAM_LFO1_RATE];
                    ui.add(Knob::new(&mut rate, 0.01, 50.0, colors).with_label("Rate"));
                    params[PARAM_LFO1_RATE] = rate;

                    let mut depth = params[PARAM_LFO1_DEPTH];
                    ui.add(Knob::new(&mut depth, 0.0, 1.0, colors).with_label("Depth"));
                    params[PARAM_LFO1_DEPTH] = depth;
                });

                selector(
                    ui,
                    "lfo1_target",
                    "Target",
                    &mut params[PARAM_LFO1_TARGET],
                    LFO_TARGET_NAMES,
                );
                selector(
                    ui,
                    "lfo1_shape",
                    "Shape",
                    &mut params[PARAM_LFO1_SHAPE],
                    LFO_SHAPE_NAMES,
                );
            });

            // --- LFO 2 ---
            ui.collapsing(SECTION_LFO2, |ui| {
                ui.horizontal(|ui| {
                    let mut rate = params[PARAM_LFO2_RATE];
                    ui.add(Knob::new(&mut rate, 0.01, 50.0, colors).with_label("Rate"));
                    params[PARAM_LFO2_RATE] = rate;

                    let mut depth = params[PARAM_LFO2_DEPTH];
                    ui.add(Knob::new(&mut depth, 0.0, 1.0, colors).with_label("Depth"));
                    params[PARAM_LFO2_DEPTH] = depth;
                });

                selector(
                    ui,
                    "lfo2_target",
                    "Target",
                    &mut params[PARAM_LFO2_TARGET],
                    LFO_TARGET_NAMES,
                );
                selector(
                    ui,
                    "lfo2_shape",
                    "Shape",
                    &mut params[PARAM_LFO2_SHAPE],
                    LFO_SHAPE_NAMES,
                );
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Waveform name mapping ---

    #[test]
    fn waveform_names_all_present() {
        assert_eq!(WAVEFORM_NAMES.len(), 5);
        assert_eq!(name_from_float(0.0, WAVEFORM_NAMES), "Sine");
        assert_eq!(name_from_float(1.0, WAVEFORM_NAMES), "Saw");
        assert_eq!(name_from_float(2.0, WAVEFORM_NAMES), "Square");
        assert_eq!(name_from_float(3.0, WAVEFORM_NAMES), "Triangle");
        assert_eq!(name_from_float(4.0, WAVEFORM_NAMES), "Noise");
    }

    // --- Filter mode name mapping ---

    #[test]
    fn filter_mode_names_all_present() {
        assert_eq!(FILTER_MODE_NAMES.len(), 4);
        assert_eq!(name_from_float(0.0, FILTER_MODE_NAMES), "LowPass");
        assert_eq!(name_from_float(1.0, FILTER_MODE_NAMES), "HighPass");
        assert_eq!(name_from_float(2.0, FILTER_MODE_NAMES), "BandPass");
        assert_eq!(name_from_float(3.0, FILTER_MODE_NAMES), "Notch");
    }

    // --- LFO target name mapping ---

    #[test]
    fn lfo_target_names_all_present() {
        assert_eq!(LFO_TARGET_NAMES.len(), 4);
        assert_eq!(name_from_float(0.0, LFO_TARGET_NAMES), "None");
        assert_eq!(name_from_float(1.0, LFO_TARGET_NAMES), "Cutoff");
        assert_eq!(name_from_float(2.0, LFO_TARGET_NAMES), "Pitch");
        assert_eq!(name_from_float(3.0, LFO_TARGET_NAMES), "Volume");
    }

    // --- LFO shape name mapping ---

    #[test]
    fn lfo_shape_names_all_present() {
        assert_eq!(LFO_SHAPE_NAMES.len(), 6);
        assert_eq!(name_from_float(0.0, LFO_SHAPE_NAMES), "Sine");
        assert_eq!(name_from_float(1.0, LFO_SHAPE_NAMES), "Triangle");
        assert_eq!(name_from_float(2.0, LFO_SHAPE_NAMES), "Square");
        assert_eq!(name_from_float(3.0, LFO_SHAPE_NAMES), "SawUp");
        assert_eq!(name_from_float(4.0, LFO_SHAPE_NAMES), "SawDown");
        assert_eq!(name_from_float(5.0, LFO_SHAPE_NAMES), "S&H");
    }

    // --- Param ensure size ---

    #[test]
    fn ensure_param_count_pads_short_vec() {
        let mut params = vec![0.5; 5];
        ensure_param_count(&mut params);
        assert_eq!(params.len(), MIN_PARAM_COUNT);
        // Original values preserved
        for param in params.iter().take(5) {
            assert!((param - 0.5).abs() < f32::EPSILON);
        }
        // Padded values are 0.0
        for param in params.iter().take(MIN_PARAM_COUNT).skip(5) {
            assert!((param - 0.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn ensure_param_count_noop_when_large_enough() {
        let mut params = vec![1.0; 40];
        ensure_param_count(&mut params);
        assert_eq!(params.len(), 40);
        for &v in &params {
            assert!((v - 1.0).abs() < f32::EPSILON);
        }
    }

    // --- Default param values ---

    #[test]
    fn ensure_param_count_empty_vec() {
        let mut params: Vec<f32> = Vec::new();
        ensure_param_count(&mut params);
        assert_eq!(params.len(), MIN_PARAM_COUNT);
        // All defaults should be 0.0
        for &v in &params {
            assert!((v - 0.0).abs() < f32::EPSILON);
        }
    }

    // --- Float-to-index conversion ---

    #[test]
    fn float_to_index_rounds_correctly() {
        assert_eq!(float_to_index(0.0, 5), 0);
        assert_eq!(float_to_index(0.4, 5), 0);
        assert_eq!(float_to_index(0.5, 5), 1); // f32::round uses "round half away from zero"
        assert_eq!(float_to_index(0.6, 5), 1);
        assert_eq!(float_to_index(2.3, 5), 2);
        assert_eq!(float_to_index(4.0, 5), 4);
    }

    #[test]
    fn float_to_index_clamps_out_of_range() {
        assert_eq!(float_to_index(-1.0, 5), 0);
        assert_eq!(float_to_index(10.0, 5), 4);
        assert_eq!(float_to_index(100.0, 4), 3);
    }

    // --- Index-to-float conversion ---

    #[test]
    fn index_to_float_converts() {
        assert!((index_to_float(0) - 0.0).abs() < f32::EPSILON);
        assert!((index_to_float(3) - 3.0).abs() < f32::EPSILON);
        assert!((index_to_float(5) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn float_index_roundtrip() {
        for i in 0..6 {
            let f = index_to_float(i);
            let back = float_to_index(f, 6);
            assert_eq!(back, i, "roundtrip failed for index {i}");
        }
    }

    // --- Section label constants ---

    #[test]
    fn section_labels_are_non_empty() {
        assert!(!SECTION_OSCILLATORS.is_empty());
        assert!(!SECTION_AMP_ENVELOPE.is_empty());
        assert!(!SECTION_FILTER.is_empty());
        assert!(!SECTION_FILTER_ENVELOPE.is_empty());
        assert!(!SECTION_LFO1.is_empty());
        assert!(!SECTION_LFO2.is_empty());
    }

    #[test]
    fn section_labels_are_distinct() {
        let labels = [
            SECTION_OSCILLATORS,
            SECTION_AMP_ENVELOPE,
            SECTION_FILTER,
            SECTION_FILTER_ENVELOPE,
            SECTION_LFO1,
            SECTION_LFO2,
        ];
        for (i, a) in labels.iter().enumerate() {
            for (j, b) in labels.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "labels at {i} and {j} should be distinct");
                }
            }
        }
    }

    // --- Parameter index constants ---

    #[test]
    fn param_indices_are_sequential_and_complete() {
        assert_eq!(PARAM_WAVEFORM, 0);
        assert_eq!(PARAM_ATTACK, 1);
        assert_eq!(PARAM_DECAY, 2);
        assert_eq!(PARAM_SUSTAIN, 3);
        assert_eq!(PARAM_RELEASE, 4);
        assert_eq!(PARAM_VOLUME, 5);
        assert_eq!(PARAM_DETUNE, 6);
        assert_eq!(PARAM_FILTER_CUTOFF, 7);
        assert_eq!(PARAM_FILTER_RESONANCE, 8);
        assert_eq!(PARAM_FILTER_MODE, 9);
        assert_eq!(PARAM_FILTER_ENV_ATTACK, 10);
        assert_eq!(PARAM_FILTER_ENV_DECAY, 11);
        assert_eq!(PARAM_FILTER_ENV_SUSTAIN, 12);
        assert_eq!(PARAM_FILTER_ENV_RELEASE, 13);
        assert_eq!(PARAM_FILTER_ENV_DEPTH, 14);
        assert_eq!(PARAM_LFO1_RATE, 15);
        assert_eq!(PARAM_LFO1_DEPTH, 16);
        assert_eq!(PARAM_LFO1_TARGET, 17);
        assert_eq!(PARAM_LFO1_SHAPE, 18);
        assert_eq!(PARAM_LFO2_RATE, 19);
        assert_eq!(PARAM_LFO2_DEPTH, 20);
        assert_eq!(PARAM_LFO2_TARGET, 21);
        assert_eq!(PARAM_LFO2_SHAPE, 22);
        assert_eq!(PARAM_OSC2_ENABLE, 23);
        assert_eq!(PARAM_OSC2_WAVEFORM, 24);
        assert_eq!(PARAM_OSC2_DETUNE, 25);
        assert_eq!(PARAM_OSC2_LEVEL, 26);
        assert_eq!(PARAM_OSC3_ENABLE, 27);
        assert_eq!(PARAM_OSC3_WAVEFORM, 28);
        assert_eq!(PARAM_OSC3_DETUNE, 29);
        assert_eq!(PARAM_OSC3_LEVEL, 30);
        assert_eq!(PARAM_HARD_SYNC, 31);
        assert_eq!(PARAM_RING_MOD, 32);
        assert_eq!(PARAM_FM_AMOUNT, 33);
        assert_eq!(MIN_PARAM_COUNT, 34);
    }

    // --- name_from_float edge cases ---

    #[test]
    fn name_from_float_clamps_negative() {
        assert_eq!(name_from_float(-5.0, WAVEFORM_NAMES), "Sine");
    }

    #[test]
    fn name_from_float_clamps_beyond_max() {
        assert_eq!(name_from_float(99.0, WAVEFORM_NAMES), "Noise");
    }
}
