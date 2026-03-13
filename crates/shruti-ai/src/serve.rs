//! HTTP server wrapping AgentApi for AGNOS integration.
//!
//! Run with `shruti serve --port 8050`.

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::agent_api::{AgentApi, ApiResult};
use crate::mcp::McpTools;

/// Maximum request body size in bytes (1 MB).
pub const MAX_BODY_SIZE: usize = 1_048_576;

/// Maximum requests per second for rate limiting.
pub const RATE_LIMIT_RPS: u32 = 100;

/// Rate limiter using a sliding window counter.
#[derive(Debug)]
pub struct RateLimiter {
    /// Timestamps of recent requests within the current window.
    window_start: Instant,
    count: u32,
    max_rps: u32,
}

impl RateLimiter {
    pub fn new(max_rps: u32) -> Self {
        Self {
            window_start: Instant::now(),
            count: 0,
            max_rps,
        }
    }

    /// Check if a request is allowed. Returns true if allowed, false if rate-limited.
    pub fn check(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.window_start);

        // Reset window every second
        if elapsed.as_secs() >= 1 {
            self.window_start = now;
            self.count = 0;
        }

        if self.count >= self.max_rps {
            return false;
        }

        self.count += 1;
        true
    }
}

/// Shared application state: the AgentApi and rate limiter behind a mutex.
pub struct SharedState {
    pub api: AgentApi,
    pub rate_limiter: RateLimiter,
}

/// Shared application state: the AgentApi and rate limiter behind a mutex.
pub type AppState = Arc<Mutex<SharedState>>;

/// Build the axum Router with all endpoints.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/session", post(handle_session))
        .route("/api/tracks", post(handle_tracks))
        .route("/api/transport", post(handle_transport))
        .route("/api/export", post(handle_export))
        .route("/api/mixer", post(handle_mixer))
        .route("/api/analysis", post(handle_analysis))
        .route("/api/mcp", post(handle_mcp))
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        // Only allow requests from localhost origins — the agent API is
        // not designed for cross-origin browser access from arbitrary sites.
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|origin, _| {
                    origin.as_bytes().starts_with(b"http://localhost")
                        || origin.as_bytes().starts_with(b"http://127.0.0.1")
                }))
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state)
}

/// Start the HTTP server on the given port.
pub async fn run_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let shared = SharedState {
        api: AgentApi::new(),
        rate_limiter: RateLimiter::new(RATE_LIMIT_RPS),
    };
    let state = Arc::new(Mutex::new(shared));
    let app = app(state);

    // Bind to localhost only — the agent API should not be exposed to the
    // network.  Use a reverse proxy if remote access is needed.
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("shruti serve listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Check rate limit on the shared state. Returns Err with 429 if rate-limited.
async fn check_rate_limit(state: &AppState) -> Result<(), (StatusCode, Json<ApiResult>)> {
    let mut shared = state.lock().await;
    if !shared.rate_limiter.check() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResult::err("rate limit exceeded")),
        ));
    }
    Ok(())
}

// --- Health check ---

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

// --- Session endpoint ---

#[derive(Deserialize)]
struct SessionRequest {
    action: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default = "default_sample_rate")]
    sample_rate: u32,
    #[serde(default = "default_buffer_size")]
    buffer_size: u32,
}

fn default_sample_rate() -> u32 {
    48000
}
fn default_buffer_size() -> u32 {
    256
}

async fn handle_session(
    State(state): State<AppState>,
    Json(req): Json<SessionRequest>,
) -> (StatusCode, Json<ApiResult>) {
    if let Err(e) = check_rate_limit(&state).await {
        return e;
    }
    let mut shared = state.lock().await;
    let api = &mut shared.api;
    let result = match req.action.as_str() {
        "create" => {
            let name = req.name.as_deref().unwrap_or("Untitled");
            api.create_session(name, req.sample_rate, req.buffer_size)
        }
        "open" => {
            let path = req.path.as_deref().unwrap_or("");
            api.open_session(path)
        }
        "save" => {
            let path = req.path.as_deref().unwrap_or("");
            api.save_session(path)
        }
        "info" => api.session_info(),
        _ => ApiResult::err(format!("unknown session action: {}", req.action)),
    };
    let status = if result.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(result))
}

