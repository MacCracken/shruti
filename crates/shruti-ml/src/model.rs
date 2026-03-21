//! Model runtime, inference scheduling, and model management.
//!
//! Provides a pluggable backend for running music LLM inference.
//! The default implementation uses a stub that generates random tokens —
//! replace with ONNX Runtime or candle for real model inference.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::tokenizer::MidiToken;

/// Information about a loaded model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name.
    pub name: String,
    /// Model version string.
    pub version: String,
    /// Number of parameters (approximate).
    pub parameters: u64,
    /// Vocabulary size.
    pub vocab_size: u32,
    /// Maximum sequence length.
    pub max_seq_len: u32,
    /// Style tags (e.g., "jazz", "classical", "electronic").
    pub styles: Vec<String>,
}

/// Generation configuration (temperature, sampling, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Temperature for sampling (0.0 = greedy, 1.0 = creative).
    pub temperature: f32,
    /// Top-k sampling (0 = disabled).
    pub top_k: u32,
    /// Repetition penalty (1.0 = no penalty).
    pub repetition_penalty: f32,
    /// Maximum tokens to generate per call.
    pub max_tokens: u32,
    /// Inference timeout in seconds (for remote backends like hoosh).
    pub timeout_secs: u64,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: 0.8,
            top_k: 40,
            repetition_penalty: 1.1,
            max_tokens: 256,
            timeout_secs: 30,
        }
    }
}

/// Trait for model inference backends.
///
/// Implement this trait to plug in ONNX Runtime, candle, or another
/// inference engine. The default `StubRuntime` generates random tokens
/// for testing without a real model.
pub trait ModelRuntime: Send {
    /// Get model info.
    fn info(&self) -> &ModelInfo;

    /// Generate the next token given input context.
    ///
    /// `context` is the sequence of tokens so far (prompt + generated).
    /// `config` controls sampling behavior.
    /// Returns the next token ID.
    fn generate_next(&mut self, context: &[u32], config: &GenerationConfig) -> u32;

    /// Reset the model's internal state (KV cache, etc.).
    fn reset(&mut self);
}

/// Stub runtime that generates musically plausible random token sequences.
///
/// Used for testing the pipeline without a real model. Produces simple
/// ascending/descending note patterns.
pub struct StubRuntime {
    info: ModelInfo,
    step: u32,
    rng_state: u32,
}

impl StubRuntime {
    pub fn new() -> Self {
        Self {
            info: ModelInfo {
                name: "stub-music-model".into(),
                version: "0.1.0".into(),
                parameters: 0,
                vocab_size: MidiToken::VOCAB_SIZE,
                max_seq_len: 2048,
                styles: vec!["test".into()],
            },
            step: 0,
            rng_state: 42,
        }
    }

    /// Simple xorshift32 RNG for deterministic pseudo-random generation.
    fn next_rng(&mut self) -> u32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        self.rng_state
    }
}

impl Default for StubRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRuntime for StubRuntime {
    fn info(&self) -> &ModelInfo {
        &self.info
    }

    fn generate_next(&mut self, _context: &[u32], _config: &GenerationConfig) -> u32 {
        // Generate a simple pattern: velocity, duration, note-on, time-shift, repeat
        let token_id = match self.step % 4 {
            0 => MidiToken::Velocity(16).to_id(), // medium velocity
            1 => MidiToken::Duration(8).to_id(),  // ~400ms
            2 => {
                // Random note in C major scale
                let scale = [60, 62, 64, 65, 67, 69, 71, 72];
                let idx = (self.next_rng() as usize) % scale.len();
                MidiToken::NoteOn(scale[idx]).to_id()
            }
            3 => MidiToken::TimeShift(25).to_id(), // 250ms gap
            _ => MidiToken::TimeShift(25).to_id(),
        };
        self.step += 1;
        token_id
    }

    fn reset(&mut self) {
        self.step = 0;
        self.rng_state = 42;
    }
}

/// Non-blocking inference scheduler with lookahead buffer.
///
/// Runs inference on a background conceptual thread (currently synchronous
/// for simplicity) and maintains a buffer of pre-generated tokens that
/// the audio thread can consume without blocking.
pub struct InferenceScheduler {
    runtime: Box<dyn ModelRuntime>,
    config: GenerationConfig,
    /// Buffer of generated tokens ready for consumption.
    buffer: VecDeque<MidiToken>,
    /// Context window (recent tokens for model input).
    context: Vec<u32>,
    /// Maximum lookahead buffer size in tokens.
    lookahead_size: usize,
}

impl InferenceScheduler {
    /// Create a new scheduler with the given runtime and config.
    pub fn new(
        runtime: Box<dyn ModelRuntime>,
        config: GenerationConfig,
        lookahead_size: usize,
    ) -> Self {
        Self {
            runtime,
            config,
            buffer: VecDeque::new(),
            context: vec![MidiToken::Bos.to_id()],
            lookahead_size,
        }
    }

