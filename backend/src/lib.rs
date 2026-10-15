#[cfg(not(test))]
use agent_builder::provider::ProviderExecutionPolicy;
#[cfg(not(test))]
use agent_builder::{AgentRequest, AgentTemplate, Config, RigAgentExecutor};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use session_api::{
    SessionMetadata, SessionRecord, SessionRole, SessionStoreConfig, SessionStorePlan, SessionTurn,
    SessionTurnEventMeta,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
struct AppState {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    store: SessionStoreConfig,
    transaction: Arc<tokio::sync::Mutex<()>>,
    live: bool,
}

/// Builds the Context Agent API and same-origin static frontend service.
///
/// `SESSION_STORE_ROOT` selects the native session-api store.  It is explicit
/// for the browser runner and defaults to a local `.session` store for manual
/// development.
pub fn router(static_dir: &str) -> Router {
    #[cfg(test)]
    let store_root = std::env::temp_dir().join(format!("context-agent-{}", uuid::Uuid::new_v4()));
    #[cfg(not(test))]
    let store_root = std::env::var_os("SESSION_STORE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".session"));
    build_router(static_dir, store_root, true)
}

pub fn router_with_store(static_dir: &str, store_root: PathBuf) -> Router {
    build_router(static_dir, store_root, false)
}

fn build_router(static_dir: &str, store_root: PathBuf, live: bool) -> Router {
    let state = AppState {
        sessions: Arc::new(Mutex::new(HashMap::new())),
        store: SessionStoreConfig::new(store_root),
        transaction: Arc::new(tokio::sync::Mutex::new(())),
        live,
    };
    let serve_dir = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(format!("{static_dir}/index.html")));
    Router::new()
        .route("/health", get(health))
        .route("/api/sessions", get(list_sessions).post(create_session))
        .route("/api/preambles", get(preambles::get).put(preambles::save))
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
        .layer(axum::middleware::map_response(structured_error))
        .with_state(state)
        .fallback_service(serve_dir)
}

async fn structured_error(response: Response) -> Response {
    let status = response.status();
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }
    let correlation_id = next_id("diagnostic");
    tracing::warn!(%correlation_id, status = status.as_u16(), "application request failed");
    let bytes = match axum::body::to_bytes(response.into_body(), 8192).await {
        Ok(bytes) => bytes,
        Err(_) => {
            tracing::error!("application error body exceeded its diagnostics boundary");
            return (
                status,
                Json(serde_json::json!({
                    "code": "diagnostic_unavailable", "status": status.as_u16(),
                    "message": "The application could not return bounded diagnostics.",
                    "details": ["Inspect the server diagnostic log."],
                    "correlation_id": correlation_id
                })),
            )
                .into_response();
        }
    };
    let message = String::from_utf8_lossy(&bytes);
    let code = message
        .split(':')
        .next()
        .unwrap_or("application_error")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_");
    (status, Json(serde_json::json!({
        "code": code, "status": status.as_u16(), "message": message,
        "details": ["Use the message identity to inspect the persisted timeline; a new identity is required for a new attempt."],
        "correlation_id": correlation_id
    }))).into_response()
}

async fn health() -> &'static str {
    "ok"
}

async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<SessionSummary>>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    hydrate_sessions(&state)?;
    let sessions = state.sessions.lock().map_err(lock_error)?;
    let mut summaries = sessions
        .iter()
        .map(|(session_id, session)| SessionSummary {
            session_id: session_id.clone(),
            state: "interactive".to_string(),
            worktree_id: session.worktree_id.clone(),
            parent_session_id: session.parent_session_id.clone(),
            event_count: session.events.len(),
            latest_message: session
                .events
                .iter()
                .rev()
                .filter(|event| event.evidence_kind == EvidenceKind::ChatMessage)
                .filter_map(|event| event.source_id.clone())
                .next(),
            created_at: session.created_at,
        })
        .collect::<Vec<_>>();
    summaries.sort_by_key(|summary| std::cmp::Reverse(summary.created_at));
    Ok(Json(summaries))
}

