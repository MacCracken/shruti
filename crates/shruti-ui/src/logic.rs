//! Pure logic functions extracted from UI callbacks.
//!
//! These functions contain the computation and state mutation logic that was
//! previously embedded in egui rendering callbacks. Extracting them enables
//! unit testing without a GPU context.

use shruti_session::{FramePos, Region};

/// Calculate the fast-forward target position (advance by one bar).
pub fn fast_forward_position(
    current_pos: FramePos,
    bpm: f64,
    sample_rate: u32,
    time_sig_num: u8,
) -> FramePos {
    let frames_per_beat = (sample_rate as f64 * 60.0) / bpm;
    let frames_per_bar = frames_per_beat * time_sig_num as f64;
    current_pos + FramePos(frames_per_bar as u64)
}

/// Convert a pixel X coordinate to a frame position.
///
/// Used in arrangement drag operations (move, trim-start, trim-end).
/// `x_px` is the mouse X relative to the lane left edge.
pub fn pixel_to_frame(x_px: f32, scroll_x: f64, pixels_per_frame: f64) -> FramePos {
    FramePos(((x_px as f64 + scroll_x) / pixels_per_frame).max(0.0) as u64)
}

/// Calculate trim-start geometry with bounds checking.
///
/// Returns `(new_pos, new_source_offset, new_duration)` for the region.
/// Clamps the new start so it doesn't go past the end or before the source beginning.
pub fn calculate_trim_start(
    original_pos: FramePos,
    original_offset: FramePos,
    original_duration: FramePos,
    new_start: FramePos,
) -> (FramePos, FramePos, FramePos) {
    let original_end = original_pos + original_duration;
    // Don't let start go past the end
    let clamped = new_start.min(original_end.saturating_sub(FramePos(1)));
    // Don't let start go before the original start minus offset
    let clamped = clamped.max(original_pos.saturating_sub(original_offset));
    let delta = clamped.saturating_sub(original_pos);
    let new_offset = original_offset + delta;
    let new_duration = original_duration.saturating_sub(delta).max(FramePos(1));
    (clamped, new_offset, new_duration)
}

/// Calculate trim-end duration from mouse position.
///
/// Returns the new duration, ensuring minimum of 1 frame.
pub fn calculate_trim_end(timeline_pos: FramePos, end_frame: FramePos) -> FramePos {
    end_frame.saturating_sub(timeline_pos).max(FramePos(1))
}

/// Calculate the target track index from a mouse Y position.
///
/// Used for track reorder drag operations.
pub fn row_index_from_y(y_offset: f32, track_height: f32, track_count: usize) -> usize {
    ((y_offset / track_height).floor() as usize).min(track_count.saturating_sub(1))
}

/// Convert a linear gain value to decibels for display.
pub fn gain_to_db_string(gain: f32) -> String {
    if gain < 1e-10 {
        "-inf".to_string()
    } else {
        format!("{:.1}", 20.0 * gain.log10())
    }
}

/// Check if a parameter change is significant (beyond float epsilon).
pub fn parameter_changed(old: f32, new: f32) -> bool {
    (new - old).abs() > f32::EPSILON
}

