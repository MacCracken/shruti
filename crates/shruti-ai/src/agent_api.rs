use serde::{Deserialize, Serialize};
use shruti_dsp::AudioFormat;
use shruti_dsp::io::{BitDepth, ExportConfig, ExportFormat, write_audio_file, write_wav_file};
use shruti_session::edit::EditCommand;
use shruti_session::region::Region;
use shruti_session::session::Session;
use shruti_session::store::SessionStore;
use shruti_session::undo::UndoManager;
use std::path::Path;

/// Structured API for AI agents to control Shruti.
///
/// All methods accept and return serializable types suitable for
/// JSON-based protocols (MCP, REST, IPC).
pub struct AgentApi {
    session: Option<Session>,
    undo: UndoManager,
    store: Option<SessionStore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ApiResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn ok_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

impl AgentApi {
    pub fn new() -> Self {
        Self {
            session: None,
            undo: UndoManager::default(),
            store: None,
        }
    }

    // --- Session Control ---

    pub fn create_session(&mut self, name: &str, sample_rate: u32, buffer_size: u32) -> ApiResult {
        self.session = Some(Session::new(name, sample_rate, buffer_size));
        ApiResult::ok(format!("session '{name}' created"))
    }

    pub fn open_session(&mut self, path: &str) -> ApiResult {
        match SessionStore::open(Path::new(path)) {
            Ok((store, session)) => {
                self.session = Some(session);
                self.store = Some(store);
                ApiResult::ok(format!("session opened from '{path}'"))
            }
            Err(e) => ApiResult::err(format!("failed to open session: {e}")),
        }
    }

    pub fn save_session(&self, path: &str) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        match SessionStore::create(Path::new(path), session) {
            Ok(_) => ApiResult::ok(format!("session saved to '{path}'")),
            Err(e) => ApiResult::err(format!("failed to save session: {e}")),
        }
    }

