//! Music LLM runtime, tokenizer, and AI player agents.
//!
//! Provides the infrastructure for AI-driven virtual instruments that can
//! perform, improvise, and accompany — powered by music LLMs running locally.
//!
//! # Architecture
//!
//! - **Tokenizer**: Converts between MIDI events and token sequences for LLM input/output
//! - **Model runtime**: Loads and runs inference on `.shruti-model` files (pluggable backend)
//! - **Inference scheduler**: Non-blocking generation with lookahead buffer
//! - **AI Player**: `InstrumentNode` implementation that generates MIDI from session context
//! - **Model manager**: Download, cache, validate models

#![deny(unsafe_code)]

pub mod model;
pub mod player;
pub mod tokenizer;

pub use model::{GenerationConfig, InferenceScheduler, ModelInfo, ModelRuntime};
#[cfg(feature = "hoosh")]
pub use model::{HooshRuntime, list_available_models};
pub use player::{AiPlayer, AiPlayerConfig, PlaybackMode};
pub use tokenizer::{MidiToken, MidiTokenizer};
