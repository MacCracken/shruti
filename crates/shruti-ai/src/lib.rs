//! AI agent integration for the AGNOS ecosystem.
//!
//! Provides a structured JSON API for agents to control Shruti sessions,
//! and MCP tool definitions for integration with the daimon agent runtime.

#![deny(unsafe_code)]

pub mod agent_api;
pub mod mcp;
pub mod voice;

pub use agent_api::AgentApi;
pub use mcp::McpTools;
pub use voice::{VoiceAction, VoiceIntent, parse_voice_input};