    pub fn session_info(&self) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        ApiResult::ok_with_data(
            "session info",
            serde_json::json!({
                "name": session.name,
                "sample_rate": session.sample_rate,
                "buffer_size": session.buffer_size,
                "track_count": session.track_count(),
                "length_frames": session.session_length(),
                "bpm": session.transport.bpm,
                "time_sig": format!("{}/{}", session.transport.time_sig_num, session.transport.time_sig_den),
            }),
        )
    }

    // --- Track Management ---

    pub fn add_track(&mut self, name: &str, kind: &str) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let id = match kind {
            "audio" => session.add_audio_track(name),
            "midi" => session.add_midi_track(name),
            "bus" => session.add_bus_track(name),
            _ => return ApiResult::err(format!("unknown track kind: {kind}")),
        };

        ApiResult::ok_with_data(
            format!("{kind} track '{name}' added"),
            serde_json::json!({ "track_id": id.0.to_string() }),
        )
    }

    pub fn list_tracks(&self) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let tracks: Vec<serde_json::Value> = session
            .tracks
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id.0.to_string(),
                    "name": t.name,
                    "kind": format!("{:?}", t.kind),
                    "gain": t.gain,
                    "pan": t.pan,
                    "muted": t.muted,
                    "solo": t.solo,
                    "regions": t.regions.len(),
                })
            })
            .collect();

        ApiResult::ok_with_data("tracks", serde_json::json!({ "tracks": tracks }))
    }

    pub fn set_track_gain(&mut self, track_name: &str, gain: f32) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let track = match session.tracks.iter_mut().find(|t| t.name == track_name) {
            Some(t) => t,
            None => return ApiResult::err(format!("track '{track_name}' not found")),
        };

        let old_gain = track.gain;
        track.gain = gain;

        self.undo.execute(
            EditCommand::SetTrackGain {
                track_id: track.id,
                old_gain,
                new_gain: gain,
            },
            session,
        );

        ApiResult::ok(format!("track '{track_name}' gain set to {gain}"))
    }

    pub fn set_track_pan(&mut self, track_name: &str, pan: f32) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let track = match session.tracks.iter_mut().find(|t| t.name == track_name) {
            Some(t) => t,
            None => return ApiResult::err(format!("track '{track_name}' not found")),
        };

        let old_pan = track.pan;
        track.pan = pan;

        self.undo.execute(
            EditCommand::SetTrackPan {
                track_id: track.id,
                old_pan,
                new_pan: pan,
            },
            session,
        );

        ApiResult::ok(format!("track '{track_name}' pan set to {pan}"))
    }

    pub fn mute_track(&mut self, track_name: &str) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let track = match session.tracks.iter_mut().find(|t| t.name == track_name) {
            Some(t) => t,
            None => return ApiResult::err(format!("track '{track_name}' not found")),
        };

        track.muted = !track.muted;
        let state = if track.muted { "muted" } else { "unmuted" };

        ApiResult::ok(format!("track '{track_name}' {state}"))
    }

    pub fn solo_track(&mut self, track_name: &str) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let track = match session.tracks.iter_mut().find(|t| t.name == track_name) {
            Some(t) => t,
            None => return ApiResult::err(format!("track '{track_name}' not found")),
        };

        track.solo = !track.solo;
        let state = if track.solo { "soloed" } else { "unsoloed" };

        ApiResult::ok(format!("track '{track_name}' {state}"))
    }

    // --- Region Management ---

    pub fn add_region(&mut self, track_name: &str, audio_file: &str, position: u64) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        // Load the audio file into the pool if not already present
        let file_id = if session.audio_pool.get(audio_file).is_some() {
            audio_file.to_string()
        } else {
            match session.audio_pool.load(Path::new(audio_file)) {
                Ok(id) => id,
                Err(e) => return ApiResult::err(format!("failed to load audio: {e}")),
            }
        };

        let duration = session
            .audio_pool
            .get(&file_id)
            .map(|b| b.frames() as u64)
            .unwrap_or(0);

        let region = Region::new(file_id, position, 0, duration);
        let region_id = region.id;

        let track = match session.tracks.iter_mut().find(|t| t.name == track_name) {
            Some(t) => t,
            None => return ApiResult::err(format!("track '{track_name}' not found")),
        };

        track.add_region(region);

        ApiResult::ok_with_data(
            format!("region added to '{track_name}' at frame {position}"),
            serde_json::json!({ "region_id": region_id.0.to_string() }),
        )
    }

    // --- Transport Control ---

    pub fn transport(&mut self, action: &str) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        match action {
            "play" => {
                session.transport.play();
                ApiResult::ok("playing")
            }
            "stop" => {
                session.transport.stop();
                ApiResult::ok("stopped")
            }
            "pause" => {
                session.transport.pause();
                ApiResult::ok("paused")
            }
            _ => ApiResult::err(format!("unknown transport action: {action}")),
        }
    }

    pub fn seek(&mut self, position: u64) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        session.transport.seek(position);
        ApiResult::ok(format!("seeked to frame {position}"))
    }

    pub fn set_tempo(&mut self, bpm: f64) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        session.transport.bpm = bpm;
        ApiResult::ok(format!("tempo set to {bpm} BPM"))
    }

    // --- Export ---

    pub fn export_wav(&self, path: &str) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let length = session.session_length();
        if length == 0 {
            return ApiResult::err("session is empty");
        }

        let channels = 2u16;
        let mut output = shruti_dsp::AudioBuffer::new(channels, length as u32);

        // Render the full session through the timeline
        if session.timeline.is_some() {
            let mut tl = shruti_session::Timeline::new(channels, length as u32);
            tl.render(
                &session.tracks,
                &session.transport,
                &session.audio_pool,
                &mut output,
            );
        }

        let format = AudioFormat::new(session.sample_rate, channels, 0);
        match write_wav_file(Path::new(path), &output, &format) {
            Ok(()) => ApiResult::ok(format!("exported to '{path}'")),
            Err(e) => ApiResult::err(format!("export failed: {e}")),
        }
    }

    /// Export the session to an audio file with configurable format and bit depth.
    ///
    /// `format` should be `"wav"` or `"flac"`.
    /// `bit_depth` should be `"16"`, `"24"`, or `"32"`.
    pub fn export_audio(&self, path: &str, format: &str, bit_depth: &str) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let length = session.session_length();
        if length == 0 {
            return ApiResult::err("session is empty");
        }

        let export_format = match format {
            "wav" => ExportFormat::Wav,
            "flac" => ExportFormat::Flac,
            _ => return ApiResult::err(format!("unsupported format: {format}")),
        };

        let export_bit_depth = match bit_depth {
            "16" => BitDepth::Int16,
            "24" => BitDepth::Int24,
            "32" => BitDepth::Float32,
            _ => return ApiResult::err(format!("unsupported bit depth: {bit_depth}")),
        };

        let channels = 2u16;
        let mut output = shruti_dsp::AudioBuffer::new(channels, length as u32);

        if session.timeline.is_some() {
            let mut tl = shruti_session::Timeline::new(channels, length as u32);
            tl.render(
                &session.tracks,
                &session.transport,
                &session.audio_pool,
                &mut output,
            );
        }

        let config = ExportConfig {
            format: export_format,
            bit_depth: export_bit_depth,
            sample_rate: session.sample_rate,
            channels,
        };

        match write_audio_file(Path::new(path), &output, &config) {
            Ok(()) => ApiResult::ok(format!("exported to '{path}' ({format}, {bit_depth}-bit)")),
            Err(e) => ApiResult::err(format!("export failed: {e}")),
        }
    }

    // --- Analysis & Auto-Mix ---

    /// Analyze the frequency spectrum of a track's audio.
    pub fn analyze_spectrum(&self, track_name: &str, fft_size: usize) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let track = match session.tracks.iter().find(|t| t.name == track_name) {
            Some(t) => t,
            None => return ApiResult::err(format!("track '{}' not found", track_name)),
        };

        if track.regions.is_empty() {
            return ApiResult::err("track has no audio regions");
        }

        // Render the track's first region to a buffer for analysis
        let region = &track.regions[0];
        if let Some(source) = session.audio_pool.get(&region.audio_file_id) {
            let samples = source.as_interleaved();
            let channels = source.channels();
            let buf = shruti_dsp::AudioBuffer::from_interleaved(samples.to_vec(), channels);

            let analysis = shruti_dsp::analyze_spectrum(&buf, 0, session.sample_rate, fft_size);

            let data = serde_json::json!({
                "peak_frequency_hz": analysis.peak_frequency,
                "peak_magnitude_db": analysis.peak_magnitude_db,
                "spectral_centroid_hz": analysis.spectral_centroid,
                "spectral_rolloff_hz": analysis.spectral_rolloff,
                "frequency_resolution_hz": analysis.frequency_resolution,
                "fft_size": analysis.fft_size,
                "num_bins": analysis.magnitude_db.len(),
            });

            ApiResult::ok_with_data("spectral analysis complete", data)
        } else {
            ApiResult::err("audio source not found in pool")
        }
    }

    /// Analyze the dynamics of a track's audio.
    pub fn analyze_dynamics(&self, track_name: &str) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let track = match session.tracks.iter().find(|t| t.name == track_name) {
            Some(t) => t,
            None => return ApiResult::err(format!("track '{}' not found", track_name)),
        };

        if track.regions.is_empty() {
            return ApiResult::err("track has no audio regions");
        }

        let region = &track.regions[0];
        if let Some(source) = session.audio_pool.get(&region.audio_file_id) {
            let samples = source.as_interleaved();
            let channels = source.channels();
            let buf = shruti_dsp::AudioBuffer::from_interleaved(samples.to_vec(), channels);

            let analysis = shruti_dsp::analyze_dynamics(&buf, session.sample_rate);

            let data = serde_json::json!({
                "channels": analysis.channel_count,
                "frames": analysis.frame_count,
                "peak_db": analysis.peak_db,
                "rms_db": analysis.rms_db,
                "true_peak_db": analysis.true_peak_db,
                "crest_factor_db": analysis.crest_factor_db,
                "lufs": analysis.lufs,
                "dynamic_range_db": analysis.dynamic_range_db,
            });

            ApiResult::ok_with_data("dynamics analysis complete", data)
        } else {
            ApiResult::err("audio source not found in pool")
        }
    }

    /// Generate auto-mix suggestions based on track analysis.
    /// Returns gain, pan, and EQ recommendations for each track.
    pub fn auto_mix_suggest(&self) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let audio_tracks: Vec<&shruti_session::track::Track> = session
            .tracks
            .iter()
            .filter(|t| t.kind == shruti_session::track::TrackKind::Audio && !t.regions.is_empty())
            .collect();

        if audio_tracks.is_empty() {
            return ApiResult::err("no audio tracks with regions to analyze");
        }

        let mut suggestions = Vec::new();
        let track_count = audio_tracks.len();

        for (i, track) in audio_tracks.iter().enumerate() {
            let region = &track.regions[0];
            let (peak_db, rms_db, centroid) =
                if let Some(source) = session.audio_pool.get(&region.audio_file_id) {
                    let samples = source.as_interleaved();
                    let channels = source.channels();
                    let buf = shruti_dsp::AudioBuffer::from_interleaved(samples.to_vec(), channels);

                    let dyn_analysis = shruti_dsp::analyze_dynamics(&buf, session.sample_rate);
                    let spec_analysis =
                        shruti_dsp::analyze_spectrum(&buf, 0, session.sample_rate, 4096);

                    let avg_peak = dyn_analysis.peak_db.iter().sum::<f32>()
                        / dyn_analysis.peak_db.len() as f32;
                    let avg_rms =
                        dyn_analysis.rms_db.iter().sum::<f32>() / dyn_analysis.rms_db.len() as f32;
                    (avg_peak, avg_rms, spec_analysis.spectral_centroid)
                } else {
                    continue;
                };

            // Gain staging: target -18 dBFS RMS for headroom
            let target_rms = -18.0f32;
            let suggested_gain_db = target_rms - rms_db;

            // Pan suggestion: spread tracks across stereo field
            let suggested_pan = if track_count <= 1 {
                0.0f32
            } else {
                // Spread evenly from -0.7 to 0.7 (leaving center for bass/vocals)
                let spread = 1.4;
                -0.7 + (i as f32 / (track_count - 1).max(1) as f32) * spread
            };

            // EQ suggestion based on spectral centroid
            let eq_suggestion = if centroid < 300.0 {
                "low-frequency dominant — consider high-shelf boost around 3kHz for clarity"
            } else if centroid > 4000.0 {
                "high-frequency dominant — consider low-shelf boost around 200Hz for warmth"
            } else {
                "balanced spectrum — minimal EQ needed"
            };

            suggestions.push(serde_json::json!({
                "track": track.name,
                "current_peak_db": peak_db,
                "current_rms_db": rms_db,
                "spectral_centroid_hz": centroid,
                "suggested_gain_db": suggested_gain_db,
                "suggested_pan": suggested_pan,
                "eq_suggestion": eq_suggestion,
            }));
        }

        ApiResult::ok_with_data(
            format!("auto-mix suggestions for {} tracks", suggestions.len()),
            serde_json::json!({ "suggestions": suggestions }),
        )
    }

    /// Suggest arrangement changes based on session structure.
    pub fn composition_suggest(&self) -> ApiResult {
        let session = match &self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        let track_count = session.tracks.len();
        let audio_tracks = session.audio_tracks().len();
        let midi_tracks = session.midi_tracks().len();
        let length = session.session_length();
        let bpm = session.transport.bpm;
        let frames_per_bar = (session.sample_rate as f64 * 60.0 / bpm) * 4.0;
        let bars = if frames_per_bar > 0.0 {
            (length as f64 / frames_per_bar).ceil() as u64
        } else {
            0
        };

        let mut suggestions = Vec::new();

        // Structure suggestions
        if bars < 8 {
            suggestions.push("Session is very short. Consider extending to at least 16 bars for a basic song structure (intro + verse).");
        }
        if (8..32).contains(&bars) {
            suggestions.push("Consider adding sections: typical pop structure is intro (4-8 bars), verse (8-16 bars), chorus (8-16 bars), bridge (4-8 bars).");
        }

        // Instrumentation suggestions
        if audio_tracks > 0 && midi_tracks == 0 {
            suggestions.push(
                "No MIDI tracks. Consider adding a MIDI track for synthesizer or drum machine parts.",
            );
        }
        if midi_tracks > 0 && audio_tracks == 0 {
            suggestions.push(
                "No audio tracks. Consider recording live instruments or vocals for a richer mix.",
            );
        }
        if track_count <= 2 {
            suggestions.push(
                "Few tracks. Most productions benefit from separation: drums, bass, harmony, melody, vocals.",
            );
        }

        // Tempo suggestions
        if bpm < 60.0 {
            suggestions.push("Very slow tempo. Consider if this suits the genre — typical ranges: ballad (60-80), pop (100-130), dance (120-150), drum & bass (160-180).");
        }

        let data = serde_json::json!({
            "session_name": session.name,
            "bars": bars,
            "bpm": bpm,
            "track_count": track_count,
            "audio_tracks": audio_tracks,
            "midi_tracks": midi_tracks,
            "length_frames": length,
            "suggestions": suggestions,
        });

        ApiResult::ok_with_data(
            format!("{} composition suggestions", suggestions.len()),
            data,
        )
    }

    // --- Undo/Redo ---

    pub fn undo(&mut self) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        if self.undo.undo(session) {
            ApiResult::ok("undone")
        } else {
            ApiResult::err("nothing to undo")
        }
    }

    pub fn redo(&mut self) -> ApiResult {
        let session = match &mut self.session {
            Some(s) => s,
            None => return ApiResult::err("no active session"),
        };

        if self.undo.redo(session) {
            ApiResult::ok("redone")
        } else {
            ApiResult::err("nothing to redo")
        }
    }
}

