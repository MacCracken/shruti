use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent_api::{AgentApi, ApiResult};

/// MCP tool definitions for the daimon agent runtime.
///
/// These match the `McpToolDescription` pattern used in agnosticos'
/// `mcp_server.rs`. Each tool maps to an AgentApi method.
pub struct McpTools;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescription {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContentBlock>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContentBlock {
    pub content_type: String,
    pub text: String,
}

impl McpToolResult {
    fn from_api(result: ApiResult) -> Self {
        let text = serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.message.clone());
        Self {
            content: vec![McpContentBlock {
                content_type: "text".into(),
                text,
            }],
            is_error: !result.success,
        }
    }
}

impl McpTools {
    /// Generate the MCP tool manifest for all Shruti tools.
    /// Called by daimon's `build_tool_manifest()`.
    pub fn tool_manifest() -> Vec<McpToolDescription> {
        vec![
            McpToolDescription {
                name: "shruti_session".into(),
                description: "Manage Shruti DAW sessions (create, open, save, info)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["create", "open", "save", "info"],
                            "description": "Session action to perform"
                        },
                        "name": {
                            "type": "string",
                            "description": "Session name (for create)"
                        },
                        "path": {
                            "type": "string",
                            "description": "File path (for open/save)"
                        },
                        "sample_rate": {
                            "type": "integer",
                            "description": "Sample rate in Hz (for create, default 48000)"
                        },
                        "buffer_size": {
                            "type": "integer",
                            "description": "Buffer size in frames (for create, default 256)"
                        }
                    },
                    "required": ["action"]
                }),
            },
            McpToolDescription {
                name: "shruti_tracks".into(),
                description:
                    "Manage tracks in the current session (add, list, set gain/pan, mute, solo)"
                        .into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["add", "list", "gain", "pan", "mute", "solo", "add_region"],
                            "description": "Track action to perform"
                        },
                        "name": {
                            "type": "string",
                            "description": "Track name"
                        },
                        "kind": {
                            "type": "string",
                            "enum": ["audio", "bus"],
                            "description": "Track type (for add)"
                        },
                        "value": {
                            "type": "number",
                            "description": "Value for gain/pan"
                        },
                        "audio_file": {
                            "type": "string",
                            "description": "Audio file path (for add_region)"
                        },
                        "position": {
                            "type": "integer",
                            "description": "Timeline position in frames (for add_region)"
                        }
                    },
                    "required": ["action"]
                }),
            },
            McpToolDescription {
                name: "shruti_transport".into(),
                description: "Control playback transport (play, stop, pause, seek, tempo)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["play", "stop", "pause", "seek", "tempo"],
                            "description": "Transport action"
                        },
                        "value": {
                            "type": "number",
                            "description": "Position in frames (seek) or BPM (tempo)"
                        }
                    },
                    "required": ["action"]
                }),
            },
            McpToolDescription {
                name: "shruti_export".into(),
                description: "Export the current session to an audio file".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Output file path (.wav)"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["wav"],
                            "description": "Export format"
                        }
                    },
                    "required": ["path"]
                }),
            },
            McpToolDescription {
                name: "shruti_analysis".into(),
                description: "Analyze audio tracks and get AI-assisted suggestions (spectrum, dynamics, auto-mix, composition)".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["spectrum", "dynamics", "auto_mix", "composition"],
                            "description": "Analysis action to perform"
                        },
                        "track": {
                            "type": "string",
                            "description": "Track name (required for spectrum and dynamics)"
                        },
                        "fft_size": {
                            "type": "integer",
                            "description": "FFT size for spectral analysis (default: 4096, must be power of 2)"
                        }
                    },
                    "required": ["action"]
                }),
            },
            McpToolDescription {
                name: "shruti_mixer".into(),
                description: "Query and control the mixer state".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["status", "undo", "redo"],
                            "description": "Mixer action"
                        }
                    },
                    "required": ["action"]
                }),
            },
        ]
    }

    /// Dispatch an MCP tool call to the appropriate AgentApi method.
    pub fn dispatch(
        api: &mut AgentApi,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> McpToolResult {
        match tool_name {
            "shruti_session" => Self::handle_session(api, args),
            "shruti_tracks" => Self::handle_tracks(api, args),
            "shruti_transport" => Self::handle_transport(api, args),
            "shruti_export" => Self::handle_export(api, args),
            "shruti_analysis" => Self::handle_analysis(api, args),
            "shruti_mixer" => Self::handle_mixer(api, args),
            _ => McpToolResult {
                content: vec![McpContentBlock {
                    content_type: "text".into(),
                    text: format!("unknown tool: {tool_name}"),
                }],
                is_error: true,
            },
        }
    }

    fn handle_session(api: &mut AgentApi, args: &serde_json::Value) -> McpToolResult {
        let action = args["action"].as_str().unwrap_or("");
        let result = match action {
            "create" => {
                let name = args["name"].as_str().unwrap_or("Untitled");
                let sr = args["sample_rate"].as_u64().unwrap_or(48000) as u32;
                let bs = args["buffer_size"].as_u64().unwrap_or(256) as u32;
                api.create_session(name, sr, bs)
            }
            "open" => {
                let path = args["path"].as_str().unwrap_or("");
                api.open_session(path)
            }
            "save" => {
                let path = args["path"].as_str().unwrap_or("");
                api.save_session(path)
            }
            "info" => api.session_info(),
            _ => ApiResult::err(format!("unknown session action: {action}")),
        };
        McpToolResult::from_api(result)
    }

    fn handle_tracks(api: &mut AgentApi, args: &serde_json::Value) -> McpToolResult {
        let action = args["action"].as_str().unwrap_or("");
        let result = match action {
            "add" => {
                let name = args["name"].as_str().unwrap_or("New Track");
                let kind = args["kind"].as_str().unwrap_or("audio");
                api.add_track(name, kind)
            }
            "list" => api.list_tracks(),
            "gain" => {
                let name = args["name"].as_str().unwrap_or("");
                let value = args["value"].as_f64().unwrap_or(1.0) as f32;
                api.set_track_gain(name, value)
            }
            "pan" => {
                let name = args["name"].as_str().unwrap_or("");
                let value = args["value"].as_f64().unwrap_or(0.0) as f32;
                api.set_track_pan(name, value)
            }
            "mute" => {
                let name = args["name"].as_str().unwrap_or("");
                api.mute_track(name)
            }
            "solo" => {
                let name = args["name"].as_str().unwrap_or("");
                api.solo_track(name)
            }
            "add_region" => {
                let name = args["name"].as_str().unwrap_or("");
                let file = args["audio_file"].as_str().unwrap_or("");
                let pos = args["position"].as_u64().unwrap_or(0);
                api.add_region(name, file, pos)
            }
            _ => ApiResult::err(format!("unknown tracks action: {action}")),
        };
        McpToolResult::from_api(result)
    }

    fn handle_transport(api: &mut AgentApi, args: &serde_json::Value) -> McpToolResult {
        let action = args["action"].as_str().unwrap_or("");
        let result = match action {
            "play" | "stop" | "pause" => api.transport(action),
            "seek" => {
                let pos = args["value"].as_u64().unwrap_or(0);
                api.seek(pos)
            }
            "tempo" => {
                let bpm = args["value"].as_f64().unwrap_or(120.0);
                api.set_tempo(bpm)
            }
            _ => ApiResult::err(format!("unknown transport action: {action}")),
        };
        McpToolResult::from_api(result)
    }

    fn handle_export(api: &AgentApi, args: &serde_json::Value) -> McpToolResult {
        let path = args["path"].as_str().unwrap_or("output.wav");
        McpToolResult::from_api(api.export_wav(path))
    }

    fn handle_analysis(api: &mut AgentApi, args: &serde_json::Value) -> McpToolResult {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("");

        match action {
            "spectrum" => {
                let track = args.get("track").and_then(|t| t.as_str()).unwrap_or("");
                let fft_size = args
                    .get("fft_size")
                    .and_then(|f| f.as_u64())
                    .unwrap_or(4096) as usize;
                McpToolResult::from_api(api.analyze_spectrum(track, fft_size))
            }
            "dynamics" => {
                let track = args.get("track").and_then(|t| t.as_str()).unwrap_or("");
                McpToolResult::from_api(api.analyze_dynamics(track))
            }
            "auto_mix" => McpToolResult::from_api(api.auto_mix_suggest()),
            "composition" => McpToolResult::from_api(api.composition_suggest()),
            _ => McpToolResult {
                content: vec![McpContentBlock {
                    content_type: "text".into(),
                    text: format!("unknown analysis action: {action}"),
                }],
                is_error: true,
            },
        }
    }

    fn handle_mixer(api: &mut AgentApi, args: &serde_json::Value) -> McpToolResult {
        let action = args["action"].as_str().unwrap_or("");
        let result = match action {
            "status" => api.list_tracks(),
            "undo" => api.undo(),
            "redo" => api.redo(),
            _ => ApiResult::err(format!("unknown mixer action: {action}")),
        };
        McpToolResult::from_api(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_manifest() {
        let tools = McpTools::tool_manifest();
        assert_eq!(tools.len(), 6);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"shruti_session"));
        assert!(names.contains(&"shruti_tracks"));
        assert!(names.contains(&"shruti_transport"));
        assert!(names.contains(&"shruti_export"));
        assert!(names.contains(&"shruti_analysis"));
        assert!(names.contains(&"shruti_mixer"));
    }

    #[test]
    fn test_mcp_dispatch_session() {
        let mut api = AgentApi::new();

        let result = McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Agent Song" }),
        );
        assert!(!result.is_error);

        let result = McpTools::dispatch(&mut api, "shruti_session", &json!({ "action": "info" }));
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("Agent Song"));
    }

    #[test]
    fn test_mcp_dispatch_tracks() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(
            &mut api,
            "shruti_tracks",
            &json!({ "action": "add", "name": "Drums", "kind": "audio" }),
        );
        assert!(!result.is_error);

        let result = McpTools::dispatch(
            &mut api,
            "shruti_tracks",
            &json!({ "action": "gain", "name": "Drums", "value": 0.7 }),
        );
        assert!(!result.is_error);

        let result = McpTools::dispatch(&mut api, "shruti_tracks", &json!({ "action": "list" }));
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("Drums"));
    }

    #[test]
    fn test_mcp_dispatch_transport() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        assert!(
            !McpTools::dispatch(&mut api, "shruti_transport", &json!({ "action": "play" }),)
                .is_error
        );

        assert!(
            !McpTools::dispatch(
                &mut api,
                "shruti_transport",
                &json!({ "action": "tempo", "value": 140 }),
            )
            .is_error
        );
    }

    #[test]
    fn test_mcp_dispatch_analysis() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(
            &mut api,
            "shruti_analysis",
            &json!({ "action": "composition" }),
        );
        assert!(!result.is_error);
    }

    #[test]
    fn test_mcp_unknown_tool() {
        let mut api = AgentApi::new();
        let result = McpTools::dispatch(&mut api, "nonexistent", &json!({}));
        assert!(result.is_error);
    }

    // --- shruti_analysis dispatch paths ---

    #[test]
    fn test_mcp_dispatch_analysis_spectrum_no_track() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );
        api.add_track("Vocals", "audio");

        // spectrum with empty track (no regions)
        let result = McpTools::dispatch(
            &mut api,
            "shruti_analysis",
            &json!({ "action": "spectrum", "track": "Vocals" }),
        );
        assert!(result.is_error);
        assert!(result.content[0].text.contains("no audio regions"));
    }

    #[test]
    fn test_mcp_dispatch_analysis_spectrum_missing_track() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(
            &mut api,
            "shruti_analysis",
            &json!({ "action": "spectrum", "track": "NonExistent" }),
        );
        assert!(result.is_error);
        assert!(result.content[0].text.contains("not found"));
    }

    #[test]
    fn test_mcp_dispatch_analysis_dynamics() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );
        api.add_track("Bass", "audio");

        let result = McpTools::dispatch(
            &mut api,
            "shruti_analysis",
            &json!({ "action": "dynamics", "track": "Bass" }),
        );
        // No regions, should error
        assert!(result.is_error);
        assert!(result.content[0].text.contains("no audio regions"));
    }

    #[test]
    fn test_mcp_dispatch_analysis_auto_mix() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(
            &mut api,
            "shruti_analysis",
            &json!({ "action": "auto_mix" }),
        );
        // No audio tracks with regions
        assert!(result.is_error);
        assert!(result.content[0].text.contains("no audio tracks"));
    }

    #[test]
    fn test_mcp_dispatch_analysis_unknown_action() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(&mut api, "shruti_analysis", &json!({ "action": "bogus" }));
        assert!(result.is_error);
        assert!(result.content[0].text.contains("unknown analysis action"));
    }

    // --- shruti_mixer dispatch paths ---

    #[test]
    fn test_mcp_dispatch_mixer_status() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );
        api.add_track("Guitar", "audio");

        let result = McpTools::dispatch(&mut api, "shruti_mixer", &json!({ "action": "status" }));
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("Guitar"));
    }

    #[test]
    fn test_mcp_dispatch_mixer_undo() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        // Nothing to undo
        let result = McpTools::dispatch(&mut api, "shruti_mixer", &json!({ "action": "undo" }));
        assert!(result.is_error);
        assert!(result.content[0].text.contains("nothing to undo"));
    }

    #[test]
    fn test_mcp_dispatch_mixer_redo() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(&mut api, "shruti_mixer", &json!({ "action": "redo" }));
        assert!(result.is_error);
        assert!(result.content[0].text.contains("nothing to redo"));
    }

    #[test]
    fn test_mcp_dispatch_mixer_unknown_action() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(&mut api, "shruti_mixer", &json!({ "action": "bogus" }));
        assert!(result.is_error);
        assert!(result.content[0].text.contains("unknown mixer action"));
    }

    // --- shruti_export dispatch paths ---

    #[test]
    fn test_mcp_dispatch_export_missing_path() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        // No path provided -> defaults to "output.wav", but session is empty
        let result = McpTools::dispatch(&mut api, "shruti_export", &json!({}));
        assert!(result.is_error);
        assert!(result.content[0].text.contains("session is empty"));
    }

    #[test]
    fn test_mcp_dispatch_export_no_session() {
        let mut api = AgentApi::new();
        let result = McpTools::dispatch(
            &mut api,
            "shruti_export",
            &json!({ "path": "/tmp/test.wav" }),
        );
        assert!(result.is_error);
        assert!(result.content[0].text.contains("no active session"));
    }

    // --- shruti_tracks dispatch paths ---

    #[test]
    fn test_mcp_dispatch_tracks_pan() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );
        api.add_track("Drums", "audio");

        let result = McpTools::dispatch(
            &mut api,
            "shruti_tracks",
            &json!({ "action": "pan", "name": "Drums", "value": -0.5 }),
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("pan"));
    }

    #[test]
    fn test_mcp_dispatch_tracks_mute() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );
        api.add_track("Drums", "audio");

        let result = McpTools::dispatch(
            &mut api,
            "shruti_tracks",
            &json!({ "action": "mute", "name": "Drums" }),
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("muted"));
    }

    #[test]
    fn test_mcp_dispatch_tracks_solo() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );
        api.add_track("Drums", "audio");

        let result = McpTools::dispatch(
            &mut api,
            "shruti_tracks",
            &json!({ "action": "solo", "name": "Drums" }),
        );
        assert!(!result.is_error);
        assert!(result.content[0].text.contains("soloed"));
    }

    #[test]
    fn test_mcp_dispatch_tracks_add_region() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );
        api.add_track("Drums", "audio");

        // add_region with a non-existent file should fail
        let result = McpTools::dispatch(
            &mut api,
            "shruti_tracks",
            &json!({ "action": "add_region", "name": "Drums", "audio_file": "/nonexistent.wav", "position": 0 }),
        );
        assert!(result.is_error);
    }

    #[test]
    fn test_mcp_dispatch_tracks_unknown_action() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(&mut api, "shruti_tracks", &json!({ "action": "delete" }));
        assert!(result.is_error);
        assert!(result.content[0].text.contains("unknown tracks action"));
    }

    // --- shruti_session dispatch paths ---

    #[test]
    fn test_mcp_dispatch_session_open_nonexistent() {
        let mut api = AgentApi::new();
        let result = McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "open", "path": "/nonexistent/session.shruti" }),
        );
        assert!(result.is_error);
    }

    #[test]
    fn test_mcp_dispatch_session_save_no_session() {
        let mut api = AgentApi::new();
        let result = McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "save", "path": "/tmp/test.shruti" }),
        );
        assert!(result.is_error);
        assert!(result.content[0].text.contains("no active session"));
    }

    #[test]
    fn test_mcp_dispatch_session_unknown_action() {
        let mut api = AgentApi::new();
        let result =
            McpTools::dispatch(&mut api, "shruti_session", &json!({ "action": "destroy" }));
        assert!(result.is_error);
        assert!(result.content[0].text.contains("unknown session action"));
    }

    // --- shruti_transport dispatch paths ---

    #[test]
    fn test_mcp_dispatch_transport_stop() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(&mut api, "shruti_transport", &json!({ "action": "stop" }));
        assert!(!result.is_error);
    }

    #[test]
    fn test_mcp_dispatch_transport_pause() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result =
            McpTools::dispatch(&mut api, "shruti_transport", &json!({ "action": "pause" }));
        assert!(!result.is_error);
    }

    #[test]
    fn test_mcp_dispatch_transport_seek() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result = McpTools::dispatch(
            &mut api,
            "shruti_transport",
            &json!({ "action": "seek", "value": 48000 }),
        );
        assert!(!result.is_error);
    }

    #[test]
    fn test_mcp_dispatch_transport_unknown_action() {
        let mut api = AgentApi::new();
        McpTools::dispatch(
            &mut api,
            "shruti_session",
            &json!({ "action": "create", "name": "Test" }),
        );

        let result =
            McpTools::dispatch(&mut api, "shruti_transport", &json!({ "action": "rewind" }));
        assert!(result.is_error);
        assert!(result.content[0].text.contains("unknown transport action"));
    }

    // --- McpToolResult serialization ---

    #[test]
    fn test_mcp_tool_result_from_api_ok() {
        let api_result = ApiResult::ok_with_data("test", json!({"key": "value"}));
        let mcp_result = McpToolResult::from_api(api_result);
        assert!(!mcp_result.is_error);
        assert!(mcp_result.content[0].text.contains("test"));
        assert!(mcp_result.content[0].text.contains("key"));
    }

    #[test]
    fn test_mcp_tool_result_from_api_err() {
        let api_result = ApiResult::err("something failed");
        let mcp_result = McpToolResult::from_api(api_result);
        assert!(mcp_result.is_error);
        assert!(mcp_result.content[0].text.contains("something failed"));
    }
}
