use egui::{Rect, ScrollArea, Stroke, Ui, pos2, vec2};

use shruti_session::RegionId;
use shruti_session::edit::EditCommand;

use crate::state::{ArrangementDrag, UiState};
use crate::theme::ThemeColors;
use crate::widgets::{automation_lane, region_clip, timeline_ruler, track_header, waveform};

const TRACK_HEADER_WIDTH: f32 = 160.0;
const TRACK_HEIGHT: f32 = 60.0;
const RULER_HEIGHT: f32 = 24.0;
/// Width of the resize handles at region edges (in pixels).
const HANDLE_WIDTH: f32 = 5.0;

/// Pending actions collected during the render loop, applied afterwards.
enum PendingAction {
    SelectRegion {
        region_id: RegionId,
        track_index: usize,
    },
    StartMoveRegion {
        region_id: RegionId,
        track_index: usize,
        start_frame: u64,
        grab_offset_px: f32,
    },
    StartTrimStart {
        region_id: RegionId,
        track_index: usize,
        original_pos: u64,
        original_offset: u64,
        original_duration: u64,
    },
    StartTrimEnd {
        region_id: RegionId,
        track_index: usize,
        original_duration: u64,
    },
    StartReorderTrack {
        from_index: usize,
    },
}

/// Draw the arrangement/timeline view.
pub fn arrangement_view(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
    // Handle file drops
    let dropped_files: Vec<egui::DroppedFile> = ui.ctx().input(|i| i.raw.dropped_files.clone());
    for file in &dropped_files {
        if let Some(path) = &file.path {
            let path_str = path.display().to_string();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "wav" | "flac" | "aif" | "aiff")
                && !state.file_entries.contains(&path_str)
            {
                let _ = state.session.audio_pool.load(path);
                state.file_entries.push(path_str);
            }
        }
    }

    let available = ui.available_rect_before_wrap();

    // Ruler at top
    let ruler_rect = Rect::from_min_size(
        pos2(available.left() + TRACK_HEADER_WIDTH, available.top()),
        vec2(available.width() - TRACK_HEADER_WIDTH, RULER_HEIGHT),
    );
    timeline_ruler::draw_ruler(
        ui,
        ruler_rect,
        state.scroll_x,
        state.pixels_per_frame,
        state.session.sample_rate,
        state.session.transport.bpm,
        colors,
    );

    // Content area below ruler
    let content_top = available.top() + RULER_HEIGHT;
    let content_rect = Rect::from_min_max(
        pos2(available.left(), content_top),
        available.right_bottom(),
    );

    // Copy values we need inside the closure to avoid borrow conflicts.
    let scroll_x = state.scroll_x;
    let pixels_per_frame = state.pixels_per_frame;

    let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

    // Collect pending actions during the render loop.
    let mut pending_actions: Vec<PendingAction> = Vec::new();

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(&mut content_ui, |ui| {
            let track_count = state.session.tracks.len();

            for track_idx in 0..track_count {
                ui.horizontal(|ui| {
                    // Track header
                    let track = &state.session.tracks[track_idx];
                    let header_state = track_header::TrackHeaderState {
                        name: track.name.clone(),
                        muted: track.muted,
                        solo: track.solo,
                        armed: track.armed,
                        selected: state.selected_track == Some(track_idx),
                        color_index: track_idx,
                    };

                    let header_resp = track_header::track_header_ui(
                        ui,
                        &header_state,
                        colors,
                        TRACK_HEADER_WIDTH,
                    );

                    if header_resp.selected {
                        state.selected_track = Some(track_idx);
                    }
                    if header_resp.mute_clicked {
                        state.session.tracks[track_idx].muted =
                            !state.session.tracks[track_idx].muted;
                    }
                    if header_resp.solo_clicked {
                        state.session.tracks[track_idx].solo =
                            !state.session.tracks[track_idx].solo;
                    }
                    if header_resp.arm_clicked {
                        state.session.tracks[track_idx].armed =
                            !state.session.tracks[track_idx].armed;
                    }

                    // Make the track header area draggable for reordering.
                    // We interact on the same rect the header used.
                    let header_rect = Rect::from_min_size(
                        pos2(
                            ui.min_rect().left() - TRACK_HEADER_WIDTH,
                            ui.min_rect().top(),
                        ),
                        vec2(TRACK_HEADER_WIDTH, TRACK_HEIGHT),
                    );
                    let header_drag_resp = ui.interact(
                        header_rect,
                        egui::Id::new(("track_header_drag", track_idx)),
                        egui::Sense::drag(),
                    );
                    if header_drag_resp.drag_started() {
                        pending_actions.push(PendingAction::StartReorderTrack {
                            from_index: track_idx,
                        });
                    }

                    // Timeline lane for this track
                    let lane_width = ui.available_width();
                    let (lane_rect, _) = ui
                        .allocate_exact_size(vec2(lane_width, TRACK_HEIGHT), egui::Sense::click());

                    if ui.is_rect_visible(lane_rect) {
                        let painter = ui.painter_at(lane_rect);

                        // Lane background (alternating)
                        let bg = if track_idx % 2 == 0 {
                            colors.bg_tertiary()
                        } else {
                            colors.bg_secondary()
                        };
                        painter.rect_filled(lane_rect, 0.0, bg);

                        // Bottom separator
                        painter.line_segment(
                            [lane_rect.left_bottom(), lane_rect.right_bottom()],
                            Stroke::new(0.5, colors.separator()),
                        );

                        // Draw grid lines
                        draw_grid(
                            ui,
                            lane_rect,
                            scroll_x,
                            pixels_per_frame,
                            state.session.sample_rate,
                            state.session.transport.bpm,
                            colors,
                        );

                        // Draw regions
                        let track = &state.session.tracks[track_idx];
                        for region in &track.regions {
                            let region_x = lane_rect.left()
                                + (region.timeline_pos as f64 * pixels_per_frame - scroll_x) as f32;
                            let region_w = (region.duration as f64 * pixels_per_frame) as f32;

                            if region_x + region_w >= lane_rect.left()
                                && region_x <= lane_rect.right()
                            {
                                let region_rect = Rect::from_min_size(
                                    pos2(region_x.max(lane_rect.left()), lane_rect.top() + 2.0),
                                    vec2(
                                        region_w.min(lane_rect.right() - region_x),
                                        TRACK_HEIGHT - 4.0,
                                    ),
                                );
                                let selected = state.selected_region == Some(region.id);
                                region_clip::draw_region(
                                    ui,
                                    region_rect,
                                    &region.audio_file_id,
                                    track_idx,
                                    selected,
                                    colors,
                                );

                                // Render waveform inside region if audio data is available
                                if let Some(source) =
                                    state.session.audio_pool.get(&region.audio_file_id)
                                {
                                    let peaks = waveform::WaveformPeaks::from_samples(
                                        source.as_interleaved(),
                                        0, // left channel
                                        source.channels() as usize,
                                    );
                                    let samples_per_pixel = (1.0 / pixels_per_frame) as f32;
                                    let start_sample = region.source_offset as usize;
                                    waveform::draw_waveform(
                                        ui,
                                        region_rect.shrink2(vec2(1.0, 4.0)),
                                        &peaks,
                                        start_sample,
                                        samples_per_pixel,
                                        colors,
                                    );
                                }

                                // Selection highlight border
                                if selected {
                                    painter.rect_stroke(
                                        region_rect,
                                        egui::CornerRadius::same(3),
                                        Stroke::new(2.0, colors.accent()),
                                        egui::StrokeKind::Outside,
                                    );
                                }

                                // Make region interactive (click-to-select and drag-to-move/resize)
                                let region_id = region.id;
                                let timeline_pos = region.timeline_pos;
                                let source_offset = region.source_offset;
                                let duration = region.duration;

                                let region_resp = ui.interact(
                                    region_rect,
                                    egui::Id::new(("region", region_id.0)),
                                    egui::Sense::click_and_drag(),
                                );

                                if region_resp.clicked() {
                                    pending_actions.push(PendingAction::SelectRegion {
                                        region_id,
                                        track_index: track_idx,
                                    });
                                }

                                // Determine resize handle rects
                                let left_handle = Rect::from_min_size(
                                    region_rect.left_top(),
                                    vec2(HANDLE_WIDTH, region_rect.height()),
                                );
                                let right_handle = Rect::from_min_size(
                                    pos2(region_rect.right() - HANDLE_WIDTH, region_rect.top()),
                                    vec2(HANDLE_WIDTH, region_rect.height()),
                                );

                                if region_resp.drag_started()
                                    && let Some(pos) = region_resp.interact_pointer_pos()
                                {
                                    if left_handle.contains(pos) {
                                        pending_actions.push(PendingAction::StartTrimStart {
                                            region_id,
                                            track_index: track_idx,
                                            original_pos: timeline_pos,
                                            original_offset: source_offset,
                                            original_duration: duration,
                                        });
                                    } else if right_handle.contains(pos) {
                                        pending_actions.push(PendingAction::StartTrimEnd {
                                            region_id,
                                            track_index: track_idx,
                                            original_duration: duration,
                                        });
                                    } else {
                                        let grab_offset = pos.x - region_rect.left();
                                        pending_actions.push(PendingAction::StartMoveRegion {
                                            region_id,
                                            track_index: track_idx,
                                            start_frame: timeline_pos,
                                            grab_offset_px: grab_offset,
                                        });
                                    }
                                    // Also select on drag start
                                    pending_actions.push(PendingAction::SelectRegion {
                                        region_id,
                                        track_index: track_idx,
                                    });
                                }

                                // Show resize cursor when hovering edges
                                if let Some(pos) = ui.ctx().input(|i| i.pointer.hover_pos())
                                    && region_rect.contains(pos)
                                    && (left_handle.contains(pos) || right_handle.contains(pos))
                                {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                }
                            }
                        }

                        // Draw automation lanes
                        let track = &state.session.tracks[track_idx];
                        for lane in &track.automation {
                            if lane.enabled {
                                let points: Vec<(f64, f32)> = lane
                                    .points
                                    .iter()
                                    .map(|p| (p.position as f64, p.value))
                                    .collect();
                                automation_lane::draw_automation(
                                    ui,
                                    lane_rect,
                                    &points,
                                    scroll_x,
                                    pixels_per_frame,
                                    colors,
                                );
                            }
                        }

                        // Draw MIDI clips
                        let track = &state.session.tracks[track_idx];
                        for clip in &track.midi_clips {
                            let clip_x = lane_rect.left()
                                + (clip.timeline_pos as f64 * pixels_per_frame - scroll_x) as f32;
                            let clip_w = (clip.duration as f64 * pixels_per_frame) as f32;

                            if clip_x + clip_w >= lane_rect.left() && clip_x <= lane_rect.right() {
                                let clip_rect = Rect::from_min_size(
                                    pos2(clip_x.max(lane_rect.left()), lane_rect.top() + 2.0),
                                    vec2(
                                        clip_w.min(lane_rect.right() - clip_x),
                                        TRACK_HEIGHT - 4.0,
                                    ),
                                );

                                let midi_color = track_header::track_color(track_idx);
                                painter.rect_filled(
                                    clip_rect,
                                    egui::CornerRadius::same(3),
                                    midi_color.linear_multiply(0.2),
                                );
                                painter.rect_stroke(
                                    clip_rect,
                                    egui::CornerRadius::same(3),
                                    Stroke::new(1.0, midi_color.linear_multiply(0.5)),
                                    egui::StrokeKind::Outside,
                                );

                                // Draw note bars inside clip
                                if clip_rect.width() > 10.0 {
                                    for note in &clip.notes {
                                        let note_x = clip_rect.left()
                                            + (note.position as f64 * pixels_per_frame) as f32;
                                        let note_w =
                                            (note.duration as f64 * pixels_per_frame) as f32;
                                        let note_y = clip_rect.bottom()
                                            - (note.note as f32 / 127.0) * clip_rect.height();
                                        let note_h = 2.0;

                                        if note_x + note_w >= clip_rect.left()
                                            && note_x <= clip_rect.right()
                                        {
                                            let note_rect = Rect::from_min_size(
                                                pos2(note_x.max(clip_rect.left()), note_y),
                                                vec2(
                                                    note_w.min(clip_rect.right() - note_x).max(1.0),
                                                    note_h,
                                                ),
                                            );
                                            painter.rect_filled(
                                                note_rect,
                                                0.0,
                                                midi_color.linear_multiply(0.7),
                                            );
                                        }
                                    }
                                }

                                // Clip name
                                if clip_rect.width() > 30.0 {
                                    painter.text(
                                        pos2(clip_rect.left() + 4.0, clip_rect.top() + 4.0),
                                        egui::Align2::LEFT_TOP,
                                        &clip.name,
                                        egui::FontId::new(9.0, egui::FontFamily::Proportional),
                                        colors.text_primary(),
                                    );
                                }
                            }
                        }

                        // Draw playhead
                        let playhead_x = lane_rect.left()
                            + (state.session.transport.position as f64 * pixels_per_frame
                                - scroll_x) as f32;
                        if playhead_x >= lane_rect.left() && playhead_x <= lane_rect.right() {
                            painter.line_segment(
                                [
                                    pos2(playhead_x, lane_rect.top()),
                                    pos2(playhead_x, lane_rect.bottom()),
                                ],
                                Stroke::new(1.0, colors.playhead()),
                            );
                        }
                    }
                });
            }
        });

    // ---------------------------------------------------------------
    // Process pending actions from the render loop
    // ---------------------------------------------------------------
    for action in pending_actions {
        match action {
            PendingAction::SelectRegion {
                region_id,
                track_index,
            } => {
                state.selected_region = Some(region_id);
                state.selected_track = Some(track_index);
            }
            PendingAction::StartMoveRegion {
                region_id,
                track_index,
                start_frame,
                grab_offset_px,
            } => {
                state.drag = Some(ArrangementDrag::MoveRegion {
                    region_id,
                    track_index,
                    start_frame,
                    grab_offset_px,
                });
            }
            PendingAction::StartTrimStart {
                region_id,
                track_index,
                original_pos,
                original_offset,
                original_duration,
            } => {
                state.drag = Some(ArrangementDrag::TrimStart {
                    region_id,
                    track_index,
                    original_pos,
                    original_offset,
                    original_duration,
                });
            }
            PendingAction::StartTrimEnd {
                region_id,
                track_index,
                original_duration,
            } => {
                state.drag = Some(ArrangementDrag::TrimEnd {
                    region_id,
                    track_index,
                    original_duration,
                });
            }
            PendingAction::StartReorderTrack { from_index } => {
                state.drag = Some(ArrangementDrag::ReorderTrack {
                    from_index,
                    current_index: from_index,
                });
            }
        }
    }

    // ---------------------------------------------------------------
    // Handle active drag operations
    // ---------------------------------------------------------------
    let pointer_released = ui.ctx().input(|i| i.pointer.any_released());
    let pointer_pos = ui.ctx().input(|i| i.pointer.interact_pos());

    if let Some(drag) = state.drag.clone() {
        match drag {
            ArrangementDrag::MoveRegion {
                region_id,
                track_index,
                start_frame,
                grab_offset_px,
            } => {
                if pointer_released {
                    if let Some(pos) = pointer_pos {
                        let lane_left = available.left() + TRACK_HEADER_WIDTH;
                        let new_x = pos.x - lane_left - grab_offset_px;
                        let new_frame =
                            ((new_x as f64 + scroll_x) / pixels_per_frame).max(0.0) as u64;

                        if new_frame != start_frame && track_index < state.session.tracks.len() {
                            let track_id = state.session.tracks[track_index].id;
                            let cmd = EditCommand::MoveRegion {
                                track_id,
                                region_id,
                                old_pos: start_frame,
                                new_pos: new_frame,
                            };
                            state.undo.execute(cmd, &mut state.session);
                        }
                    }
                    state.drag = None;
                } else if let Some(pos) = pointer_pos {
                    // Live preview: move region to current mouse position
                    let lane_left = available.left() + TRACK_HEADER_WIDTH;
                    let new_x = pos.x - lane_left - grab_offset_px;
                    let new_frame = ((new_x as f64 + scroll_x) / pixels_per_frame).max(0.0) as u64;

                    if track_index < state.session.tracks.len()
                        && let Some(r) = state.session.tracks[track_index].region_mut(region_id)
                    {
                        r.timeline_pos = new_frame;
                    }
                    // Update start_frame in drag state so undo uses the original
                    // (keep start_frame unchanged -- it's the original position for undo)
                }
            }
            ArrangementDrag::TrimEnd {
                region_id,
                track_index,
                original_duration,
            } => {
                if pointer_released {
                    if track_index < state.session.tracks.len()
                        && let Some(r) = state.session.tracks[track_index].region(region_id)
                        && r.duration != original_duration
                    {
                        let current_duration = r.duration;
                        let track_id = state.session.tracks[track_index].id;
                        let cmd = EditCommand::TrimEnd {
                            track_id,
                            region_id,
                            old_duration: original_duration,
                            new_end: current_duration,
                        };
                        state.undo.execute(cmd, &mut state.session);
                    }
                    state.drag = None;
                } else if let Some(pos) = pointer_pos {
                    // Live preview: resize the region end
                    if track_index < state.session.tracks.len()
                        && let Some(r) = state.session.tracks[track_index].region_mut(region_id)
                    {
                        let lane_left = available.left() + TRACK_HEADER_WIDTH;
                        let end_x = pos.x - lane_left;
                        let end_frame =
                            ((end_x as f64 + scroll_x) / pixels_per_frame).max(0.0) as u64;
                        let new_duration = end_frame.saturating_sub(r.timeline_pos).max(1);
                        r.duration = new_duration;
                    }
                }
            }
            ArrangementDrag::TrimStart {
                region_id,
                track_index,
                original_pos,
                original_offset,
                original_duration,
            } => {
                if pointer_released {
                    if track_index < state.session.tracks.len()
                        && let Some(r) = state.session.tracks[track_index].region(region_id)
                        && r.timeline_pos != original_pos
                    {
                        let current_pos = r.timeline_pos;
                        let track_id = state.session.tracks[track_index].id;
                        let cmd = EditCommand::TrimStart {
                            track_id,
                            region_id,
                            old_start: original_pos,
                            old_offset: original_offset,
                            old_duration: original_duration,
                            new_start: current_pos,
                        };
                        state.undo.execute(cmd, &mut state.session);
                    }
                    state.drag = None;
                } else if let Some(pos) = pointer_pos {
                    // Live preview: trim start
                    if track_index < state.session.tracks.len()
                        && let Some(r) = state.session.tracks[track_index].region_mut(region_id)
                    {
                        let lane_left = available.left() + TRACK_HEADER_WIDTH;
                        let start_x = pos.x - lane_left;
                        let new_start_frame =
                            ((start_x as f64 + scroll_x) / pixels_per_frame).max(0.0) as u64;
                        let original_end = original_pos + original_duration;
                        // Don't let start go past the end
                        let clamped_start = new_start_frame.min(original_end.saturating_sub(1));
                        // Don't let start go before the original start minus offset
                        let clamped_start =
                            clamped_start.max(original_pos.saturating_sub(original_offset));
                        let delta = clamped_start.saturating_sub(original_pos);
                        r.timeline_pos = clamped_start;
                        r.source_offset = original_offset + delta;
                        r.duration = original_duration.saturating_sub(delta).max(1);
                    }
                }
            }
            ArrangementDrag::ReorderTrack {
                from_index,
                current_index: _,
            } => {
                if pointer_released {
                    if let Some(pos) = pointer_pos {
                        // Determine which track row the mouse is over
                        let row_y = pos.y - content_rect.top();
                        let to_index = ((row_y / TRACK_HEIGHT).floor() as usize)
                            .min(state.session.tracks.len().saturating_sub(1));
                        if to_index != from_index && state.session.tracks.len() > 1 {
                            let cmd = EditCommand::MoveTrack {
                                from_index,
                                to_index,
                            };
                            state.undo.execute(cmd, &mut state.session);
                        }
                    }
                    state.drag = None;
                } else if let Some(pos) = pointer_pos {
                    // Update current_index for visual feedback
                    let row_y = pos.y - content_rect.top();
                    let to_index = ((row_y / TRACK_HEIGHT).floor() as usize)
                        .min(state.session.tracks.len().saturating_sub(1));
                    state.drag = Some(ArrangementDrag::ReorderTrack {
                        from_index,
                        current_index: to_index,
                    });
                }
            }
        }
    }

    // Show drop zone overlay when dragging files
    let is_hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    if is_hovering {
        let available = ui.available_rect_before_wrap();
        let painter = ui.painter();
        painter.rect_filled(
            available,
            0.0,
            egui::Color32::from_rgba_premultiplied(100, 150, 255, 30),
        );
        painter.text(
            available.center(),
            egui::Align2::CENTER_CENTER,
            "Drop audio files here",
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
            egui::Color32::from_rgba_premultiplied(200, 220, 255, 180),
        );
    }

    // Draw reorder indicator line when dragging a track header
    if let Some(ArrangementDrag::ReorderTrack {
        current_index,
        from_index,
        ..
    }) = &state.drag
        && *current_index != *from_index
    {
        let indicator_y = content_rect.top() + (*current_index as f32 * TRACK_HEIGHT);
        let painter = ui.painter();
        painter.line_segment(
            [
                pos2(content_rect.left(), indicator_y),
                pos2(content_rect.right(), indicator_y),
            ],
            Stroke::new(2.0, colors.accent()),
        );
    }
}

