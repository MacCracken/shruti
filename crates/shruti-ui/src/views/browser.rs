use egui::{ScrollArea, Ui};

use crate::state::UiState;
use crate::theme::ThemeColors;

/// Browser panel tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserTab {
    Files,
    Plugins,
}

/// Draw the browser panel.
pub fn browser_panel(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    // Tab bar
    ui.horizontal(|ui| {
        if ui
            .selectable_label(state.browser_tab == BrowserTab::Files, "Files")
            .clicked()
        {
            state.browser_tab = BrowserTab::Files;
        }
        if ui
            .selectable_label(state.browser_tab == BrowserTab::Plugins, "Plugins")
            .clicked()
        {
            state.browser_tab = BrowserTab::Plugins;
        }

        // Import button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Import...").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio", &["wav", "flac", "aif", "aiff"])
                    .pick_file()
            {
                state.file_entries.push(path.display().to_string());
            }
        });
    });

    ui.separator();

    match state.browser_tab {
        BrowserTab::Files => files_tab(ui, state, colors),
        BrowserTab::Plugins => plugins_tab(ui, state, colors),
    }
}

fn files_tab(ui: &mut Ui, state: &UiState, colors: &ThemeColors) {
    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            if state.file_entries.is_empty() {
                ui.label(
                    egui::RichText::new("No files imported yet. Click Import to add audio files.")
                        .size(10.0)
                        .color(colors.text_secondary()),
                );
            } else {
                for entry in &state.file_entries {
                    let filename = entry.rsplit('/').next().unwrap_or(entry);
                    let icon = if entry.ends_with(".wav") {
                        "WAV"
                    } else if entry.ends_with(".flac") {
                        "FLAC"
                    } else {
                        "AIF"
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(icon)
                                .monospace()
                                .size(8.0)
                                .color(colors.accent()),
                        );
                        ui.label(
                            egui::RichText::new(filename)
                                .size(10.0)
                                .color(colors.text_primary()),
                        );
                    });
                }
            }
        });
}

fn plugins_tab(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    // Search field
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Search:")
                .size(10.0)
                .color(colors.text_secondary()),
        );
        ui.text_edit_singleline(&mut state.plugin_search);
    });

    ui.add_space(4.0);

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            if state.plugin_entries.is_empty() {
                ui.label(
                    egui::RichText::new("No plugins found. Use Scan to detect installed plugins.")
                        .size(10.0)
                        .color(colors.text_secondary()),
                );
            } else {
                let search_lower = state.plugin_search.to_lowercase();
                for plugin in &state.plugin_entries {
                    if !search_lower.is_empty()
                        && !plugin.name.to_lowercase().contains(&search_lower)
                    {
                        continue;
                    }

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&plugin.format)
                                .monospace()
                                .size(8.0)
                                .color(colors.accent()),
                        );
                        ui.label(
                            egui::RichText::new(&plugin.name)
                                .size(10.0)
                                .color(colors.text_primary()),
                        );
                    });
                }
            }
        });
}