impl Default for AgentApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_session_lifecycle() {
        let mut api = AgentApi::new();

        let r = api.create_session("Test Song", 48000, 256);
        assert!(r.success);

        let r = api.session_info();
        assert!(r.success);
        let data = r.data.unwrap();
        assert_eq!(data["name"], "Test Song");
        assert_eq!(data["sample_rate"], 48000);

        let r = api.add_track("Guitar", "audio");
        assert!(r.success);

        let r = api.add_track("Reverb Bus", "bus");
        assert!(r.success);

        let r = api.list_tracks();
        assert!(r.success);
        let tracks = r.data.unwrap()["tracks"].as_array().unwrap().len();
        assert_eq!(tracks, 3); // Guitar + Reverb Bus + Master
    }

    #[test]
    fn test_agent_transport() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);

        assert!(api.transport("play").success);
        assert!(api.transport("pause").success);
        assert!(api.transport("stop").success);
        assert!(!api.transport("invalid").success);

        assert!(api.set_tempo(140.0).success);
        assert!(api.seek(48000).success);
    }

    #[test]
    fn test_agent_track_controls() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Vocals", "audio");

        assert!(api.set_track_gain("Vocals", 0.8).success);
        assert!(api.set_track_pan("Vocals", -0.5).success);
        assert!(api.mute_track("Vocals").success);
        assert!(api.solo_track("Vocals").success);

        assert!(!api.set_track_gain("NonExistent", 1.0).success);
    }

    #[test]
    fn test_agent_export_audio() {
        let mut api = AgentApi::new();
        api.create_session("Export Test", 48000, 256);

        // Empty session should fail
        let r = api.export_audio("/tmp/shruti_test_export.wav", "wav", "16");
        assert!(!r.success);
        assert_eq!(r.message, "session is empty");

        // Invalid format
        let r = api.export_audio("/tmp/shruti_test_export.ogg", "ogg", "16");
        assert!(!r.success);

        // Invalid bit depth
        let r = api.export_audio("/tmp/shruti_test_export.wav", "wav", "8");
        assert!(!r.success);

        // No session
        let api2 = AgentApi::new();
        let r = api2.export_audio("/tmp/shruti_test_export.wav", "wav", "16");
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_agent_no_session() {
        let mut api = AgentApi::new();
        assert!(!api.list_tracks().success);
        assert!(!api.transport("play").success);
        assert!(!api.undo().success);
    }

    #[test]
    fn test_auto_mix_no_session() {
        let api = AgentApi::new();
        let result = api.auto_mix_suggest();
        assert!(!result.success);
    }

    #[test]
    fn test_composition_suggest() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 512);
        let result = api.composition_suggest();
        assert!(result.success);
        let data = result.data.unwrap();
        assert!(data["suggestions"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_analyze_spectrum_no_session() {
        let api = AgentApi::new();
        let result = api.analyze_spectrum("track1", 4096);
        assert!(!result.success);
    }

    #[test]
    fn test_analyze_dynamics_no_session() {
        let api = AgentApi::new();
        let result = api.analyze_dynamics("track1");
        assert!(!result.success);
    }

    /// Helper: create an AgentApi with a session that has an audio track with a region
    /// backed by actual audio data in the pool.
    fn api_with_audio_track() -> AgentApi {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Drums", "audio");

        // Insert a synthetic audio buffer into the pool
        let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin() * 0.5).collect();
        let buf = shruti_dsp::AudioBuffer::from_interleaved(samples, 1);
        let session = api.session.as_mut().unwrap();
        session.audio_pool.insert("test_audio".to_string(), buf);

        // Add a region referencing that audio to the Drums track
        let region = Region::new("test_audio".to_string(), 0, 0, 4096);
        let track = session
            .tracks
            .iter_mut()
            .find(|t| t.name == "Drums")
            .unwrap();
        track.add_region(region);

        api
    }

    // --- analyze_spectrum ---

    #[test]
    fn test_analyze_spectrum_track_not_found() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.analyze_spectrum("NonExistent", 4096);
        assert!(!r.success);
        assert!(r.message.contains("not found"));
    }

    #[test]
    fn test_analyze_spectrum_empty_regions() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Vocals", "audio");
        let r = api.analyze_spectrum("Vocals", 4096);
        assert!(!r.success);
        assert_eq!(r.message, "track has no audio regions");
    }

    #[test]
    fn test_analyze_spectrum_success() {
        let api = api_with_audio_track();
        let r = api.analyze_spectrum("Drums", 1024);
        assert!(r.success);
        let data = r.data.unwrap();
        assert!(data["peak_frequency_hz"].is_number());
        assert!(data["peak_magnitude_db"].is_number());
        assert!(data["spectral_centroid_hz"].is_number());
        assert!(data["spectral_rolloff_hz"].is_number());
        assert!(data["fft_size"].is_number());
        assert!(data["num_bins"].is_number());
    }

    #[test]
    fn test_analyze_spectrum_audio_not_in_pool() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Vocals", "audio");
        // Add region referencing non-existent audio
        let session = api.session.as_mut().unwrap();
        let region = Region::new("missing_audio".to_string(), 0, 0, 1000);
        session
            .tracks
            .iter_mut()
            .find(|t| t.name == "Vocals")
            .unwrap()
            .add_region(region);
        let r = api.analyze_spectrum("Vocals", 4096);
        assert!(!r.success);
        assert_eq!(r.message, "audio source not found in pool");
    }

    // --- analyze_dynamics ---

    #[test]
    fn test_analyze_dynamics_track_not_found() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.analyze_dynamics("NonExistent");
        assert!(!r.success);
        assert!(r.message.contains("not found"));
    }

    #[test]
    fn test_analyze_dynamics_empty_regions() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Vocals", "audio");
        let r = api.analyze_dynamics("Vocals");
        assert!(!r.success);
        assert_eq!(r.message, "track has no audio regions");
    }

    #[test]
    fn test_analyze_dynamics_success() {
        let api = api_with_audio_track();
        let r = api.analyze_dynamics("Drums");
        assert!(r.success);
        let data = r.data.unwrap();
        assert!(data["peak_db"].is_array());
        assert!(data["rms_db"].is_array());
        assert!(data["lufs"].is_number());
        assert!(data["dynamic_range_db"].is_number());
    }

    #[test]
    fn test_analyze_dynamics_audio_not_in_pool() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Vocals", "audio");
        let session = api.session.as_mut().unwrap();
        let region = Region::new("missing_audio".to_string(), 0, 0, 1000);
        session
            .tracks
            .iter_mut()
            .find(|t| t.name == "Vocals")
            .unwrap()
            .add_region(region);
        let r = api.analyze_dynamics("Vocals");
        assert!(!r.success);
        assert_eq!(r.message, "audio source not found in pool");
    }

    // --- auto_mix_suggest ---

    #[test]
    fn test_auto_mix_no_audio_tracks() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Synth", "midi");
        let r = api.auto_mix_suggest();
        assert!(!r.success);
        assert!(r.message.contains("no audio tracks"));
    }

    #[test]
    fn test_auto_mix_audio_tracks_no_regions() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Guitar", "audio");
        let r = api.auto_mix_suggest();
        assert!(!r.success);
        assert!(r.message.contains("no audio tracks"));
    }

    #[test]
    fn test_auto_mix_single_track() {
        let api = api_with_audio_track();
        let r = api.auto_mix_suggest();
        assert!(r.success);
        let data = r.data.unwrap();
        let suggestions = data["suggestions"].as_array().unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0]["track"], "Drums");
        // Single track should be panned center
        let pan = suggestions[0]["suggested_pan"].as_f64().unwrap();
        assert!((pan - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_auto_mix_multiple_tracks() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);

        // Add two audio tracks with regions
        for name in &["Guitar", "Bass"] {
            api.add_track(name, "audio");
            let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin() * 0.5).collect();
            let buf = shruti_dsp::AudioBuffer::from_interleaved(samples, 1);
            let pool_id = format!("audio_{name}");
            let session = api.session.as_mut().unwrap();
            session.audio_pool.insert(pool_id.clone(), buf);
            let region = Region::new(pool_id, 0, 0, 4096);
            session
                .tracks
                .iter_mut()
                .find(|t| t.name == *name)
                .unwrap()
                .add_region(region);
        }

        let r = api.auto_mix_suggest();
        assert!(r.success);
        let data = r.data.unwrap();
        let suggestions = data["suggestions"].as_array().unwrap();
        assert_eq!(suggestions.len(), 2);
        // Two tracks should have different pan positions
        let pan0 = suggestions[0]["suggested_pan"].as_f64().unwrap();
        let pan1 = suggestions[1]["suggested_pan"].as_f64().unwrap();
        assert!((pan0 - pan1).abs() > 0.1);
    }

    // --- composition_suggest ---

    #[test]
    fn test_composition_suggest_no_session() {
        let api = AgentApi::new();
        let r = api.composition_suggest();
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_composition_suggest_audio_only() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Guitar", "audio");
        let r = api.composition_suggest();
        assert!(r.success);
        let data = r.data.unwrap();
        let suggestions = data["suggestions"].as_array().unwrap();
        // Should mention no MIDI tracks and few tracks
        let text: String = suggestions
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert!(text.contains("MIDI"));
        assert!(text.contains("Few tracks"));
    }

    #[test]
    fn test_composition_suggest_midi_only() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Synth", "midi");
        let r = api.composition_suggest();
        assert!(r.success);
        let data = r.data.unwrap();
        let suggestions = data["suggestions"].as_array().unwrap();
        let text: String = suggestions
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert!(text.contains("No audio tracks"));
    }

    #[test]
    fn test_composition_suggest_slow_tempo() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.set_tempo(50.0);
        let r = api.composition_suggest();
        assert!(r.success);
        let data = r.data.unwrap();
        let suggestions = data["suggestions"].as_array().unwrap();
        let text: String = suggestions
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert!(text.contains("Very slow tempo"));
    }

    #[test]
    fn test_composition_suggest_many_tracks() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        for i in 0..5 {
            api.add_track(&format!("Track{i}"), "audio");
        }
        let r = api.composition_suggest();
        assert!(r.success);
        let data = r.data.unwrap();
        // With 5+ audio tracks, should NOT have "Few tracks" suggestion
        let suggestions = data["suggestions"].as_array().unwrap();
        let text: String = suggestions
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert!(!text.contains("Few tracks"));
    }

    #[test]
    fn test_composition_suggest_medium_length() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Lead", "audio");
        // Add a region that makes the session ~20 bars long at 120 BPM
        // 1 bar at 120 BPM, 48kHz = 48000 * 60 / 120 * 4 = 96000 frames
        // 20 bars = 1920000 frames
        let session = api.session.as_mut().unwrap();
        let samples: Vec<f32> = vec![0.1; 1_920_000];
        let buf = shruti_dsp::AudioBuffer::from_interleaved(samples, 1);
        session.audio_pool.insert("long_audio".to_string(), buf);
        let region = Region::new("long_audio".to_string(), 0, 0, 1_920_000);
        session
            .tracks
            .iter_mut()
            .find(|t| t.name == "Lead")
            .unwrap()
            .add_region(region);

        let r = api.composition_suggest();
        assert!(r.success);
        let data = r.data.unwrap();
        let suggestions = data["suggestions"].as_array().unwrap();
        let text: String = suggestions
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert!(text.contains("Consider adding sections"));
    }

    // --- Error paths for set_track_pan, mute_track, solo_track ---

    #[test]
    fn test_set_track_pan_no_session() {
        let mut api = AgentApi::new();
        let r = api.set_track_pan("Vocals", 0.5);
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_set_track_pan_not_found() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.set_track_pan("Ghost", 0.5);
        assert!(!r.success);
        assert!(r.message.contains("not found"));
    }

    #[test]
    fn test_set_track_gain_no_session() {
        let mut api = AgentApi::new();
        let r = api.set_track_gain("Vocals", 0.5);
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_mute_track_no_session() {
        let mut api = AgentApi::new();
        let r = api.mute_track("Vocals");
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_mute_track_not_found() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.mute_track("Ghost");
        assert!(!r.success);
        assert!(r.message.contains("not found"));
    }

    #[test]
    fn test_mute_track_toggle() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Drums", "audio");
        let r = api.mute_track("Drums");
        assert!(r.success);
        assert!(r.message.contains("muted"));
        let r = api.mute_track("Drums");
        assert!(r.success);
        assert!(r.message.contains("unmuted"));
    }

    #[test]
    fn test_solo_track_no_session() {
        let mut api = AgentApi::new();
        let r = api.solo_track("Vocals");
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_solo_track_not_found() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.solo_track("Ghost");
        assert!(!r.success);
        assert!(r.message.contains("not found"));
    }

    #[test]
    fn test_solo_track_toggle() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Drums", "audio");
        let r = api.solo_track("Drums");
        assert!(r.success);
        assert!(r.message.contains("soloed"));
        let r = api.solo_track("Drums");
        assert!(r.success);
        assert!(r.message.contains("unsoloed"));
    }

    // --- export_wav ---

    #[test]
    fn test_export_wav_no_session() {
        let api = AgentApi::new();
        let r = api.export_wav("/tmp/test.wav");
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_export_wav_empty_session() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.export_wav("/tmp/test.wav");
        assert!(!r.success);
        assert_eq!(r.message, "session is empty");
    }

    #[test]
    fn test_export_wav_with_audio() {
        let api = api_with_audio_track();
        let r = api.export_wav("/tmp/shruti_test_export_wav.wav");
        assert!(r.success);
        assert!(r.message.contains("exported"));
        // Cleanup
        let _ = std::fs::remove_file("/tmp/shruti_test_export_wav.wav");
    }

    #[test]
    fn test_export_audio_with_data() {
        let api = api_with_audio_track();

        // WAV 16-bit
        let r = api.export_audio("/tmp/shruti_test_ea_16.wav", "wav", "16");
        assert!(r.success);
        let _ = std::fs::remove_file("/tmp/shruti_test_ea_16.wav");

        // WAV 24-bit
        let r = api.export_audio("/tmp/shruti_test_ea_24.wav", "wav", "24");
        assert!(r.success);
        let _ = std::fs::remove_file("/tmp/shruti_test_ea_24.wav");

        // WAV 32-bit float
        let r = api.export_audio("/tmp/shruti_test_ea_32.wav", "wav", "32");
        assert!(r.success);
        let _ = std::fs::remove_file("/tmp/shruti_test_ea_32.wav");

        // FLAC
        let r = api.export_audio("/tmp/shruti_test_ea.flac", "flac", "16");
        assert!(r.success);
        let _ = std::fs::remove_file("/tmp/shruti_test_ea.flac");
    }

    // --- undo/redo ---

    #[test]
    fn test_undo_no_session() {
        let mut api = AgentApi::new();
        let r = api.undo();
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_redo_no_session() {
        let mut api = AgentApi::new();
        let r = api.redo();
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_redo_nothing_to_redo() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.redo();
        assert!(!r.success);
        assert_eq!(r.message, "nothing to redo");
    }

    #[test]
    fn test_undo_nothing_to_undo() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.undo();
        assert!(!r.success);
        assert_eq!(r.message, "nothing to undo");
    }

    #[test]
    fn test_undo_redo_gain_change() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Vocals", "audio");

        // Set gain (creates undo entry)
        api.set_track_gain("Vocals", 0.5);

        // Undo should succeed
        let r = api.undo();
        assert!(r.success);
        assert_eq!(r.message, "undone");

        // Redo should succeed
        let r = api.redo();
        assert!(r.success);
        assert_eq!(r.message, "redone");
    }

    #[test]
    fn test_undo_redo_pan_change() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        api.add_track("Vocals", "audio");

        api.set_track_pan("Vocals", -0.3);
        let r = api.undo();
        assert!(r.success);
        let r = api.redo();
        assert!(r.success);
    }

    // --- add_track error paths ---

    #[test]
    fn test_add_track_no_session() {
        let mut api = AgentApi::new();
        let r = api.add_track("Drums", "audio");
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_add_track_unknown_kind() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.add_track("FX", "effect");
        assert!(!r.success);
        assert!(r.message.contains("unknown track kind"));
    }

    #[test]
    fn test_add_midi_track() {
        let mut api = AgentApi::new();
        api.create_session("Test", 48000, 256);
        let r = api.add_track("Synth", "midi");
        assert!(r.success);
        assert!(r.message.contains("midi"));
    }

    // --- session_info / save_session error paths ---

    #[test]
    fn test_session_info_no_session() {
        let api = AgentApi::new();
        let r = api.session_info();
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_save_session_no_session() {
        let api = AgentApi::new();
        let r = api.save_session("/tmp/test.shruti");
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_list_tracks_no_session() {
        let api = AgentApi::new();
        let r = api.list_tracks();
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    // --- seek / set_tempo error paths ---

    #[test]
    fn test_seek_no_session() {
        let mut api = AgentApi::new();
        let r = api.seek(1000);
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    #[test]
    fn test_set_tempo_no_session() {
        let mut api = AgentApi::new();
        let r = api.set_tempo(120.0);
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    // --- add_region error paths ---

    #[test]
    fn test_add_region_no_session() {
        let mut api = AgentApi::new();
        let r = api.add_region("Track1", "file.wav", 0);
        assert!(!r.success);
        assert_eq!(r.message, "no active session");
    }

    // --- Default impl ---

    #[test]
    fn test_agent_api_default() {
        let api = AgentApi::default();
        assert!(api.session.is_none());
    }
}