    /// Fill the lookahead buffer up to `lookahead_size` tokens.
    ///
    /// Call this periodically (e.g., once per audio block) to keep the
    /// buffer topped up. Generation stops at EOS or when buffer is full.
    pub fn fill_buffer(&mut self) {
        while self.buffer.len() < self.lookahead_size {
            let token_id = self.runtime.generate_next(&self.context, &self.config);
            self.context.push(token_id);

            // Trim context if it exceeds max_seq_len, keeping 75% for overlap
            let max_len = self.runtime.info().max_seq_len as usize;
            if max_len > 0 && self.context.len() >= max_len {
                let keep = (max_len * 3) / 4;
                let trim = self.context.len().saturating_sub(keep);
                if trim > 0 {
                    self.context.drain(..trim);
                }
            }

            if let Some(token) = MidiToken::from_id(token_id) {
                if token == MidiToken::Eos {
                    break;
                }
                self.buffer.push_back(token);
            }
        }
    }

    /// Consume the next token from the buffer, if available.
    pub fn next_token(&mut self) -> Option<MidiToken> {
        self.buffer.pop_front()
    }

    /// Number of tokens currently in the buffer.
    pub fn buffered_count(&self) -> usize {
        self.buffer.len()
    }

    /// Reset the scheduler state (clears buffer and context).
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.context = vec![MidiToken::Bos.to_id()];
        self.runtime.reset();
    }

    /// Update the generation config.
    pub fn set_config(&mut self, config: GenerationConfig) {
        self.config = config;
    }

    /// Get the model info.
    pub fn model_info(&self) -> &ModelInfo {
        self.runtime.info()
    }
}

// ── Hoosh inference gateway integration ───────────────────────────────────

#[cfg(feature = "hoosh")]
mod hoosh_runtime {
    use super::*;

    /// Model runtime backed by hoosh inference gateway.
    ///
    /// Connects to a running hoosh server for real LLM inference.
    /// Uses blocking runtime for compatibility with the sync ModelRuntime trait.
    pub struct HooshRuntime {
        client: hoosh::HooshClient,
        model_name: String,
        info: ModelInfo,
        tokio_rt: tokio::runtime::Runtime,
    }

    impl HooshRuntime {
        /// Create a new HooshRuntime connected to the given server.
        pub fn new(base_url: &str, model_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
            let client = hoosh::HooshClient::new(base_url);
            let tokio_rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;

            let info = ModelInfo {
                name: model_name.to_string(),
                version: "hoosh-0.20.3".into(),
                parameters: 0,
                vocab_size: MidiToken::VOCAB_SIZE,
                max_seq_len: 4096,
                styles: vec!["general".into()],
            };

            Ok(Self {
                client,
                model_name: model_name.to_string(),
                info,
                tokio_rt,
            })
        }
    }

    impl ModelRuntime for HooshRuntime {
        fn info(&self) -> &ModelInfo {
            &self.info
        }

        fn generate_next(&mut self, context: &[u32], config: &GenerationConfig) -> u32 {
            // Build a prompt from the token context
            let mut token_str = String::with_capacity(context.len() * 16);
            for id in context {
                if let Some(t) = MidiToken::from_id(*id) {
                    if !token_str.is_empty() {
                        token_str.push(' ');
                    }
                    use std::fmt::Write;
                    let _ = write!(token_str, "{t:?}");
                }
            }
            let prompt = format!(
                "You are a music token generator. Given a sequence of music tokens, output ONLY the next token. \
                 Valid token formats: NoteOn(0-127), NoteOff(0-127), Velocity(0-31), Duration(0-63), TimeShift(0-99), Bar. \
                 Sequence: {token_str}",
            );

            let request = hoosh::InferenceRequest {
                model: self.model_name.clone(),
                prompt,
                temperature: Some(config.temperature as f64),
                max_tokens: Some(16),
                ..Default::default()
            };

            let client = self.client.clone();
            let response = self.tokio_rt.block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(config.timeout_secs),
                    client.infer(&request),
                )
                .await
            });

            match response {
                Ok(Ok(resp)) => {
                    // Parse the response text back to a token
                    parse_token_response(&resp.text)
                        .unwrap_or_else(|| MidiToken::TimeShift(25).to_id())
                }
                _ => {
                    // Timeout or error: fallback to time shift
                    MidiToken::TimeShift(25).to_id()
                }
            }
        }

        fn reset(&mut self) {
            // No persistent state to reset in the hoosh client
        }
    }

    /// Parse a model response text into a token ID.
    pub(crate) fn parse_token_response(text: &str) -> Option<u32> {
        let text = text.trim();

        /// Try to parse a "Prefix(N)" style token, returning the inner u8 value.
        fn parse_paren_u8(text: &str, prefix: &str) -> Option<u8> {
            text.strip_prefix(prefix)
                .and_then(|rest| rest.strip_suffix(')'))
                .and_then(|num_str| num_str.trim().parse::<u8>().ok())
        }

        if let Some(n) = parse_paren_u8(text, "NoteOn(") {
            return Some(MidiToken::NoteOn(n).to_id());
        }
        if let Some(n) = parse_paren_u8(text, "Velocity(") {
            return Some(MidiToken::Velocity(n).to_id());
        }
        if let Some(n) = parse_paren_u8(text, "Duration(") {
            return Some(MidiToken::Duration(n).to_id());
        }
        if let Some(n) = parse_paren_u8(text, "TimeShift(") {
            return Some(MidiToken::TimeShift(n).to_id());
        }
        if text == "Bar" {
            return Some(MidiToken::Bar.to_id());
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_note_on() {
            assert_eq!(
                parse_token_response("NoteOn(60)"),
                Some(MidiToken::NoteOn(60).to_id())
            );
        }

        #[test]
        fn parse_velocity() {
            assert_eq!(
                parse_token_response("Velocity(16)"),
                Some(MidiToken::Velocity(16).to_id())
            );
        }

        #[test]
        fn parse_invalid() {
            assert_eq!(parse_token_response("garbage"), None);
        }

        #[test]
        fn hoosh_runtime_creates() {
            // This just tests construction, not actual server connectivity
            let rt = HooshRuntime::new("http://localhost:9999", "test-model");
            assert!(rt.is_ok());
        }
    }
}

