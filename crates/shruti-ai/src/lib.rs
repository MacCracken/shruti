//! AI agent integration for the AGNOS ecosystem.
//!
//! Provides a structured JSON API for agents to control Shruti sessions,
//! and MCP tool definitions for integration with the daimon agent runtime.
//!
//! With the `tarang` feature: adds media analysis, audio fingerprinting,
//! and transcription routing via tarang-ai.

#![deny(unsafe_code)]

pub mod agent_api;
pub mod hardware;
pub mod mcp;
pub mod serve;
pub mod voice;

#[cfg(feature = "tarang")]
pub mod media_analysis;

pub use agent_api::AgentApi;
pub use hardware::HardwareInfo;
pub use mcp::McpTools;
pub use serve::{MAX_BODY_SIZE, RATE_LIMIT_RPS, RateLimiter};
pub use voice::{VoiceAction, VoiceIntent, parse_voice_input};