// --- Tracks endpoint ---

#[derive(Deserialize)]
struct TracksRequest {
    action: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    value: Option<f64>,
    #[serde(default)]
    audio_file: Option<String>,
    #[serde(default)]
    position: Option<u64>,
}

async fn handle_tracks(
    State(state): State<AppState>,
    Json(req): Json<TracksRequest>,
) -> (StatusCode, Json<ApiResult>) {
    if let Err(e) = check_rate_limit(&state).await {
        return e;
    }
    let mut shared = state.lock().await;
    let api = &mut shared.api;
    let result = match req.action.as_str() {
        "add" => {
            let name = req.name.as_deref().unwrap_or("New Track");
            let kind = req.kind.as_deref().unwrap_or("audio");
            api.add_track(name, kind)
        }
        "list" => api.list_tracks(),
        "gain" => {
            let name = req.name.as_deref().unwrap_or("");
            let value = req.value.unwrap_or(1.0) as f32;
            api.set_track_gain(name, value)
        }
        "pan" => {
            let name = req.name.as_deref().unwrap_or("");
            let value = req.value.unwrap_or(0.0) as f32;
            api.set_track_pan(name, value)
        }
        "mute" => {
            let name = req.name.as_deref().unwrap_or("");
            api.mute_track(name)
        }
        "solo" => {
            let name = req.name.as_deref().unwrap_or("");
            api.solo_track(name)
        }
        "add_region" => {
            let name = req.name.as_deref().unwrap_or("");
            let file = req.audio_file.as_deref().unwrap_or("");
            let pos = req.position.unwrap_or(0);
            api.add_region(name, file, pos)
        }
        _ => ApiResult::err(format!("unknown tracks action: {}", req.action)),
    };
    let status = if result.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(result))
}

// --- Transport endpoint ---

#[derive(Deserialize)]
struct TransportRequest {
    action: String,
    #[serde(default)]
    value: Option<f64>,
}

async fn handle_transport(
    State(state): State<AppState>,
    Json(req): Json<TransportRequest>,
) -> (StatusCode, Json<ApiResult>) {
    if let Err(e) = check_rate_limit(&state).await {
        return e;
    }
    let mut shared = state.lock().await;
    let api = &mut shared.api;
    let result = match req.action.as_str() {
        "play" | "stop" | "pause" => api.transport(&req.action),
        "seek" => {
            let pos = req.value.unwrap_or(0.0) as u64;
            api.seek(pos)
        }
        "tempo" => {
            let bpm = req.value.unwrap_or(120.0);
            api.set_tempo(bpm)
        }
        _ => ApiResult::err(format!("unknown transport action: {}", req.action)),
    };
    let status = if result.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(result))
}

// --- Export endpoint ---

#[derive(Deserialize)]
struct ExportRequest {
    #[serde(default = "default_export_path")]
    path: String,
    #[serde(default = "default_export_format")]
    format: String,
    #[serde(default = "default_bit_depth")]
    bit_depth: String,
}

fn default_export_path() -> String {
    "output.wav".into()
}
fn default_export_format() -> String {
    "wav".into()
}
fn default_bit_depth() -> String {
    "24".into()
}

async fn handle_export(
    State(state): State<AppState>,
    Json(req): Json<ExportRequest>,
) -> (StatusCode, Json<ApiResult>) {
    if let Err(e) = check_rate_limit(&state).await {
        return e;
    }
    let shared = state.lock().await;
    let api = &shared.api;
    let result = api.export_audio(&req.path, &req.format, &req.bit_depth);
    let status = if result.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(result))
}

// --- Mixer endpoint ---