async fn create_session(
    State(state): State<AppState>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    hydrate_sessions(&state)?;
    if request.worktree_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "worktree_id is required".to_string(),
        ));
    }
    if let Some(parent) = request.parent_session_id.as_deref() {
        let sessions = state.sessions.lock().map_err(lock_error)?;
        let mut cursor = Some(parent);
        let mut visited = std::collections::HashSet::new();
        while let Some(id) = cursor {
            if !visited.insert(id) {
                return Err((StatusCode::CONFLICT, "session_parent_cycle".to_string()));
            }
            cursor = sessions
                .get(id)
                .ok_or((
                    StatusCode::NOT_FOUND,
                    "parent_session_not_found".to_string(),
                ))?
                .parent_session_id
                .as_deref();
        }
    }
    let session_id = uuid::Uuid::new_v4().to_string();
    let session = Session {
        worktree_id: request.worktree_id,
        parent_session_id: request.parent_session_id,
        created_at: chrono::Utc::now(),
        events: Vec::new(),
        usage: Usage::default(),
    };
    persist_session(&state.store, &session_id, &session)?;
    state
        .sessions
        .lock()
        .map_err(lock_error)?
        .insert(session_id.clone(), session);
    Ok(Json(CreateSessionResponse {
        session_id,
        state: "interactive".to_string(),
    }))
}

async fn get_timeline(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<TimelineResponse>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    hydrate_sessions(&state)?;
    let sessions = state.sessions.lock().map_err(lock_error)?;
    let session = sessions
        .get(&session_id)
        .ok_or((StatusCode::NOT_FOUND, "session not found".to_string()))?;
    Ok(Json(TimelineResponse {
        session_id,
        events: session.events.clone(),
    }))
}

async fn add_evidence(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<EvidenceRequest>,
) -> Result<Json<TimelineEvent>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    hydrate_sessions(&state)?;
    let mut sessions = state.sessions.lock().map_err(lock_error)?;
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
    persist_session(&state.store, &session_id, session)?;
    Ok(Json(event))
}

async fn get_credits(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<CreditUsage>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    hydrate_sessions(&state)?;
    let sessions = state.sessions.lock().map_err(lock_error)?;
    let session = sessions
        .get(&session_id)
        .ok_or((StatusCode::NOT_FOUND, "session not found".to_string()))?;
    let mut missing = Vec::new();
    if session.usage.input_tokens.is_none() {
        missing.push("input_tokens".to_string());
    }
    if session.usage.output_tokens.is_none() {
        missing.push("output_tokens".to_string());
    }
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

fn lock_error<T>(_: std::sync::PoisonError<T>) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "session_state_unavailable".to_string(),
    )
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

async fn lifecycle(
    State(state): State<AppState>,
    Path((session_id, server_id)): Path<(String, String)>,
    Json(request): Json<LifecycleRequest>,
) -> Result<Json<LifecycleResponse>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    hydrate_sessions(&state)?;
    let sessions = state.sessions.lock().map_err(lock_error)?;
    let session = sessions.get(&session_id).ok_or((
        StatusCode::NOT_FOUND,
        "session not found; MCP lifecycle is session-bound".to_string(),
    ))?;
    let _worktree_id = &session.worktree_id;
    let _action = request.action;
    Err((StatusCode::SERVICE_UNAVAILABLE, format!(
        "mcp_lifecycle_unavailable: {server_id} is owned by its bounded model invocation; use the configured message route"
    )))
}

fn next_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}

mod choices;
mod models;
mod preambles;
pub use models::*;
use models::{Session, Usage};
mod messages;
use messages::create_message;
mod persistence;
use persistence::{hydrate_sessions, persist_session};
#[cfg(not(test))]
mod provider;
#[cfg(not(test))]
use provider::live_agent_response;
mod weather;
use weather::{fetch_weather, get_weather, message_plain_text, weather_message};
#[cfg(test)]
mod choice_tests;
#[cfg(test)]
mod tests;