#[cfg(feature = "hoosh")]
pub use hoosh_runtime::HooshRuntime;

/// List available models from a hoosh inference gateway.
#[cfg(feature = "hoosh")]
pub async fn list_available_models(
    client: &hoosh::HooshClient,
) -> Result<Vec<hoosh::ModelInfo>, hoosh::HooshError> {
    client.list_models().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_runtime_produces_tokens() {
        let mut rt = StubRuntime::new();
        let config = GenerationConfig::default();
        let ctx = vec![MidiToken::Bos.to_id()];

        for _ in 0..20 {
            let id = rt.generate_next(&ctx, &config);
            assert!(MidiToken::from_id(id).is_some(), "invalid token id {id}");
        }
    }

    #[test]
    fn stub_runtime_is_deterministic() {
        let config = GenerationConfig::default();
        let ctx = vec![MidiToken::Bos.to_id()];

        let mut rt1 = StubRuntime::new();
        let mut rt2 = StubRuntime::new();

        let tokens1: Vec<u32> = (0..20).map(|_| rt1.generate_next(&ctx, &config)).collect();
        let tokens2: Vec<u32> = (0..20).map(|_| rt2.generate_next(&ctx, &config)).collect();
        assert_eq!(tokens1, tokens2);
    }

    #[test]
    fn stub_runtime_reset() {
        let mut rt = StubRuntime::new();
        let config = GenerationConfig::default();
        let ctx = vec![MidiToken::Bos.to_id()];

        let first = rt.generate_next(&ctx, &config);
        rt.reset();
        let after_reset = rt.generate_next(&ctx, &config);
        assert_eq!(first, after_reset);
    }

    #[test]
    fn scheduler_fills_buffer() {
        let rt = Box::new(StubRuntime::new());
        let config = GenerationConfig::default();
        let mut sched = InferenceScheduler::new(rt, config, 32);

        assert_eq!(sched.buffered_count(), 0);
        sched.fill_buffer();
        assert!(sched.buffered_count() > 0);
        assert!(sched.buffered_count() <= 32);
    }

    #[test]
    fn scheduler_next_token_consumes() {
        let rt = Box::new(StubRuntime::new());
        let config = GenerationConfig::default();
        let mut sched = InferenceScheduler::new(rt, config, 16);

        sched.fill_buffer();
        let count_before = sched.buffered_count();
        let token = sched.next_token();
        assert!(token.is_some());
        assert_eq!(sched.buffered_count(), count_before - 1);
    }

    #[test]
    fn scheduler_reset_clears() {
        let rt = Box::new(StubRuntime::new());
        let config = GenerationConfig::default();
        let mut sched = InferenceScheduler::new(rt, config, 16);

        sched.fill_buffer();
        assert!(sched.buffered_count() > 0);
        sched.reset();
        assert_eq!(sched.buffered_count(), 0);
    }

    #[test]
    fn model_info_serializes() {
        let info = ModelInfo {
            name: "test-model".into(),
            version: "1.0".into(),
            parameters: 7_000_000_000,
            vocab_size: 584,
            max_seq_len: 2048,
            styles: vec!["jazz".into(), "classical".into()],
        };
        let json = serde_json::to_string(&info).unwrap();
        let rt: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.name, "test-model");
        assert_eq!(rt.parameters, 7_000_000_000);
    }

    #[test]
    fn generation_config_default() {
        let config = GenerationConfig::default();
        assert!((config.temperature - 0.8).abs() < 0.01);
        assert_eq!(config.top_k, 40);
    }
}
