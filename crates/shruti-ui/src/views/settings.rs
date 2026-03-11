use egui::{ScrollArea, Ui};

use crate::state::UiState;
use crate::theme::ThemeColors;

use shruti_engine::backend::{AudioHost, CpalBackend, DeviceInfo};
use shruti_engine::midi_io::{MidiPortInfo, enumerate_midi_ports};

/// Cached device enumeration to avoid scanning every frame.
pub struct DeviceCache {
    pub audio_devices: Vec<DeviceInfo>,
    pub midi_ports: Vec<MidiPortInfo>,
    pub needs_refresh: bool,
}

impl Default for DeviceCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceCache {
    pub fn new() -> Self {
        Self {
            audio_devices: Vec::new(),
            midi_ports: Vec::new(),
            needs_refresh: true,
        }
    }

    pub fn refresh(&mut self) {
        let backend = CpalBackend::new();
        self.audio_devices = backend.all_devices();
        self.midi_ports = enumerate_midi_ports();
        self.needs_refresh = false;
    }
}

/// Draw the settings/devices panel.
pub fn settings_view(
    ui: &mut Ui,
    state: &mut UiState,
    colors: &ThemeColors,
    device_cache: &mut DeviceCache,
) {
    if device_cache.needs_refresh {
        device_cache.refresh();
    }

    ui.heading(
        egui::RichText::new("Audio & MIDI Settings")
            .size(14.0)
            .color(colors.text_primary()),
    );
    ui.add_space(8.0);

    // Rescan button
    if ui.button("Rescan Devices").clicked() {
        device_cache.needs_refresh = true;
    }

    ui.add_space(12.0);

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            // --- Audio Interfaces ---
            ui.label(
                egui::RichText::new("Audio Interfaces")
                    .size(12.0)
                    .strong()
                    .color(colors.text_primary()),
            );
            ui.separator();
            ui.add_space(4.0);

            if device_cache.audio_devices.is_empty() {
                ui.label(
                    egui::RichText::new("No audio devices detected.")
                        .size(10.0)
                        .color(colors.text_secondary()),
                );
            } else {
                for device in &device_cache.audio_devices {
                    ui.horizontal(|ui| {
                        // Default indicator
                        if device.is_default {
                            ui.label(
                                egui::RichText::new("\u{2605}")
                                    .size(10.0)
                                    .color(colors.accent()),
                            );
                        } else {
                            ui.label(egui::RichText::new("  ").size(10.0));
                        }

                        // Direction badge
                        let direction = match (device.is_input, device.is_output) {
                            (true, true) => "I/O",
                            (true, false) => "IN",
                            (false, true) => "OUT",
                            _ => "?",
                        };
                        ui.label(
                            egui::RichText::new(direction)
                                .monospace()
                                .size(9.0)
                                .color(colors.accent()),
                        );

                        // Device name
                        ui.label(
                            egui::RichText::new(&device.name)
                                .size(10.0)
                                .color(colors.text_primary()),
                        );

                        // Channel count
                        ui.label(
                            egui::RichText::new(format!("{}ch", device.max_channels))
                                .monospace()
                                .size(9.0)
                                .color(colors.text_secondary()),
                        );

                        // Sample rates
                        if !device.supported_sample_rates.is_empty() {
                            let rates: Vec<String> = device
                                .supported_sample_rates
                                .iter()
                                .map(|r| format!("{}k", r / 1000))
                                .collect();
                            ui.label(
                                egui::RichText::new(rates.join("/"))
                                    .monospace()
                                    .size(8.0)
                                    .color(colors.text_secondary()),
                            );
                        }
                    });
                }
            }

            ui.add_space(16.0);

            // --- MIDI Devices ---
            ui.label(
                egui::RichText::new("MIDI Devices")
                    .size(12.0)
                    .strong()
                    .color(colors.text_primary()),
            );
            ui.separator();
            ui.add_space(4.0);

            let midi_inputs: Vec<&MidiPortInfo> = device_cache
                .midi_ports
                .iter()
                .filter(|p| p.is_input)
                .collect();
            let midi_outputs: Vec<&MidiPortInfo> = device_cache
                .midi_ports
                .iter()
                .filter(|p| p.is_output)
                .collect();

            if midi_inputs.is_empty() && midi_outputs.is_empty() {
                ui.label(
                    egui::RichText::new("No MIDI devices detected.")
                        .size(10.0)
                        .color(colors.text_secondary()),
                );
            } else {
                if !midi_inputs.is_empty() {
                    ui.label(
                        egui::RichText::new("Inputs:")
                            .size(10.0)
                            .color(colors.text_secondary()),
                    );
                    for port in &midi_inputs {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("  MIDI IN")
                                    .monospace()
                                    .size(9.0)
                                    .color(colors.accent()),
                            );
                            ui.label(
                                egui::RichText::new(&port.name)
                                    .size(10.0)
                                    .color(colors.text_primary()),
                            );
                        });
                    }
                }

                if !midi_outputs.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Outputs:")
                            .size(10.0)
                            .color(colors.text_secondary()),
                    );
                    for port in &midi_outputs {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("  MIDI OUT")
                                    .monospace()
                                    .size(9.0)
                                    .color(colors.accent()),
                            );
                            ui.label(
                                egui::RichText::new(&port.name)
                                    .size(10.0)
                                    .color(colors.text_primary()),
                            );
                        });
                    }
                }
            }

            ui.add_space(16.0);

            // --- Current Preferences ---
            ui.label(
                egui::RichText::new("Current Configuration")
                    .size(12.0)
                    .strong()
                    .color(colors.text_primary()),
            );
            ui.separator();
            ui.add_space(4.0);

            let prefs_device = state
                .session
                .audio_device_name
                .as_deref()
                .unwrap_or("System Default");
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Device:")
                        .size(10.0)
                        .color(colors.text_secondary()),
                );
                ui.label(
                    egui::RichText::new(prefs_device)
                        .size(10.0)
                        .color(colors.text_primary()),
                );
            });
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Sample Rate:")
                        .size(10.0)
                        .color(colors.text_secondary()),
                );
                ui.label(
                    egui::RichText::new(format!("{} Hz", state.session.sample_rate))
                        .size(10.0)
                        .color(colors.text_primary()),
                );
            });
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Buffer Size:")
                        .size(10.0)
                        .color(colors.text_secondary()),
                );
                ui.label(
                    egui::RichText::new(format!("{} frames", state.session.buffer_size))
                        .size(10.0)
                        .color(colors.text_primary()),
                );
            });
        });
}
