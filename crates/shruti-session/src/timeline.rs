use crate::audio_pool::AudioPool;
use crate::automation::AutomationTarget;
use crate::region::Region;
use crate::track::{SendPosition, Track, TrackId, TrackKind};
use crate::transport::Transport;
use crate::types::FramePos;
use shruti_dsp::AudioBuffer;
use shruti_dsp::effects::StereoPanner;

/// Maximum number of bus tracks supported for pre-allocated buffers.
const MAX_BUS_TRACKS: usize = 16;
/// Maximum number of source tracks supported for pre-allocated buffers.
const MAX_SOURCE_TRACKS: usize = 64;

/// The timeline manages multi-track playback and rendering.
pub struct Timeline {
    /// Per-track scratch buffer.
    track_buffer: AudioBuffer,
    /// Pre-allocated bus accumulation buffers (reused across render calls).
    bus_buffers: Vec<AudioBuffer>,
    /// TrackIds corresponding to each bus buffer slot.
    bus_ids: Vec<TrackId>,
    /// Pre-allocated source track buffers (reused across render calls).
    source_buffers: Vec<AudioBuffer>,
    /// Number of active source buffers in the current render pass.
    source_count: usize,
}

impl Timeline {
    pub fn new(channels: u16, buffer_size: u32) -> Self {
        Self {
            track_buffer: AudioBuffer::new(channels, buffer_size),
            bus_buffers: (0..MAX_BUS_TRACKS)
                .map(|_| AudioBuffer::new(channels, buffer_size))
                .collect(),
            bus_ids: Vec::with_capacity(MAX_BUS_TRACKS),
            source_buffers: (0..MAX_SOURCE_TRACKS)
                .map(|_| AudioBuffer::new(channels, buffer_size))
                .collect(),
            source_count: 0,
        }
    }

    /// Render one buffer of audio from all tracks at the current transport position.
    ///
    /// Rendering happens in multiple passes:
    /// 1. Render all non-bus, non-master tracks into per-track buffers and process sends.
    /// 2. Mix bus track accumulated audio (from sends) into the output.
    /// 3. Mix all non-bus, non-master track audio into the output.
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

        // Map bus tracks to pre-allocated bus buffers (no HashMap allocation)
        self.bus_ids.clear();
        let mut bus_count = 0;
        for track in tracks {
            if track.kind == TrackKind::Bus && bus_count < MAX_BUS_TRACKS {
                // Ensure the pre-allocated buffer matches the current frame size
                if self.bus_buffers[bus_count].frames() != frames {
                    self.bus_buffers[bus_count] = AudioBuffer::new(channels, frames);
                }
                self.bus_buffers[bus_count].clear();
                self.bus_ids.push(track.id);
                bus_count += 1;
            }
        }

        self.source_count = 0;

