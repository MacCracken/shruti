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
        assert_eq!(tools.len(), 5);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"shruti_session"));
        assert!(names.contains(&"shruti_tracks"));
        assert!(names.contains(&"shruti_transport"));
        assert!(names.contains(&"shruti_export"));
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
    fn test_mcp_unknown_tool() {
        let mut api = AgentApi::new();
        let result = McpTools::dispatch(&mut api, "nonexistent", &json!({}));
        assert!(result.is_error);
    }
}
