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
}
