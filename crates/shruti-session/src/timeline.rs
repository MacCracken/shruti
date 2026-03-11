use crate::audio_pool::AudioPool;
use crate::automation::AutomationTarget;
use crate::region::Region;
use crate::track::{Track, TrackKind};
use crate::transport::Transport;
use shruti_dsp::AudioBuffer;
use shruti_dsp::effects::StereoPanner;

/// The timeline manages multi-track playback and rendering.
pub struct Timeline {
    /// Per-track scratch buffer.
    track_buffer: AudioBuffer,
}

impl Timeline {
    pub fn new(channels: u16, buffer_size: u32) -> Self {
        Self {
            track_buffer: AudioBuffer::new(channels, buffer_size),
        }
    }

    /// Render one buffer of audio from all tracks at the current transport position.
    /// Returns the mixed output in the provided buffer.
    pub fn render(
        &mut self,
        tracks: &[Track],
        transport: &Transport,
        audio_pool: &AudioPool,
        output: &mut AudioBuffer,
    ) {
        let frames = output.frames();
        let channels = output.channels();
        let position = transport.position;

        output.clear();

        // Determine if any track is soloed
        let has_solo = tracks.iter().any(|t| t.solo);

        for track in tracks {
            if track.muted {
                continue;
            }
            if has_solo && !track.solo && track.kind != TrackKind::Master {
                continue;
            }

            self.track_buffer.clear();
            self.render_track(track, position, frames, channels, audio_pool);

            // Apply automation overrides for this buffer position
            let mut gain = track.gain;
            let mut pan = track.pan;
            for lane in &track.automation {
                if !lane.enabled {
                    continue;
                }
                if let Some(value) = lane.value_at(position) {
                    match &lane.target {
                        AutomationTarget::TrackGain => gain = value,
                        AutomationTarget::TrackPan => pan = value,
                        _ => {}
                    }
                }
            }

            // Apply track gain
            self.track_buffer.apply_gain(gain);

            // Apply panning (stereo only)
            if self.track_buffer.channels() >= 2 {
                let mut panner = StereoPanner::new(pan);
                panner.process(&mut self.track_buffer);
            }

            output.mix_from(&self.track_buffer);
        }
    }

    fn render_track(
        &mut self,
        track: &Track,
        position: u64,
        frames: u32,
        channels: u16,
        audio_pool: &AudioPool,
    ) {
        let end = position + frames as u64;
        let active_regions = track.regions_in_range(position, end);

        for region in active_regions {
            if let Some(source) = audio_pool.get(&region.audio_file_id) {
                self.render_region(region, source, position, frames, channels);
            }
        }
    }

    fn render_region(
        &mut self,
        region: &Region,
        source: &AudioBuffer,
        position: u64,
        frames: u32,
        channels: u16,
    ) {
        let src_channels = source.channels().min(channels);

        for frame_offset in 0..frames {
            let timeline_frame = position + frame_offset as u64;

            if let Some(source_frame) = region.source_frame_at(timeline_frame) {
                if source_frame >= source.frames() as u64 {
                    continue;
                }

                let fade_gain = region.fade_gain_at(timeline_frame);

                for ch in 0..src_channels {
                    let sample = source.get(source_frame as u32, ch) * fade_gain;
                    let existing = self.track_buffer.get(frame_offset, ch);
                    self.track_buffer.set(frame_offset, ch, existing + sample);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::Region;

    #[test]
    fn test_timeline_render_single_track() {
        let mut pool = AudioPool::new();
        let source = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3, 0.1, -0.1], 2);
        pool.insert("file1".into(), source);

        let mut track = Track::new_audio("Track 1");
        track.add_region(Region::new("file1".into(), 0, 0, 3));

        let transport = Transport::new(48000);

        let mut timeline = Timeline::new(2, 3);
        let mut output = AudioBuffer::new(2, 3);

        timeline.render(&[track], &transport, &pool, &mut output);

        assert!((output.get(0, 0) - 0.5).abs() < 1e-6);
        assert!((output.get(0, 1) - -0.5).abs() < 1e-6);
        assert!((output.get(2, 0) - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_timeline_render_offset_region() {
        let mut pool = AudioPool::new();
        let source = AudioBuffer::from_interleaved(vec![1.0, 1.0, 0.5, 0.5, 0.25, 0.25], 2);
        pool.insert("file1".into(), source);

        let mut track = Track::new_audio("Track 1");
        // Region starts at frame 2 on timeline, reads from source offset 1
        track.add_region(Region::new("file1".into(), 2, 1, 2));

        let mut transport = Transport::new(48000);
        transport.position = 2;

        let mut timeline = Timeline::new(2, 2);
        let mut output = AudioBuffer::new(2, 2);

        timeline.render(&[track], &transport, &pool, &mut output);

        // Should read source frames 1 and 2 (values 0.5 and 0.25)
        assert!((output.get(0, 0) - 0.5).abs() < 1e-6);
        assert!((output.get(1, 0) - 0.25).abs() < 1e-6);
    }
}
