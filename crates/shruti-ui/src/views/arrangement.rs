use egui::{Rect, ScrollArea, Stroke, Ui, pos2, vec2};

use crate::state::UiState;
use crate::theme::ThemeColors;
use crate::widgets::{region_clip, timeline_ruler, track_header};

const TRACK_HEADER_WIDTH: f32 = 160.0;
const TRACK_HEIGHT: f32 = 60.0;
const RULER_HEIGHT: f32 = 24.0;

/// Draw the arrangement/timeline view.
pub fn arrangement_view(ui: &mut Ui, state: &mut UiState, colors: &ThemeColors) {
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

    let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

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
                            state.scroll_x,
                            state.pixels_per_frame,
                            state.session.sample_rate,
                            state.session.transport.bpm,
                            colors,
                        );

                        // Draw regions
                        let track = &state.session.tracks[track_idx];
                        for region in &track.regions {
                            let region_x = lane_rect.left()
                                + (region.timeline_pos as f64 * state.pixels_per_frame
                                    - state.scroll_x) as f32;
                            let region_w = (region.duration as f64 * state.pixels_per_frame) as f32;

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
                            }
                        }

                        // Draw playhead
                        let playhead_x = lane_rect.left()
                            + (state.session.transport.position as f64 * state.pixels_per_frame
                                - state.scroll_x) as f32;
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
