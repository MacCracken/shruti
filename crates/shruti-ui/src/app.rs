use eframe::Frame;
use egui::{CentralPanel, Context, TopBottomPanel};

use crate::input::{Action, ShortcutRegistry, default_keymap};
use crate::state::{UiState, ViewMode};
use crate::theme::{Theme, apply_theme};
use crate::views::{arrangement, browser, mixer, transport};

/// The main Shruti application.
pub struct ShrutiApp {
    state: UiState,
    theme: Theme,
    shortcuts: ShortcutRegistry,
}

impl ShrutiApp {
    pub fn new(state: UiState) -> Self {
        Self {
            state,
            theme: Theme::default(),
            shortcuts: default_keymap(),
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    fn handle_action(&mut self, action: Action) {
        use shruti_session::TransportState;

        match action {
            Action::Play => {
                self.state.session.transport.state = TransportState::Playing;
            }
            Action::Stop => {
                self.state.session.transport.state = TransportState::Stopped;
                self.state.recording = false;
            }
            Action::Pause => {
                self.state.session.transport.state = TransportState::Paused;
            }
            Action::Record => {
                self.state.recording = !self.state.recording;
            }
            Action::ToggleLoop => {
                self.state.session.transport.loop_enabled =
                    !self.state.session.transport.loop_enabled;
            }
            Action::Rewind | Action::GoToStart => {
                self.state.session.transport.position = 0;
            }
            Action::GoToEnd => {
                self.state.session.transport.position = self.state.session.session_length();
            }
            Action::ToggleArrangement => {
                self.state.view_mode = ViewMode::Arrangement;
            }
            Action::ToggleMixer => {
                self.state.view_mode = ViewMode::Mixer;
            }
            Action::ToggleBrowser => {
                self.state.show_browser = !self.state.show_browser;
            }
            Action::ZoomIn => {
                self.state.pixels_per_frame *= 1.3;
            }
            Action::ZoomOut => {
                self.state.pixels_per_frame = (self.state.pixels_per_frame / 1.3).max(0.0001);
            }
            Action::NewAudioTrack => {
                let count = self.state.session.audio_tracks().len() + 1;
                self.state.session.add_audio_track(format!("Audio {count}"));
            }
            Action::ToggleMute => {
                if let Some(idx) = self.state.selected_track
                    && idx < self.state.session.tracks.len()
                {
                    self.state.session.tracks[idx].muted = !self.state.session.tracks[idx].muted;
                }
            }
            Action::ToggleSolo => {
                if let Some(idx) = self.state.selected_track
                    && idx < self.state.session.tracks.len()
                {
                    self.state.session.tracks[idx].solo = !self.state.session.tracks[idx].solo;
                }
            }
            // These actions are recognized but not yet wired
            Action::Undo
            | Action::Redo
            | Action::Cut
            | Action::Copy
            | Action::Paste
            | Action::Delete
            | Action::SelectAll
            | Action::SplitAtPlayhead
            | Action::Duplicate
            | Action::FastForward
            | Action::ZoomToFit
            | Action::NewBusTrack
            | Action::ToggleArm
            | Action::NewSession
            | Action::OpenSession
            | Action::SaveSession
            | Action::ExportAudio => {}
        }
    }
}

impl eframe::App for ShrutiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Apply theme once
        if !self.state.theme_applied {
            apply_theme(ctx, &self.theme.colors);
            self.state.theme_applied = true;
        }

        // Handle keyboard shortcuts
        if let Some(action) = self.shortcuts.check_input(ctx) {
            self.handle_action(action);
        }

        // Handle scroll zoom in arrangement
        if self.state.view_mode == ViewMode::Arrangement {
            ctx.input(|input| {
                if input.modifiers.ctrl {
                    let scroll = input.raw_scroll_delta.y;
                    if scroll != 0.0 {
                        let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
                        self.state.pixels_per_frame =
                            (self.state.pixels_per_frame * factor).clamp(0.00001, 1.0);
                    }
                } else {
                    // Horizontal scroll with shift or trackpad
                    let scroll_x = input.raw_scroll_delta.x - input.raw_scroll_delta.y;
                    self.state.scroll_x = (self.state.scroll_x - scroll_x as f64).max(0.0);
                }
            });
        }

        // Request continuous repaint during playback
        if self.state.session.transport.state == shruti_session::TransportState::Playing {
            ctx.request_repaint();
        }

        let colors = self.theme.colors.clone();

        // Top: Transport bar
        TopBottomPanel::top("transport")
            .min_height(36.0)
            .show(ctx, |ui| {
                transport::transport_bar(ui, &mut self.state, &colors);
            });

        // Bottom: Browser (toggleable)
        if self.state.show_browser {
            TopBottomPanel::bottom("browser")
                .resizable(true)
                .default_height(180.0)
                .min_height(100.0)
                .show(ctx, |ui| {
                    browser::browser_panel(ui, &mut self.state, &colors);
                });
        }

        // Center: Arrangement or Mixer
        CentralPanel::default().show(ctx, |ui| {
            // View switcher bar
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;

                let arr_selected = self.state.view_mode == ViewMode::Arrangement;
                if ui.selectable_label(arr_selected, "Arrangement").clicked() {
                    self.state.view_mode = ViewMode::Arrangement;
                }
                if ui.selectable_label(!arr_selected, "Mixer").clicked() {
                    self.state.view_mode = ViewMode::Mixer;
                }

                ui.separator();

                // Quick-add track button
                if ui.small_button("+ Track").clicked() {
                    let count = self.state.session.audio_tracks().len() + 1;
                    self.state.session.add_audio_track(format!("Audio {count}"));
                }
            });

            ui.separator();

            match self.state.view_mode {
                ViewMode::Arrangement => {
                    arrangement::arrangement_view(ui, &mut self.state, &colors);
                }
                ViewMode::Mixer => {
                    mixer::mixer_view(ui, &mut self.state, &colors);
                }
            }
        });
    }
}