#[derive(Deserialize)]
struct MixerRequest {
    action: String,
}

async fn handle_mixer(
    State(state): State<AppState>,
    Json(req): Json<MixerRequest>,
) -> (StatusCode, Json<ApiResult>) {
    if let Err(e) = check_rate_limit(&state).await {
        return e;
    }
    let mut shared = state.lock().await;
    let api = &mut shared.api;
    let result = match req.action.as_str() {
        "status" => api.list_tracks(),
        "undo" => api.undo(),
        "redo" => api.redo(),
        _ => ApiResult::err(format!("unknown mixer action: {}", req.action)),
    };
    let status = if result.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(result))
}

// --- Analysis endpoint ---

#[derive(Deserialize)]
struct AnalysisRequest {
    action: String,
    #[serde(default)]
    track: Option<String>,
    #[serde(default)]
    fft_size: Option<usize>,
}

async fn handle_analysis(
    State(state): State<AppState>,
    Json(req): Json<AnalysisRequest>,
) -> (StatusCode, Json<ApiResult>) {
    if let Err(e) = check_rate_limit(&state).await {
        return e;
    }
    let shared = state.lock().await;
    let api = &shared.api;
    let result = match req.action.as_str() {
        "spectrum" => {
            let track = req.track.as_deref().unwrap_or("");
            let fft_size = req.fft_size.unwrap_or(4096);
            api.analyze_spectrum(track, fft_size)
        }
        "dynamics" => {
            let track = req.track.as_deref().unwrap_or("");
            api.analyze_dynamics(track)
        }
        "auto_mix" => api.auto_mix_suggest(),
        "composition" => api.composition_suggest(),
        _ => ApiResult::err(format!("unknown analysis action: {}", req.action)),
    };
    let status = if result.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(result))
}

// --- Raw MCP dispatch endpoint ---

#[derive(Deserialize)]
struct McpRequest {
    tool: String,
    #[serde(default)]
    args: serde_json::Value,
}

#[derive(Serialize)]
struct McpResponse {
    content: Vec<crate::mcp::McpContentBlock>,
    is_error: bool,
}