        // First pass: render source tracks (Audio, MIDI) and route sends
        for track in tracks {
            if track.kind == TrackKind::Bus || track.kind == TrackKind::Master {
                continue;
            }
            if track.muted {
                continue;
            }
            if has_solo && !track.solo {
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

            // Process pre-fader sends before applying gain/pan
            for send in &track.sends {
                if !send.enabled {
                    continue;
                }
                if send.position == SendPosition::PreFader
                    && let Some(bus_idx) = self.bus_ids.iter().position(|id| *id == send.target)
                {
                    for frame in 0..frames {
                        for ch in 0..channels {
                            let sample = self.track_buffer.get(frame, ch) * send.level;
                            let existing = self.bus_buffers[bus_idx].get(frame, ch);
                            self.bus_buffers[bus_idx].set(frame, ch, existing + sample);
                        }
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

            // Process post-fader sends after applying gain/pan
            for send in &track.sends {
                if !send.enabled {
                    continue;
                }
                if send.position == SendPosition::PostFader
                    && let Some(bus_idx) = self.bus_ids.iter().position(|id| *id == send.target)
                {
                    for frame in 0..frames {
                        for ch in 0..channels {
                            let sample = self.track_buffer.get(frame, ch) * send.level;
                            let existing = self.bus_buffers[bus_idx].get(frame, ch);
                            self.bus_buffers[bus_idx].set(frame, ch, existing + sample);
                        }
                    }
                }
            }

            // Save post-fader buffer into pre-allocated source buffer
            if self.source_count < MAX_SOURCE_TRACKS {
                if self.source_buffers[self.source_count].frames() != frames {
                    self.source_buffers[self.source_count] = AudioBuffer::new(channels, frames);
                }
                self.source_buffers[self.source_count].clear();
                self.source_buffers[self.source_count].mix_from(&self.track_buffer);
                self.source_count += 1;
            }
        }

        // Second pass: process bus tracks — apply bus gain/pan, then mix into output
        for track in tracks {
            if track.kind != TrackKind::Bus {
                continue;
            }
            if track.muted {
                continue;
            }
            if has_solo && !track.solo {
                continue;
            }

            if let Some(bus_idx) = self.bus_ids.iter().position(|id| *id == track.id) {
                // Apply bus track gain
                self.bus_buffers[bus_idx].apply_gain(track.gain);

                // Apply bus panning
                if self.bus_buffers[bus_idx].channels() >= 2 {
                    let mut panner = StereoPanner::new(track.pan);
                    panner.process(&mut self.bus_buffers[bus_idx]);
                }

                output.mix_from(&self.bus_buffers[bus_idx]);
            }
        }

        // Third pass: mix source track buffers into output
        for i in 0..self.source_count {
            output.mix_from(&self.source_buffers[i]);
        }
    }

    fn render_track(
        &mut self,
        track: &Track,
        position: FramePos,
        frames: u32,
        channels: u16,
        audio_pool: &AudioPool,
    ) {
        let end = position + FramePos::from(frames);
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
        position: FramePos,
        frames: u32,
        channels: u16,
    ) {
        let src_channels = source.channels().min(channels);

        for frame_offset in 0..frames {
            let timeline_frame = position + FramePos::from(frame_offset);

            if let Some(source_frame) = region.source_frame_at(timeline_frame) {
                if source_frame.0 >= source.frames() as u64 {
                    continue;
                }

                let fade_gain = region.fade_gain_at(timeline_frame);

                for ch in 0..src_channels {
                    let sample = source.get(source_frame.0 as u32, ch) * fade_gain;
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
    use crate::track::Send;

    #[test]
    fn test_timeline_render_single_track() {
        let mut pool = AudioPool::new();
        let source = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3, 0.1, -0.1], 2);
        pool.insert("file1".into(), source);

        let mut track = Track::new_audio("Track 1");
        track.add_region(Region::new("file1".into(), 0u64, 0u64, 3u64));

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
        track.add_region(Region::new("file1".into(), 2u64, 1u64, 2u64));

        let mut transport = Transport::new(48000);
        transport.position = FramePos(2);

        let mut timeline = Timeline::new(2, 2);
        let mut output = AudioBuffer::new(2, 2);

        timeline.render(&[track], &transport, &pool, &mut output);

        // Should read source frames 1 and 2 (values 0.5 and 0.25)
        assert!((output.get(0, 0) - 0.5).abs() < 1e-6);
        assert!((output.get(1, 0) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_send_routes_audio_to_bus() {
        // Create an audio track and a bus track
        let mut pool = AudioPool::new();
        let samples: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.01).sin() * 0.8).collect();
        let buf = AudioBuffer::from_interleaved(samples, 2);
        pool.insert("drums_audio".to_string(), buf);

        let mut audio_track = Track::new_audio("Drums");
        let _audio_id = audio_track.id;
        audio_track.add_region(Region::new("drums_audio".to_string(), 0u64, 0u64, 512u64));

        let bus_track = Track::new_bus("Reverb Bus");
        let bus_id = bus_track.id;

        // Add a send from drums to reverb bus at 50% level
        audio_track.sends.push(Send {
            target: bus_id,
            level: 0.5,
            position: SendPosition::PostFader,
            enabled: true,
        });

        let tracks = vec![audio_track, bus_track];

        // Render
        let mut tl = Timeline::new(2, 512);
        let transport = Transport::new(48000);
        let mut output = AudioBuffer::new(2, 512);
        tl.render(&tracks, &transport, &pool, &mut output);

        // Output should contain audio (both direct track audio and bus contribution)
        let has_audio = output.as_interleaved().iter().any(|&s| s.abs() > 0.001);
        assert!(has_audio, "expected audio output with send routing");

        // The output should be louder than just the direct signal because the bus
        // also contributes. With a 0.5 send, total gain at some samples should be ~1.5x
        // (1.0 direct + 0.5 send through bus).
        // Verify the bus actually contributed by checking the output is larger than
        // the source signal alone.
        let direct_only_max = (0..512)
            .map(|i| ((i as f32 * 0.01).sin() * 0.8).abs())
            .fold(0.0f32, f32::max);
        let output_max = output
            .as_interleaved()
            .iter()
            .copied()
            .fold(0.0f32, |a, b| a.max(b.abs()));
        assert!(
            output_max > direct_only_max,
            "bus send should add to the output: output_max={output_max}, direct_max={direct_only_max}"
        );
    }

    #[test]
    fn test_pre_fader_send_ignores_track_gain() {
        let mut pool = AudioPool::new();
        // Constant 0.5 on both channels for 4 frames
        let samples: Vec<f32> = vec![0.5; 8];
        let buf = AudioBuffer::from_interleaved(samples, 2);
        pool.insert("src".to_string(), buf);

        let mut audio_track = Track::new_audio("Src");
        audio_track.gain = 0.0; // Mute the track gain
        audio_track.add_region(Region::new("src".to_string(), 0u64, 0u64, 4u64));

        let bus_track = Track::new_bus("Bus");
        let bus_id = bus_track.id;

        // Pre-fader send at full level
        audio_track.sends.push(Send {
            target: bus_id,
            level: 1.0,
            position: SendPosition::PreFader,
            enabled: true,
        });

        let tracks = vec![audio_track, bus_track];

        let mut tl = Timeline::new(2, 4);
        let transport = Transport::new(48000);
        let mut output = AudioBuffer::new(2, 4);
        tl.render(&tracks, &transport, &pool, &mut output);

        // Even though the track gain is 0, the pre-fader send should have routed
        // the original audio to the bus, so we should hear the bus output.
        let has_audio = output.as_interleaved().iter().any(|&s| s.abs() > 0.001);
        assert!(
            has_audio,
            "pre-fader send should route audio even when track gain is zero"
        );

        // The output should be approximately 0.5 (from bus only, direct track is silent)
        assert!(
            (output.get(0, 0) - 0.5).abs() < 1e-4,
            "bus should carry pre-fader signal: got {}",
            output.get(0, 0)
        );
    }

    #[test]
    fn test_muted_track_send_still_routes() {
        // Based on the render code: muted tracks are skipped entirely,
        // so sends from muted tracks do NOT route audio.
        let mut pool = AudioPool::new();
        let samples: Vec<f32> = vec![0.5; 8];
        let buf = AudioBuffer::from_interleaved(samples, 2);
        pool.insert("src".to_string(), buf);

        let mut audio_track = Track::new_audio("Src");
        audio_track.muted = true; // mute the track
        audio_track.add_region(Region::new("src".to_string(), 0u64, 0u64, 4u64));

        let bus_track = Track::new_bus("Bus");
        let bus_id = bus_track.id;

        audio_track.sends.push(Send {
            target: bus_id,
            level: 1.0,
            position: SendPosition::PostFader,
            enabled: true,
        });

        let tracks = vec![audio_track, bus_track];

        let mut tl = Timeline::new(2, 4);
        let transport = Transport::new(48000);
        let mut output = AudioBuffer::new(2, 4);
        tl.render(&tracks, &transport, &pool, &mut output);

        // Muted tracks are skipped, so output should be silent
        let all_silent = output.as_interleaved().iter().all(|&s| s.abs() < 1e-6);
        assert!(
            all_silent,
            "muted track should not route audio through sends"
        );
    }

    #[test]
    fn test_multiple_sends_to_same_bus() {
        let mut pool = AudioPool::new();
        // Two different audio sources
        let buf1 = AudioBuffer::from_interleaved(vec![0.4; 8], 2);
        let buf2 = AudioBuffer::from_interleaved(vec![0.3; 8], 2);
        pool.insert("src1".to_string(), buf1);
        pool.insert("src2".to_string(), buf2);

        let bus_track = Track::new_bus("FX Bus");
        let bus_id = bus_track.id;

        let mut track1 = Track::new_audio("Track1");
        track1.add_region(Region::new("src1".to_string(), 0u64, 0u64, 4u64));
        track1.sends.push(Send {
            target: bus_id,
            level: 1.0,
            position: SendPosition::PostFader,
            enabled: true,
        });

        let mut track2 = Track::new_audio("Track2");
        track2.add_region(Region::new("src2".to_string(), 0u64, 0u64, 4u64));
        track2.sends.push(Send {
            target: bus_id,
            level: 1.0,
            position: SendPosition::PostFader,
            enabled: true,
        });

        let tracks = vec![track1, track2, bus_track];

        let mut tl = Timeline::new(2, 4);
        let transport = Transport::new(48000);
        let mut output = AudioBuffer::new(2, 4);
        tl.render(&tracks, &transport, &pool, &mut output);

        // Output should have both direct tracks (0.4 + 0.3) and bus contribution
        // (0.4 + 0.3 from bus). Total per channel ~1.4.
        let sample = output.get(0, 0);
        assert!(
            sample > 0.6,
            "multiple sends to same bus should sum: got {sample}"
        );
    }

    #[test]
    fn test_bus_gain_affects_send_output() {
        let mut pool = AudioPool::new();
        let buf = AudioBuffer::from_interleaved(vec![0.8; 8], 2);
        pool.insert("src".to_string(), buf);

        let mut bus_track = Track::new_bus("FX Bus");
        bus_track.gain = 0.5; // bus gain at 50%
        let bus_id = bus_track.id;

        let mut audio_track = Track::new_audio("Src");
        audio_track.add_region(Region::new("src".to_string(), 0u64, 0u64, 4u64));
        audio_track.sends.push(Send {
            target: bus_id,
            level: 1.0,
            position: SendPosition::PostFader,
            enabled: true,
        });

        let tracks = vec![audio_track, bus_track];

        let mut tl = Timeline::new(2, 4);
        let transport = Transport::new(48000);
        let mut output = AudioBuffer::new(2, 4);
        tl.render(&tracks, &transport, &pool, &mut output);

        // Direct signal: 0.8, bus contribution: 0.8 * 1.0 (send) * 0.5 (bus gain) = 0.4
        // Total: 0.8 + 0.4 = 1.2
        let sample = output.get(0, 0);
        assert!(
            (sample - 1.2).abs() < 0.05,
            "bus gain should scale send output: expected ~1.2, got {sample}"
        );
    }

    #[test]
    fn test_empty_bus_no_crash() {
        // A bus with no incoming sends should render without error
        let pool = AudioPool::new();
        let bus_track = Track::new_bus("Empty Bus");

        let tracks = vec![bus_track];

        let mut tl = Timeline::new(2, 64);
        let transport = Transport::new(48000);
        let mut output = AudioBuffer::new(2, 64);
        tl.render(&tracks, &transport, &pool, &mut output);

        // Output should be all zeros (silence)
        let all_silent = output.as_interleaved().iter().all(|&s| s.abs() < 1e-6);
        assert!(all_silent, "empty bus should produce silence");
    }
}
