use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone, Default)]
struct AppState {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

#[derive(Clone)]
struct Session {
    worktree_id: String,
    events: Vec<TimelineEvent>,
    usage: Usage,
}

#[derive(Clone, Default)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: Option<u64>,
    cache_write_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub mode: SessionMode,
    pub worktree_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    Chat,
    Autonomous,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimelineEvent {
    pub event_id: String,
    pub session_id: String,
    pub run_id: Option<String>,
    pub message_id: Option<String>,
    pub model_request_id: Option<String>,
    pub tool_call_id: Option<String>,
    pub prompt_assembly_id: Option<String>,
    pub provider_model_id: Option<String>,
    pub evidence_kind: EvidenceKind,
    pub sequence: u64,
    pub state: String,
    pub source_id: Option<String>,
    pub prompt_sources: Vec<PromptSource>,
    pub message: Option<ConversationMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ConversationMessage {
    pub role: String,
    pub blocks: Vec<RichTextBlock>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichTextBlock {
    Paragraph { text: String },
    Heading { level: u8, text: String },
    BulletList { items: Vec<String> },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct WeatherObservation {
    pub location: String,
    pub temperature_c: f64,
    pub apparent_temperature_c: f64,
    pub precipitation_mm: f64,
    pub wind_speed_kmh: f64,
    pub weather_code: u16,
    pub observed_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PromptSource {
    pub source_kind: String,
    pub source_id: String,
    pub revision: Option<String>,
    pub path: Option<String>,
    pub inclusion_reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    ChatMessage,
    PromptAssembly,
    ModelRequest,
    ModelResponse,
    ToolCall,
    ReasoningTrace,
    Validation,
    Artifact,
    Failure,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineResponse {
    pub session_id: String,
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub state: String,
    pub worktree_id: String,
    pub event_count: usize,
    pub latest_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreditUsage {
    pub provider_model_id: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub known_cost: Option<f64>,
    pub missing_telemetry: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct EvidenceRequest {
    pub evidence_kind: EvidenceKind,
    pub state: String,
    pub source_id: Option<String>,
    pub provider_model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    pub message_id: String,
    pub content: String,
    pub agent_template_id: Option<String>,
    #[serde(default)]
    pub prompt_sources: Vec<PromptSource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message_id: String,
    pub prompt_assembly_id: String,
    pub model_request_id: String,
    pub model_response_id: String,
    pub model_response_text: String,
    pub assistant_message: ConversationMessage,
}

#[derive(Debug, Deserialize)]
pub struct LifecycleRequest {
    pub action: LifecycleAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Start,
    Stop,
    Reload,
}

#[derive(Debug, Serialize)]
pub struct LifecycleResponse {
    pub event_id: String,
    pub state: String,
    pub diagnostic: Option<String>,
}

/// Builds the Context Agent API and same-origin static frontend service.
pub fn router(static_dir: &str) -> Router {
    let state = AppState::default();
    let serve_dir = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(format!("{static_dir}/index.html")));
    Router::new()
        .route("/health", get(health))
        .route("/api/sessions", get(list_sessions).post(create_session))
        .route("/api/sessions/:session_id/timeline", get(get_timeline))
        .route("/api/sessions/:session_id/reconnect", get(get_timeline))
        .route("/api/sessions/:session_id/messages", post(create_message))
        .route("/api/sessions/:session_id/evidence", post(add_evidence))
        .route("/api/sessions/:session_id/credits", get(get_credits))
        .route("/api/tools/weather/:city", get(get_weather))
        .route(
            "/api/sessions/:session_id/mcp-servers/:server_id/lifecycle",
            post(lifecycle),
        )
        .with_state(state)
        .fallback_service(serve_dir)
}

async fn health() -> &'static str {
    "ok"
}

async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<SessionSummary>>, (StatusCode, String)> {
    let sessions = state.sessions.lock().unwrap();
    let summaries = sessions
        .iter()
        .map(|(session_id, session)| SessionSummary {
            session_id: session_id.clone(),
            state: "interactive".to_string(),
            worktree_id: session.worktree_id.clone(),
            event_count: session.events.len(),
            latest_message: session
                .events
                .iter()
                .filter(|event| event.evidence_kind == EvidenceKind::ChatMessage)
                .filter_map(|event| event.source_id.clone())
                .last(),
        })
        .collect();
    Ok(Json(summaries))
}

async fn create_session(
    State(state): State<AppState>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> {
    if request.worktree_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "worktree_id is required".to_string(),
        ));
    }
    let session_id = next_id("session");
    state.sessions.lock().unwrap().insert(
        session_id.clone(),
        Session {
            worktree_id: request.worktree_id,
            events: Vec::new(),
            usage: Usage::default(),
        },
    );
    Ok(Json(CreateSessionResponse {
        session_id,
        state: "interactive".to_string(),
    }))
}

async fn get_timeline(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<TimelineResponse>, (StatusCode, String)> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(&session_id)
        .ok_or((StatusCode::NOT_FOUND, "session not found".to_string()))?;
    Ok(Json(TimelineResponse {
        session_id,
        events: session.events.clone(),
    }))
}

async fn create_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<MessageRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, String)> {
    if !state.sessions.lock().unwrap().contains_key(&session_id) {
        return Err((StatusCode::NOT_FOUND, "session not found".to_string()));
    }
    let prompt_assembly_id = next_id("prompt");
    let model_request_id = next_id("request");
    let model_response_id = next_id("response");
    let is_munster_weather = {
        let content = request.content.to_lowercase();
        content.contains("weather") && (content.contains("munster") || content.contains("münster"))
    };
    let assistant_message = if is_munster_weather {
        let observation = fetch_weather("Munster").await.map_err(|error| {
            tracing::error!(%error, "weather tool failed");
            (
                StatusCode::BAD_GATEWAY,
                format!("weather_fetch_failed: {error}"),
            )
        })?;
        weather_message(&observation)
    } else {
        ConversationMessage {
            role: "assistant".to_string(),
            blocks: vec![RichTextBlock::Paragraph {
                text: format!("Model response: {}", redact_content(&request.content)),
            }],
            tool_calls: Vec::new(),
        }
    };
    let model_response_text = message_plain_text(&assistant_message);
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get_mut(&session_id)
        .ok_or((StatusCode::NOT_FOUND, "session not found".to_string()))?;
    let run_id = next_id("run");
    let prompt_sources = request.prompt_sources.clone();
    let mut add_event = |kind, message_id, prompt_id, request_id, source_id, sources, message| {
        let sequence = session.events.len() as u64;
        session.events.push(TimelineEvent {
            event_id: next_id("event"),
            session_id: session_id.clone(),
            run_id: Some(run_id.clone()),
            message_id,
            model_request_id: request_id,
            tool_call_id: None,
            prompt_assembly_id: prompt_id,
            provider_model_id: None,
            evidence_kind: kind,
            sequence,
            state: "recorded".to_string(),
            source_id,
            prompt_sources: sources,
            message,
        });
    };
    let user_content = redact_content(&request.content);
    add_event(
        EvidenceKind::ChatMessage,
        Some(request.message_id.clone()),
        None,
        None,
        None,
        Vec::new(),
        Some(ConversationMessage {
            role: "user".to_string(),
            blocks: vec![RichTextBlock::Paragraph { text: user_content }],
            tool_calls: Vec::new(),
        }),
    );
    add_event(
        EvidenceKind::PromptAssembly,
        Some(request.message_id.clone()),
        Some(prompt_assembly_id.clone()),
        None,
        request.agent_template_id,
        prompt_sources.clone(),
        None,
    );
    add_event(
        EvidenceKind::ModelRequest,
        Some(request.message_id.clone()),
        Some(prompt_assembly_id.clone()),
        Some(model_request_id.clone()),
        None,
        Vec::new(),
        None,
    );
    add_event(
        EvidenceKind::ModelResponse,
        Some(request.message_id.clone()),
        Some(prompt_assembly_id.clone()),
        Some(model_request_id.clone()),
        None,
        Vec::new(),
        Some(assistant_message.clone()),
    );
    Ok(Json(MessageResponse {
        message_id: request.message_id,
        prompt_assembly_id,
        model_request_id,
        model_response_id,
        model_response_text,
        assistant_message,
    }))
}

async fn add_evidence(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<EvidenceRequest>,
) -> Result<Json<TimelineEvent>, (StatusCode, String)> {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get_mut(&session_id)
        .ok_or((StatusCode::NOT_FOUND, "session not found".to_string()))?;
    let event = TimelineEvent {
        event_id: next_id("event"),
        session_id: session_id.clone(),
        run_id: None,
        message_id: None,
        model_request_id: None,
        tool_call_id: None,
        prompt_assembly_id: None,
        provider_model_id: request.provider_model_id,
        evidence_kind: request.evidence_kind,
        sequence: session.events.len() as u64,
        state: request.state,
        source_id: request.source_id,
        prompt_sources: Vec::new(),
        message: None,
    };
    session.events.push(event.clone());
    Ok(Json(event))
}

async fn get_credits(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<CreditUsage>, (StatusCode, String)> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(&session_id)
        .ok_or((StatusCode::NOT_FOUND, "session not found".to_string()))?;
    let mut missing = Vec::new();
    if session.usage.cache_read_tokens.is_none() {
        missing.push("cache_read_tokens".to_string());
    }
    if session.usage.cache_write_tokens.is_none() {
        missing.push("cache_write_tokens".to_string());
    }
    Ok(Json(CreditUsage {
        provider_model_id: None,
        input_tokens: session.usage.input_tokens,
        output_tokens: session.usage.output_tokens,
        cache_read_tokens: session.usage.cache_read_tokens,
        cache_write_tokens: session.usage.cache_write_tokens,
        known_cost: None,
        missing_telemetry: missing,
    }))
}

fn redact_content(content: &str) -> String {
    content
        .split_whitespace()
        .map(|word| {
            if word.contains("API_KEY=") || word.contains("TOKEN=") || word.contains("SECRET=") {
                "[REDACTED]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
}

#[derive(Deserialize)]
struct OpenMeteoCurrent {
    time: String,
    temperature_2m: f64,
    apparent_temperature: f64,
    precipitation: f64,
    weather_code: u16,
    wind_speed_10m: f64,
}

async fn get_weather(
    Path(city): Path<String>,
) -> Result<Json<WeatherObservation>, (StatusCode, String)> {
    fetch_weather(&city).await.map(Json).map_err(|error| {
        tracing::error!(%error, %city, "weather tool failed");
        (
            StatusCode::BAD_GATEWAY,
            format!("weather_fetch_failed: {error}"),
        )
    })
}

async fn fetch_weather(city: &str) -> Result<WeatherObservation, String> {
    if !matches!(city.to_lowercase().as_str(), "munster" | "münster") {
        return Err(format!("unsupported location '{city}'; try Munster"));
    }

    let response = reqwest::Client::new()
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", "51.9607"),
            ("longitude", "7.6261"),
            (
                "current",
                "temperature_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m",
            ),
            ("timezone", "Europe/Berlin"),
        ])
        .send()
        .await
        .map_err(|error| format!("Open-Meteo request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Open-Meteo returned an error: {error}"))?
        .json::<OpenMeteoResponse>()
        .await
        .map_err(|error| format!("Open-Meteo response was invalid: {error}"))?;

    Ok(WeatherObservation {
        location: "Münster, Germany".to_string(),
        temperature_c: response.current.temperature_2m,
        apparent_temperature_c: response.current.apparent_temperature,
        precipitation_mm: response.current.precipitation,
        wind_speed_kmh: response.current.wind_speed_10m,
        weather_code: response.current.weather_code,
        observed_at: response.current.time,
    })
}

fn weather_message(observation: &WeatherObservation) -> ConversationMessage {
    let joke = if observation.precipitation_mm > 0.0 {
        "Münster brought its own water feature today."
    } else if observation.temperature_c < 10.0 {
        "A crisp forecast: even the bicycles may want tiny scarves."
    } else {
        "Good cycling weather, assuming the wind has not joined a sprint team."
    };

    ConversationMessage {
        role: "assistant".to_string(),
        blocks: vec![
            RichTextBlock::Heading {
                level: 3,
                text: "Weather in Münster".to_string(),
            },
            RichTextBlock::Paragraph {
                text: format!(
                    "Right now it is {:.1}°C and feels like {:.1}°C.",
                    observation.temperature_c, observation.apparent_temperature_c
                ),
            },
            RichTextBlock::BulletList {
                items: vec![
                    format!("Precipitation: {:.1} mm", observation.precipitation_mm),
                    format!("Wind: {:.1} km/h", observation.wind_speed_kmh),
                    format!("Observed: {} local time", observation.observed_at),
                ],
            },
            RichTextBlock::Paragraph {
                text: joke.to_string(),
            },
        ],
        tool_calls: vec![ToolCall {
            tool_call_id: next_id("tool"),
            tool_name: "fetch_weather".to_string(),
            status: "completed".to_string(),
            summary: format!(
                "Fetched current conditions for {} from Open-Meteo",
                observation.location
            ),
        }],
    }
}

fn message_plain_text(message: &ConversationMessage) -> String {
    message
        .blocks
        .iter()
        .map(|block| match block {
            RichTextBlock::Paragraph { text } | RichTextBlock::Heading { text, .. } => text.clone(),
            RichTextBlock::BulletList { items } => items.join(" · "),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn lifecycle(
    State(state): State<AppState>,
    Path((session_id, server_id)): Path<(String, String)>,
    Json(request): Json<LifecycleRequest>,
) -> Result<Json<LifecycleResponse>, (StatusCode, String)> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions.get(&session_id).ok_or((
        StatusCode::NOT_FOUND,
        "session not found; MCP lifecycle is session-bound".to_string(),
    ))?;
    let _worktree_id = &session.worktree_id;
    let _action = request.action;
    Ok(Json(LifecycleResponse {
        event_id: next_id("event"),
        state: "accepted".to_string(),
        diagnostic: Some(format!(
            "MCP server {server_id} lifecycle is pending provider integration"
        )),
    }))
}

fn next_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    format!("{prefix}-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_200() {
        let app = router("frontend/dist");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn timeline_contract_correlates_message_prompt_and_request() {
        let app = router("frontend/dist");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"mode":"chat","worktree_id":"wt-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let session: CreateSessionResponse = serde_json::from_slice(&body).unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/sessions/{}/messages", session.session_id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"message_id":"msg-1","content":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let message: MessageResponse = serde_json::from_slice(&body).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/sessions/{}/timeline", session.session_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let timeline: TimelineResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(timeline.session_id, session.session_id);
        assert_eq!(timeline.events.len(), 4);
        assert_eq!(timeline.events[0].message_id.as_deref(), Some("msg-1"));
        assert_eq!(
            timeline.events[1].prompt_assembly_id.as_deref(),
            Some(message.prompt_assembly_id.as_str())
        );
        assert_eq!(
            timeline.events[2].model_request_id.as_deref(),
            Some(message.model_request_id.as_str())
        );
        assert_eq!(
            timeline.events[3].evidence_kind,
            EvidenceKind::ModelResponse
        );
        assert!(!message.prompt_assembly_id.is_empty());
    }

    #[tokio::test]
    async fn prompt_provenance_preserves_source_identity_and_reason() {
        let app = router("frontend/dist");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"mode":"chat","worktree_id":"wt-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let session: CreateSessionResponse =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let response = app.clone()
            .oneshot(Request::builder()
                .method("POST")
                .uri(format!("/api/sessions/{}/messages", session.session_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message_id":"msg-2","content":"hi","prompt_sources":[{"source_kind":"skill","source_id":"rust-best-practices","revision":"v1","path":".agents/skills/rust-best-practices/SKILL.md","inclusion_reason":"selected for Rust implementation"}]}"#))
                .unwrap())
            .await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/sessions/{}/timeline", session.session_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let timeline: TimelineResponse =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(
            timeline.events[1].prompt_sources[0].source_id,
            "rust-best-practices"
        );
        assert_eq!(
            timeline.events[1].prompt_sources[0].inclusion_reason,
            "selected for Rust implementation"
        );
    }

    #[tokio::test]
    async fn list_sessions_returns_session_summaries() {
        let app = router("frontend/dist");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"mode":"chat","worktree_id":"wt-list"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/sessions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let sessions: Vec<SessionSummary> =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert!(!sessions.is_empty());
    }

    #[test]
    fn redaction_removes_secret_assignments() {
        assert_eq!(
            redact_content("hello API_KEY=hidden world"),
            "hello [REDACTED] world"
        );
    }

    #[test]
    fn weather_response_is_ordered_rich_text_with_tool_attribution() {
        let message = weather_message(&WeatherObservation {
            location: "Münster, Germany".to_string(),
            temperature_c: 17.4,
            apparent_temperature_c: 16.8,
            precipitation_mm: 0.0,
            wind_speed_kmh: 12.3,
            weather_code: 2,
            observed_at: "2026-09-17T14:00".to_string(),
        });

        assert_eq!(message.role, "assistant");
        assert!(matches!(
            message.blocks[0],
            RichTextBlock::Heading { level: 3, .. }
        ));
        assert!(matches!(
            message.blocks[2],
            RichTextBlock::BulletList { .. }
        ));
        assert_eq!(message.tool_calls[0].tool_name, "fetch_weather");
        assert!(message_plain_text(&message).contains("Good cycling weather"));
    }

    #[tokio::test]
    async fn mcp_lifecycle_fails_closed_for_unknown_session() {
        let response = router("frontend/dist")
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions/missing/mcp-servers/server/lifecycle")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"action":"start"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn evidence_and_credits_are_separate_read_models() {
        let app = router("frontend/dist");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"mode":"autonomous","worktree_id":"wt-2"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let session: CreateSessionResponse =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/sessions/{}/evidence", session.session_id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"evidence_kind":"validation","state":"passed","source_id":"test-run"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/sessions/{}/credits", session.session_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let credits: CreditUsage =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert!(credits
            .missing_telemetry
            .contains(&"cache_read_tokens".to_string()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/sessions/{}/reconnect", session.session_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn message_contract_generates_model_response_and_session_listing() {
        let app = router("frontend/dist");

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"mode":"chat","worktree_id":"wt-3"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let session: CreateSessionResponse = serde_json::from_slice(
            &create_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();

        let message_response = app.clone().oneshot(Request::builder()
            .method("POST")
            .uri(format!("/api/sessions/{}/messages", session.session_id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"message_id":"msg-3","content":"help me","prompt_sources":[{"source_kind":"skill","source_id":"rust-best-practices","inclusion_reason":"selected for Rust implementation"}]}"#))
            .unwrap()).await.unwrap();
        assert_eq!(message_response.status(), StatusCode::OK);
        let message: MessageResponse = serde_json::from_slice(
            &message_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        assert!(!message.model_response_id.is_empty());
        assert!(message.model_response_text.contains("help me"));

        let timeline_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/sessions/{}/timeline", session.session_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let timeline: TimelineResponse = serde_json::from_slice(
            &timeline_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        assert_eq!(timeline.events.len(), 4);
        assert!(timeline
            .events
            .iter()
            .any(|event| event.evidence_kind == EvidenceKind::ModelResponse));

        let list_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/sessions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let sessions: Vec<SessionSummary> = serde_json::from_slice(
            &list_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, session.session_id);
    }
}