async fn handle_mcp(
    State(state): State<AppState>,
    Json(req): Json<McpRequest>,
) -> Result<(StatusCode, Json<McpResponse>), (StatusCode, Json<ApiResult>)> {
    check_rate_limit(&state).await?;
    let mut shared = state.lock().await;
    let result = McpTools::dispatch(&mut shared.api, &req.tool, &req.args);
    let status = if result.is_error {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(McpResponse {
            content: result.content,
            is_error: result.is_error,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        Arc::new(Mutex::new(SharedState {
            api: AgentApi::new(),
            rate_limiter: RateLimiter::new(RATE_LIMIT_RPS),
        }))
    }

    fn test_app() -> Router {
        app(test_state())
    }

    async fn post_json(
        app: &Router,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let req = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        (status, json)
    }

    async fn get_json(app: &Router, uri: &str) -> (StatusCode, serde_json::Value) {
        let req = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn test_health() {
        let app = test_app();
        let (status, json) = get_json(&app, "/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "ok");
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_session_create_and_info() {
        let state = test_state();
        let app = app(state);

        // Create session
        let (status, json) = post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test Song"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["success"], true);

        // Get info
        let (status, json) =
            post_json(&app, "/api/session", serde_json::json!({"action": "info"})).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["name"], "Test Song");
    }

    #[tokio::test]
    async fn test_session_info_no_session() {
        let app = test_app();
        let (status, json) =
            post_json(&app, "/api/session", serde_json::json!({"action": "info"})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["success"], false);
    }

    #[tokio::test]
    async fn test_tracks_add_and_list() {
        let state = test_state();
        let app = app(state);

        // Create session first
        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;

        // Add track
        let (status, json) = post_json(
            &app,
            "/api/tracks",
            serde_json::json!({"action": "add", "name": "Drums", "kind": "audio"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["success"], true);

        // List tracks
        let (status, json) =
            post_json(&app, "/api/tracks", serde_json::json!({"action": "list"})).await;
        assert_eq!(status, StatusCode::OK);
        let tracks = json["data"]["tracks"].as_array().unwrap();
        assert!(tracks.iter().any(|t| t["name"] == "Drums"));
    }

    #[tokio::test]
    async fn test_tracks_gain_pan_mute_solo() {
        let state = test_state();
        let app = app(state);

        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;
        post_json(
            &app,
            "/api/tracks",
            serde_json::json!({"action": "add", "name": "Guitar", "kind": "audio"}),
        )
        .await;

        let (s, j) = post_json(
            &app,
            "/api/tracks",
            serde_json::json!({"action": "gain", "name": "Guitar", "value": 0.7}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(j["success"], true);

        let (s, j) = post_json(
            &app,
            "/api/tracks",
            serde_json::json!({"action": "pan", "name": "Guitar", "value": -0.3}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(j["success"], true);

        let (s, j) = post_json(
            &app,
            "/api/tracks",
            serde_json::json!({"action": "mute", "name": "Guitar"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(j["message"].as_str().unwrap().contains("muted"));

        let (s, j) = post_json(
            &app,
            "/api/tracks",
            serde_json::json!({"action": "solo", "name": "Guitar"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(j["message"].as_str().unwrap().contains("soloed"));
    }

    #[tokio::test]
    async fn test_transport() {
        let state = test_state();
        let app = app(state);

        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;

        let (s, _) = post_json(
            &app,
            "/api/transport",
            serde_json::json!({"action": "play"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);

        let (s, _) = post_json(
            &app,
            "/api/transport",
            serde_json::json!({"action": "pause"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);

        let (s, _) = post_json(
            &app,
            "/api/transport",
            serde_json::json!({"action": "stop"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);

        let (s, _) = post_json(
            &app,
            "/api/transport",
            serde_json::json!({"action": "seek", "value": 48000}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);

        let (s, _) = post_json(
            &app,
            "/api/transport",
            serde_json::json!({"action": "tempo", "value": 140}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_transport_unknown_action() {
        let state = test_state();
        let app = app(state);

        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;

        let (s, j) = post_json(
            &app,
            "/api/transport",
            serde_json::json!({"action": "rewind"}),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["success"], false);
    }

    #[tokio::test]
    async fn test_export_empty_session() {
        let state = test_state();
        let app = app(state);

        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;

        let (s, j) = post_json(
            &app,
            "/api/export",
            serde_json::json!({"path": "/tmp/test.wav"}),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert!(j["message"].as_str().unwrap().contains("empty"));
    }

    #[tokio::test]
    async fn test_mixer_status_and_undo() {
        let state = test_state();
        let app = app(state);

        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;
        post_json(
            &app,
            "/api/tracks",
            serde_json::json!({"action": "add", "name": "Bass", "kind": "audio"}),
        )
        .await;

        let (s, j) = post_json(&app, "/api/mixer", serde_json::json!({"action": "status"})).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(j["success"], true);

        // Nothing to undo
        let (s, j) = post_json(&app, "/api/mixer", serde_json::json!({"action": "undo"})).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["success"], false);
    }

    #[tokio::test]
    async fn test_analysis_composition() {
        let state = test_state();
        let app = app(state);

        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;

        let (s, j) = post_json(
            &app,
            "/api/analysis",
            serde_json::json!({"action": "composition"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(j["success"], true);
    }

    #[tokio::test]
    async fn test_mcp_dispatch() {
        let state = test_state();
        let app = app(state);

        // Create session via MCP
        let (s, j) = post_json(
            &app,
            "/api/mcp",
            serde_json::json!({"tool": "shruti_session", "args": {"action": "create", "name": "MCP Test"}}),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(j["is_error"], false);

        // Info via MCP
        let (s, j) = post_json(
            &app,
            "/api/mcp",
            serde_json::json!({"tool": "shruti_session", "args": {"action": "info"}}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(
            j["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("MCP Test")
        );
    }

    #[tokio::test]
    async fn test_mcp_unknown_tool() {
        let app = test_app();
        let (s, j) = post_json(
            &app,
            "/api/mcp",
            serde_json::json!({"tool": "nonexistent", "args": {}}),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["is_error"], true);
    }

    #[tokio::test]
    async fn test_session_unknown_action() {
        let app = test_app();
        let (s, j) = post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "destroy"}),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["success"], false);
    }

    #[tokio::test]
    async fn test_tracks_unknown_action() {
        let state = test_state();
        let app = app(state);
        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;

        let (s, j) = post_json(&app, "/api/tracks", serde_json::json!({"action": "delete"})).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["success"], false);
    }

    #[tokio::test]
    async fn test_mixer_unknown_action() {
        let state = test_state();
        let app = app(state);
        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;

        let (s, j) = post_json(&app, "/api/mixer", serde_json::json!({"action": "bogus"})).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["success"], false);
    }

    #[tokio::test]
    async fn test_analysis_unknown_action() {
        let state = test_state();
        let app = app(state);
        post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Test"}),
        )
        .await;

        let (s, j) = post_json(
            &app,
            "/api/analysis",
            serde_json::json!({"action": "bogus"}),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["success"], false);
    }

    // --- Body size limit tests ---

    #[tokio::test]
    async fn test_body_size_limit_rejects_oversized() {
        let app = test_app();
        // Create a body larger than MAX_BODY_SIZE (1 MB)
        let large_value = "x".repeat(MAX_BODY_SIZE + 1);
        let req = Request::builder()
            .method("POST")
            .uri("/api/session")
            .header("content-type", "application/json")
            .body(Body::from(large_value))
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        // axum returns 413 Payload Too Large when body limit is exceeded
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_body_size_limit_allows_normal_request() {
        let app = test_app();
        let (s, _) = post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Small"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
    }

    // --- Rate limiter unit tests ---

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let mut rl = RateLimiter::new(10);
        for _ in 0..10 {
            assert!(rl.check());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let mut rl = RateLimiter::new(5);
        for _ in 0..5 {
            assert!(rl.check());
        }
        // 6th request should be blocked
        assert!(!rl.check());
    }

    // --- Security: CORS restriction ---

    #[tokio::test]
    async fn test_cors_rejects_foreign_origin() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/api/session")
            .header("content-type", "application/json")
            .header("origin", "https://evil.example.com")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"action": "info"})).unwrap(),
            ))
            .unwrap();
        let response = app.clone().oneshot(req).await.unwrap();
        // The response should NOT include an Access-Control-Allow-Origin
        // header for a foreign origin.
        let cors_header = response.headers().get("access-control-allow-origin");
        assert!(
            cors_header.is_none(),
            "CORS should not allow foreign origin, got: {:?}",
            cors_header
        );
    }

    #[tokio::test]
    async fn test_cors_allows_localhost_origin() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/api/session")
            .header("content-type", "application/json")
            .header("origin", "http://localhost:3000")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"action": "info"})).unwrap(),
            ))
            .unwrap();
        let response = app.clone().oneshot(req).await.unwrap();
        let cors_header = response.headers().get("access-control-allow-origin");
        assert!(cors_header.is_some(), "CORS should allow localhost origin");
    }

    // --- Security: create_session input validation via HTTP ---

    #[tokio::test]
    async fn test_create_session_rejects_zero_sample_rate_http() {
        let app = test_app();
        let (s, j) = post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Bad", "sample_rate": 0}),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["success"], false);
    }

    #[tokio::test]
    async fn test_create_session_rejects_bad_buffer_size_http() {
        let app = test_app();
        let (s, j) = post_json(
            &app,
            "/api/session",
            serde_json::json!({"action": "create", "name": "Bad", "buffer_size": 0}),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(j["success"], false);
    }
}