/// Finalize a recording: create an audio buffer and region from captured samples.
///
/// Returns `Some((recording_id, region))` if `samples` is non-empty, `None` otherwise.
pub fn finalize_recording(
    samples: &[f32],
    channels: u16,
    position: FramePos,
) -> Option<(String, Region)> {
    let frames = samples.len() / channels as usize;
    if frames == 0 {
        return None;
    }
    let recording_id = format!("recording_{}", uuid::Uuid::new_v4());
    let region = Region::new(recording_id.clone(), position, 0u64, frames as u64);
    Some((recording_id, region))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- fast_forward_position ---

    #[test]
    fn fast_forward_120bpm_48k_4_4() {
        // 120 BPM, 48kHz, 4/4 time
        // frames_per_beat = 48000 * 60 / 120 = 24000
        // frames_per_bar = 24000 * 4 = 96000
        let result = fast_forward_position(FramePos(0), 120.0, 48000, 4);
        assert_eq!(result, FramePos(96000));
    }

    #[test]
    fn fast_forward_120bpm_48k_3_4() {
        // 3/4 time: frames_per_bar = 24000 * 3 = 72000
        let result = fast_forward_position(FramePos(0), 120.0, 48000, 3);
        assert_eq!(result, FramePos(72000));
    }

    #[test]
    fn fast_forward_120bpm_48k_6_8() {
        // 6/8 time (time_sig_num=6): frames_per_bar = 24000 * 6 = 144000
        let result = fast_forward_position(FramePos(0), 120.0, 48000, 6);
        assert_eq!(result, FramePos(144000));
    }

    #[test]
    fn fast_forward_from_nonzero_position() {
        let result = fast_forward_position(FramePos(10000), 120.0, 48000, 4);
        assert_eq!(result, FramePos(10000 + 96000));
    }

    #[test]
    fn fast_forward_bpm_zero() {
        // bpm=0 causes divide-by-zero → inf → 0 after cast
        // This is edge-case behavior; the function produces FramePos(current + inf as u64)
        // which in practice wraps. We just verify it doesn't panic.
        let _result = fast_forward_position(FramePos(0), 0.0, 48000, 4);
    }

    // --- pixel_to_frame ---

    #[test]
    fn pixel_to_frame_basic() {
        // 100px at 0.01 ppf, no scroll → frame 10000
        let result = pixel_to_frame(100.0, 0.0, 0.01);
        assert_eq!(result, FramePos(10000));
    }

    #[test]
    fn pixel_to_frame_with_scroll() {
        // 50px + 500.0 scroll at 0.01 ppf → (50 + 500) / 0.01 = 55000
        let result = pixel_to_frame(50.0, 500.0, 0.01);
        assert_eq!(result, FramePos(55000));
    }

    #[test]
    fn pixel_to_frame_negative_clamps_to_zero() {
        // Negative x with no scroll → clamped to 0
        let result = pixel_to_frame(-100.0, 0.0, 0.01);
        assert_eq!(result, FramePos(0));
    }

    #[test]
    fn pixel_to_frame_zero_ppf() {
        // Zero ppf causes inf, which becomes a very large number or wraps.
        // Just verify no panic.
        let _result = pixel_to_frame(100.0, 0.0, 0.0);
    }

    // --- calculate_trim_start ---

    #[test]
    fn trim_start_normal() {
        // Region at pos=100, offset=0, duration=200. Trim to new_start=150.
        // delta=50, new_offset=50, new_duration=150
        let (pos, offset, dur) =
            calculate_trim_start(FramePos(100), FramePos(0), FramePos(200), FramePos(150));
        assert_eq!(pos, FramePos(150));
        assert_eq!(offset, FramePos(50));
        assert_eq!(dur, FramePos(150));
    }

    #[test]
    fn trim_start_clamp_at_end() {
        // Try to trim past the end (pos=100, duration=200, end=300, new_start=400)
        // Clamped to 299
        let (pos, offset, dur) =
            calculate_trim_start(FramePos(100), FramePos(0), FramePos(200), FramePos(400));
        assert_eq!(pos, FramePos(299));
        assert_eq!(offset, FramePos(199));
        assert_eq!(dur, FramePos(1));
    }

    #[test]
    fn trim_start_clamp_at_source_beginning() {
        // Region at pos=100, offset=20, duration=200.
        // Can't go before pos - offset = 80.
        let (pos, offset, dur) =
            calculate_trim_start(FramePos(100), FramePos(20), FramePos(200), FramePos(50));
        assert_eq!(pos, FramePos(80));
        // offset: original(20) + delta(0 since clamped went backwards, but wait:
        // delta = 80 - 100 = saturating_sub → 0
        // Actually, clamped = max(50, 80) = 80, delta = 80.saturating_sub(100) = 0
        assert_eq!(offset, FramePos(20));
        assert_eq!(dur, FramePos(200));
    }

    #[test]
    fn trim_start_minimal_duration() {
        // Trim to exactly the end minus 1
        let (pos, offset, dur) =
            calculate_trim_start(FramePos(0), FramePos(0), FramePos(100), FramePos(99));
        assert_eq!(pos, FramePos(99));
        assert_eq!(offset, FramePos(99));
        assert_eq!(dur, FramePos(1));
    }

    // --- calculate_trim_end ---

    #[test]
    fn trim_end_normal() {
        // Region starts at 100, mouse end at 300 → duration 200
        let dur = calculate_trim_end(FramePos(100), FramePos(300));
        assert_eq!(dur, FramePos(200));
    }

    #[test]
    fn trim_end_minimum_one_frame() {
        // End at same position as start → min 1
        let dur = calculate_trim_end(FramePos(100), FramePos(100));
        assert_eq!(dur, FramePos(1));
    }

    #[test]
    fn trim_end_before_pos() {
        // End before start → saturating sub gives 0, clamped to 1
        let dur = calculate_trim_end(FramePos(200), FramePos(100));
        assert_eq!(dur, FramePos(1));
    }

    // --- row_index_from_y ---

    #[test]
    fn row_index_middle_of_track() {
        // y=90 with height=60 → floor(1.5) = 1
        let idx = row_index_from_y(90.0, 60.0, 5);
        assert_eq!(idx, 1);
    }

    #[test]
    fn row_index_bottom_clamp() {
        // y=1000 with height=60, 3 tracks → floor(16.6)=16, clamped to 2
        let idx = row_index_from_y(1000.0, 60.0, 3);
        assert_eq!(idx, 2);
    }

    #[test]
    fn row_index_zero_tracks() {
        // 0 tracks → saturating_sub(1) = 0, min(anything, 0) = 0
        let idx = row_index_from_y(50.0, 60.0, 0);
        assert_eq!(idx, 0);
    }

    // --- gain_to_db_string ---

    #[test]
    fn gain_unity_is_zero_db() {
        assert_eq!(gain_to_db_string(1.0), "0.0");
    }

    #[test]
    fn gain_silence_is_neg_inf() {
        assert_eq!(gain_to_db_string(0.0), "-inf");
    }

    #[test]
    fn gain_minus_6db() {
        // -6 dB ≈ 0.5012 linear, but exact: 10^(-6/20) = 0.501187...
        let gain = 10f32.powf(-6.0 / 20.0);
        let result = gain_to_db_string(gain);
        assert_eq!(result, "-6.0");
    }

    // --- parameter_changed ---

    #[test]
    fn parameter_identical_values() {
        assert!(!parameter_changed(1.0, 1.0));
    }

    #[test]
    fn parameter_minimal_difference() {
        // Difference exactly at epsilon should NOT be considered changed
        assert!(!parameter_changed(0.0, f32::EPSILON));
    }

    #[test]
    fn parameter_clear_difference() {
        assert!(parameter_changed(0.0, 0.1));
    }

    // --- finalize_recording ---

    #[test]
    fn finalize_recording_non_empty() {
        let samples = vec![0.0f32; 4800]; // 2400 frames at 2 channels
        let result = finalize_recording(&samples, 2, FramePos(1000));
        assert!(result.is_some());
        let (id, region) = result.unwrap();
        assert!(id.starts_with("recording_"));
        assert_eq!(region.timeline_pos, FramePos(1000));
        assert_eq!(region.duration, FramePos(2400));
        assert_eq!(region.source_offset, FramePos(0));
    }

    #[test]
    fn finalize_recording_empty_returns_none() {
        let samples: Vec<f32> = vec![];
        let result = finalize_recording(&samples, 2, FramePos(0));
        assert!(result.is_none());
    }
}
