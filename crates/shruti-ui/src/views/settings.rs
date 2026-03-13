use egui::{ScrollArea, Ui};

use crate::state::UiState;
use crate::theme::ThemeColors;

use shruti_engine::backend::{AudioHost, CpalBackend, DeviceInfo};
use shruti_engine::midi_io::{MidiPortInfo, enumerate_midi_ports};

/// Cached device enumeration to avoid scanning every frame.
///
/// Uses diff-based refresh: on each scan, new and removed devices are
/// detected by name rather than rebuilding the entire list from scratch.
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

    /// Diff-based refresh: compares the current device list against the
    /// previously cached list and only adds/removes changed entries.
    /// Existing entries that are still present are updated in-place so
    /// any UI state that references them by index stays valid.
    pub fn refresh(&mut self) {
        let backend = CpalBackend::new();
        let fresh_audio = backend.all_devices();
        let fresh_midi = enumerate_midi_ports();

        diff_device_list(&mut self.audio_devices, fresh_audio);
        diff_midi_list(&mut self.midi_ports, fresh_midi);

        self.needs_refresh = false;
    }
}

/// Diff audio device lists: remove stale devices, update existing ones,
/// and append new ones.
fn diff_device_list(cached: &mut Vec<DeviceInfo>, fresh: Vec<DeviceInfo>) {
    // Remove devices that no longer appear in the fresh list.
    cached.retain(|d| fresh.iter().any(|f| f.name == d.name));

    for new_dev in fresh {
        if let Some(existing) = cached.iter_mut().find(|d| d.name == new_dev.name) {
            // Update fields that may have changed.
            existing.is_default = new_dev.is_default;
            existing.is_input = new_dev.is_input;
            existing.is_output = new_dev.is_output;
            existing.max_channels = new_dev.max_channels;
            existing.supported_sample_rates = new_dev.supported_sample_rates;
        } else {
            // Brand new device.
            cached.push(new_dev);
        }
    }
}

/// Diff MIDI port lists by name and direction.
fn diff_midi_list(cached: &mut Vec<MidiPortInfo>, fresh: Vec<MidiPortInfo>) {
    cached.retain(|p| {
        fresh
            .iter()
            .any(|f| f.name == p.name && f.is_input == p.is_input)
    });

    for new_port in fresh {
        let already = cached
            .iter()
            .any(|p| p.name == new_port.name && p.is_input == new_port.is_input);
        if !already {
            cached.push(new_port);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_device(name: &str, is_default: bool) -> DeviceInfo {
        DeviceInfo {
            name: name.to_string(),
            is_default,
            is_input: false,
            is_output: true,
            max_channels: 2,
            supported_sample_rates: vec![48000],
        }
    }

    fn make_midi(name: &str, is_input: bool) -> MidiPortInfo {
        MidiPortInfo {
            name: name.to_string(),
            is_input,
            is_output: !is_input,
        }
    }

    #[test]
    fn test_diff_device_list_no_change() {
        let mut cached = vec![make_device("A", true), make_device("B", false)];
        let fresh = vec![make_device("A", true), make_device("B", false)];
        diff_device_list(&mut cached, fresh);
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].name, "A");
        assert_eq!(cached[1].name, "B");
    }

    #[test]
    fn test_diff_device_list_adds_new() {
        let mut cached = vec![make_device("A", true)];
        let fresh = vec![make_device("A", true), make_device("C", false)];
        diff_device_list(&mut cached, fresh);
        assert_eq!(cached.len(), 2);
        assert!(cached.iter().any(|d| d.name == "C"));
    }

    #[test]
    fn test_diff_device_list_removes_stale() {
        let mut cached = vec![make_device("A", true), make_device("B", false)];
        let fresh = vec![make_device("A", true)];
        diff_device_list(&mut cached, fresh);
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].name, "A");
    }

    #[test]
    fn test_diff_device_list_updates_fields() {
        let mut cached = vec![make_device("A", false)];
        let mut fresh_dev = make_device("A", true);
        fresh_dev.max_channels = 8;
        diff_device_list(&mut cached, vec![fresh_dev]);
        assert_eq!(cached.len(), 1);
        assert!(cached[0].is_default);
        assert_eq!(cached[0].max_channels, 8);
    }

    #[test]
    fn test_diff_device_list_empty_to_populated() {
        let mut cached = vec![];
        let fresh = vec![make_device("X", true)];
        diff_device_list(&mut cached, fresh);
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].name, "X");
    }

    #[test]
    fn test_diff_device_list_populated_to_empty() {
        let mut cached = vec![make_device("A", true)];
        diff_device_list(&mut cached, vec![]);
        assert!(cached.is_empty());
    }

    #[test]
    fn test_diff_midi_list_no_change() {
        let mut cached = vec![make_midi("M1", true)];
        let fresh = vec![make_midi("M1", true)];
        diff_midi_list(&mut cached, fresh);
        assert_eq!(cached.len(), 1);
    }

    #[test]
    fn test_diff_midi_list_adds_new() {
        let mut cached = vec![make_midi("M1", true)];
        let fresh = vec![make_midi("M1", true), make_midi("M2", false)];
        diff_midi_list(&mut cached, fresh);
        assert_eq!(cached.len(), 2);
    }

    #[test]
    fn test_diff_midi_list_removes_stale() {
        let mut cached = vec![make_midi("M1", true), make_midi("M2", false)];
        let fresh = vec![make_midi("M1", true)];
        diff_midi_list(&mut cached, fresh);
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].name, "M1");
    }

    #[test]
    fn test_diff_midi_distinguishes_input_output() {
        let mut cached = vec![make_midi("Port", true)];
        let fresh = vec![make_midi("Port", true), make_midi("Port", false)];
        diff_midi_list(&mut cached, fresh);
        assert_eq!(cached.len(), 2);
    }

    #[test]
    fn test_device_cache_starts_needing_refresh() {
        let cache = DeviceCache::new();
        assert!(cache.needs_refresh);
        assert!(cache.audio_devices.is_empty());
        assert!(cache.midi_ports.is_empty());
    }

    #[test]
    fn test_device_cache_default() {
        let cache = DeviceCache::default();
        assert!(cache.needs_refresh);
    }
}
