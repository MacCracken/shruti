use egui::{ScrollArea, Ui};

use crate::state::UiState;
use crate::theme::ThemeColors;

/// Check if a file extension is a supported audio format.
pub(crate) fn is_audio_extension(ext: &str) -> bool {
    matches!(ext, "wav" | "flac" | "aif" | "aiff")
}

/// Return a short format label for display in the file list.
pub(crate) fn audio_format_label(filename: &str) -> &'static str {
    if filename.ends_with(".wav") {
        "WAV"
    } else if filename.ends_with(".flac") {
        "FLAC"
    } else {
        "AIF"
    }
}

/// Extract the filename portion from a full path string.
pub(crate) fn filename_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Check if a plugin name matches a search query (case-insensitive).
pub(crate) fn plugin_matches_search(plugin_name: &str, search: &str) -> bool {
    if search.is_empty() {
        return true;
    }
    plugin_name.to_lowercase().contains(&search.to_lowercase())
}

/// Browser panel tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserTab {
    Files,
    Plugins,
}

/// Draw the browser panel.
pub fn browser_panel(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    // Handle file drops
    let dropped_files: Vec<egui::DroppedFile> = ui.ctx().input(|i| i.raw.dropped_files.clone());
    for file in &dropped_files {
        if let Some(path) = &file.path {
            let path_str = path.display().to_string();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if is_audio_extension(ext) && !state.file_entries.contains(&path_str) {
                if let Err(e) = state.session.audio_pool.load(path) {
                    eprintln!("Failed to load audio file: {e}");
                }
                state.file_entries.push(path_str.clone());
            }
        }
    }

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
                if let Err(e) = state.session.audio_pool.load(&path) {
                    eprintln!("Failed to load audio file: {e}");
                }
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
                    let filename = filename_from_path(entry);
                    let icon = audio_format_label(entry);

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
                for plugin in &state.plugin_entries {
                    if !plugin_matches_search(&plugin.name, &state.plugin_search) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- is_audio_extension tests ----

    #[test]
    fn audio_extension_wav() {
        assert!(is_audio_extension("wav"));
    }

    #[test]
    fn audio_extension_flac() {
        assert!(is_audio_extension("flac"));
    }

    #[test]
    fn audio_extension_aif() {
        assert!(is_audio_extension("aif"));
    }

    #[test]
    fn audio_extension_aiff() {
        assert!(is_audio_extension("aiff"));
    }

    #[test]
    fn audio_extension_mp3_not_supported() {
        assert!(!is_audio_extension("mp3"));
    }

    #[test]
    fn audio_extension_ogg_not_supported() {
        assert!(!is_audio_extension("ogg"));
    }

    #[test]
    fn audio_extension_empty() {
        assert!(!is_audio_extension(""));
    }

    #[test]
    fn audio_extension_uppercase_not_matched() {
        assert!(!is_audio_extension("WAV"));
    }

    #[test]
    fn audio_extension_txt() {
        assert!(!is_audio_extension("txt"));
    }

    // ---- audio_format_label tests ----

    #[test]
    fn format_label_wav() {
        assert_eq!(audio_format_label("song.wav"), "WAV");
    }

    #[test]
    fn format_label_flac() {
        assert_eq!(audio_format_label("song.flac"), "FLAC");
    }

    #[test]
    fn format_label_aif() {
        assert_eq!(audio_format_label("song.aif"), "AIF");
    }

    #[test]
    fn format_label_aiff() {
        assert_eq!(audio_format_label("song.aiff"), "AIF");
    }

    #[test]
    fn format_label_unknown_defaults_to_aif() {
        assert_eq!(audio_format_label("song.xyz"), "AIF");
    }

    // ---- filename_from_path tests ----

    #[test]
    fn filename_from_full_path() {
        assert_eq!(filename_from_path("/home/user/music/song.wav"), "song.wav");
    }

    #[test]
    fn filename_from_just_filename() {
        assert_eq!(filename_from_path("song.wav"), "song.wav");
    }

    #[test]
    fn filename_from_nested_path() {
        assert_eq!(filename_from_path("/a/b/c/d/track.flac"), "track.flac");
    }

    #[test]
    fn filename_from_empty() {
        assert_eq!(filename_from_path(""), "");
    }

    // ---- plugin_matches_search tests ----

    #[test]
    fn plugin_matches_empty_search() {
        assert!(plugin_matches_search("Serum", ""));
    }

    #[test]
    fn plugin_matches_exact() {
        assert!(plugin_matches_search("Serum", "Serum"));
    }

    #[test]
    fn plugin_matches_case_insensitive() {
        assert!(plugin_matches_search("Serum", "serum"));
        assert!(plugin_matches_search("serum", "SERUM"));
    }

    #[test]
    fn plugin_matches_partial() {
        assert!(plugin_matches_search("Massive X", "mass"));
    }

    #[test]
    fn plugin_no_match() {
        assert!(!plugin_matches_search("Serum", "massive"));
    }
}