fn draw_grid(
    ui: &mut Ui,
    rect: Rect,
    scroll_offset: f64,
    pixels_per_frame: f64,
    sample_rate: u32,
    bpm: f64,
    colors: &ThemeColors,
) {
    let painter = ui.painter_at(rect);

    let frames_per_beat = (sample_rate as f64 * 60.0) / bpm;
    let frames_per_bar = frames_per_beat * 4.0;
    let pixels_per_bar = frames_per_bar * pixels_per_frame;

    if pixels_per_bar < 4.0 {
        return;
    }

    let start_frame = (scroll_offset / pixels_per_frame) as i64;
    let end_frame = start_frame + (rect.width() as f64 / pixels_per_frame) as i64;

    let bar_start = (start_frame as f64 / frames_per_bar).floor() as i64;
    let bar_end = (end_frame as f64 / frames_per_bar).ceil() as i64;

    for i in bar_start..=bar_end {
        let frame = (i as f64 * frames_per_bar) as i64;
        let x = rect.left() + (frame as f64 * pixels_per_frame - scroll_offset) as f32;
        if x >= rect.left() && x <= rect.right() {
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(0.5, colors.grid()),
            );
        }

        // Beat subdivision lines if zoomed in enough
        if pixels_per_bar > 80.0 {
            for beat in 1..4 {
                let beat_frame = frame + (beat as f64 * frames_per_beat) as i64;
                let bx =
                    rect.left() + (beat_frame as f64 * pixels_per_frame - scroll_offset) as f32;
                if bx >= rect.left() && bx <= rect.right() {
                    painter.line_segment(
                        [pos2(bx, rect.top()), pos2(bx, rect.bottom())],
                        Stroke::new(0.25, colors.grid()),
                    );
                }
            }
        }
    }
}
