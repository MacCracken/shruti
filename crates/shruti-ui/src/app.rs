use eframe::Frame;
use egui::{CentralPanel, Context, TopBottomPanel};

use crate::engine::AudioEngine;
use crate::input::{Action, ShortcutRegistry, default_keymap};
use crate::state::{UiState, ViewMode};
use crate::theme::{Theme, apply_theme};
use crate::views::settings::DeviceCache;
use crate::views::{arrangement, browser, mixer, settings, transport};

/// The main Shruti application.
pub struct ShrutiApp {
    state: UiState,
    theme: Theme,
    shortcuts: ShortcutRegistry,
    engine: Option<AudioEngine>,
    device_cache: DeviceCache,
}

impl ShrutiApp {
    pub fn new(state: UiState) -> Self {
        Self {
            state,
            theme: Theme::default(),
            shortcuts: default_keymap(),
            engine: None,
            device_cache: DeviceCache::new(),
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
                if self.state.recording {
                    // Stop recording
                    self.state.recording = false;
                    if let Some(engine) = &mut self.engine
                        && let Some(samples) = engine.stop_recording()
                    {
                        let position = self.state.session.transport.position;
                        let channels = 2u16;
                        let frames = samples.len() / channels as usize;

                        if frames > 0 {
                            let buf = shruti_dsp::AudioBuffer::from_interleaved(samples, channels);
                            let recording_id = format!("recording_{}", uuid::Uuid::new_v4());
                            self.state
                                .session
                                .audio_pool
                                .insert(recording_id.clone(), buf);

                            // Add region to the first armed track
                            for track in &mut self.state.session.tracks {
                                if track.armed {
                                    let region = shruti_session::Region::new(
                                        recording_id.clone(),
                                        position,
                                        0,
                                        frames as u64,
                                    );
                                    track.add_region(region);
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    // Start recording
                    if let Some(engine) = &mut self.engine
                        && engine.start_recording().is_ok()
                    {
                        self.state.recording = true;
                    }
                }
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
            Action::Undo => {
                self.state.undo.undo(&mut self.state.session);
            }
            Action::Redo => {
                self.state.undo.redo(&mut self.state.session);
            }
            Action::Delete => {
                if let (Some(track_idx), Some(region_id)) =
                    (self.state.selected_track, self.state.selected_region)
                    && track_idx < self.state.session.tracks.len()
                {
                    self.state.session.tracks[track_idx].remove_region(region_id);
                    self.state.selected_region = None;
                }
            }
            Action::NewBusTrack => {
                let count = self
                    .state
                    .session
                    .tracks
                    .iter()
                    .filter(|t| t.kind == shruti_session::TrackKind::Bus)
                    .count()
                    + 1;
                self.state.session.add_bus_track(format!("Bus {count}"));
            }
            Action::ToggleArm => {
                if let Some(idx) = self.state.selected_track
                    && idx < self.state.session.tracks.len()
                {
                    self.state.session.tracks[idx].armed = !self.state.session.tracks[idx].armed;
                }
            }
            Action::ZoomToFit => {
                let length = self.state.session.session_length();
                if length > 0 {
                    // Approximate with 1000px available width
                    self.state.pixels_per_frame = 1000.0 / length as f64;
                    self.state.scroll_x = 0.0;
                }
            }
            Action::FastForward => {
                let frames_per_beat = (self.state.session.sample_rate as f64 * 60.0)
                    / self.state.session.transport.bpm;
                let frames_per_bar =
                    frames_per_beat * self.state.session.transport.time_sig_num as f64;
                self.state.session.transport.position += frames_per_bar as u64;
            }
            Action::SplitAtPlayhead => {
                if let (Some(track_idx), Some(region_id)) =
                    (self.state.selected_track, self.state.selected_region)
                    && track_idx < self.state.session.tracks.len()
                {
                    let playhead = self.state.session.transport.position;
                    let track = &mut self.state.session.tracks[track_idx];
                    if let Some(region) = track.region(region_id).cloned()
                        && let Some((left, right)) = region.split_at(playhead)
                    {
                        track.remove_region(region_id);
                        track.add_region(left);
                        track.add_region(right);
                        self.state.selected_region = None;
                    }
                }
            }
            Action::Duplicate => {
                if let (Some(track_idx), Some(region_id)) =
                    (self.state.selected_track, self.state.selected_region)
                    && track_idx < self.state.session.tracks.len()
                {
                    let track = &self.state.session.tracks[track_idx];
                    if let Some(region) = track.region(region_id) {
                        let mut dup = region.clone();
                        dup.id = shruti_session::RegionId::new();
                        dup.timeline_pos = region.end_pos();
                        self.state.session.tracks[track_idx].add_region(dup);
                    }
                }
            }
            Action::NewSession => {
                self.state.session = shruti_session::Session::new("Untitled", 48000, 256);
                self.state.selected_track = None;
                self.state.selected_region = None;
            }
            Action::SaveSession => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Shruti Session", &["shruti"])
                    .save_file()
                {
                    let _ = shruti_session::store::SessionStore::create(&path, &self.state.session);
                }
            }
            Action::OpenSession => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Shruti Session", &["shruti"])
                    .pick_file()
                    && let Ok((_store, session)) = shruti_session::store::SessionStore::open(&path)
                {
                    self.state.session = session;
                    self.state.selected_track = None;
                    self.state.selected_region = None;
                }
            }
            Action::ExportAudio => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("WAV", &["wav"])
                    .save_file()
                {
                    let length = self.state.session.session_length();
                    if length > 0 {
                        let channels = 2u16;
                        let mut output = shruti_dsp::AudioBuffer::new(channels, length as u32);
                        let mut tl = shruti_session::Timeline::new(channels, length as u32);
                        tl.render(
                            &self.state.session.tracks,
                            &self.state.session.transport,
                            &self.state.session.audio_pool,
                            &mut output,
                        );
                        let format = shruti_dsp::AudioFormat::new(
                            self.state.session.sample_rate,
                            channels,
                            0,
                        );
                        let _ = shruti_dsp::io::write_wav_file(&path, &output, &format);
                    }
                }
            }
            Action::Cut => {
                if let (Some(track_idx), Some(region_id)) =
                    (self.state.selected_track, self.state.selected_region)
                    && track_idx < self.state.session.tracks.len()
                    && let Some(region) =
                        self.state.session.tracks[track_idx].remove_region(region_id)
                {
                    self.state.clipboard_region = Some(region);
                    self.state.selected_region = None;
                }
            }
            Action::Copy => {
                if let (Some(track_idx), Some(region_id)) =
                    (self.state.selected_track, self.state.selected_region)
                    && track_idx < self.state.session.tracks.len()
                    && let Some(region) = self.state.session.tracks[track_idx].region(region_id)
                {
                    let mut copy = region.clone();
                    copy.id = shruti_session::RegionId::new();
                    self.state.clipboard_region = Some(copy);
                }
            }
            Action::Paste => {
                if let Some(track_idx) = self.state.selected_track
                    && track_idx < self.state.session.tracks.len()
                    && let Some(ref clip) = self.state.clipboard_region
                {
                    let mut pasted = clip.clone();
                    pasted.id = shruti_session::RegionId::new();
                    pasted.timeline_pos = self.state.session.transport.position;
                    self.state.session.tracks[track_idx].add_region(pasted);
                }
            }
            Action::SelectAll => {
                if let Some(track_idx) = self.state.selected_track
                    && track_idx < self.state.session.tracks.len()
                    && let Some(first) = self.state.session.tracks[track_idx].regions.first()
                {
                    self.state.selected_region = Some(first.id);
                }
            }
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

        // Sync meter levels from the audio engine
        if let Some(ref engine) = self.engine {
            let peaks = engine.read_meters();
            self.state
                .meter_levels
                .resize(self.state.session.tracks.len(), ([0.0; 2], [0.0; 2]));
            for (i, level) in self.state.meter_levels.iter_mut().enumerate() {
                if let Some(&peak) = peaks.get(i) {
                    level.0 = peak; // peak L/R
                    level.1 = peak; // use peak as RMS approximation
                }
            }
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

                if ui
                    .selectable_label(self.state.view_mode == ViewMode::Arrangement, "Arrangement")
                    .clicked()
                {
                    self.state.view_mode = ViewMode::Arrangement;
                }
                if ui
                    .selectable_label(self.state.view_mode == ViewMode::Mixer, "Mixer")
                    .clicked()
                {
                    self.state.view_mode = ViewMode::Mixer;
                }
                if ui
                    .selectable_label(self.state.view_mode == ViewMode::Settings, "Settings")
                    .clicked()
                {
                    self.state.view_mode = ViewMode::Settings;
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
                ViewMode::Settings => {
                    settings::settings_view(ui, &mut self.state, &colors, &mut self.device_cache);
                }
            }
        });
    }
}
