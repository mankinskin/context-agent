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
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
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
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
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
    for worktree_id in ["wt-list-1", "wt-list-2"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"mode":"chat","worktree_id":"{worktree_id}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

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
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].worktree_id, "wt-list-2");
    assert_eq!(sessions[1].worktree_id, "wt-list-1");
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
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
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
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
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
