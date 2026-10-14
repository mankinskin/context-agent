use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use session_api::SessionStoreConfig;
use tower::ServiceExt;

async fn request(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"diagnostic": String::from_utf8_lossy(&bytes)}));
    (status, value)
}

async fn create(app: &Router, parent: Option<&str>) -> String {
    let (status, body) = request(
        app,
        "POST",
        "/api/sessions",
        json!({"mode":"chat", "worktree_id":"bridge-worktree", "parent_session_id":parent}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["session_id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn native_parent_and_failed_turns_survive_a_new_router_without_duplicate_submission() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join(".session");
    let app = backend::router_with_store("unused-static", root.clone());
    let parent = create(&app, None).await;
    let child = create(&app, Some(&parent)).await;
    let path = format!("/api/sessions/{child}/messages");
    let input = json!({"message_id":"stable-message-1","content":"offline bridge input"});
    let (first, second) = tokio::join!(
        request(&app, "POST", &path, input.clone()),
        request(&app, "POST", &path, input.clone())
    );
    assert_eq!(first.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(second.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(first.1["status"], 503);
    assert!(!first.1["code"].as_str().unwrap().is_empty());
    assert!(!first.1["details"].as_array().unwrap().is_empty());
    assert!(!first.1["correlation_id"].as_str().unwrap().is_empty());
    let native = SessionStoreConfig::new(&root).read_session(&child).unwrap();
    assert_eq!(native.source, "context-agent");
    assert_eq!(native.parent_session_id.as_deref(), Some(parent.as_str()));
    assert_eq!(
        native.metadata.workspace_path,
        fixture.path().to_string_lossy()
    );
    assert_eq!(native.turns.len(), 4);
    assert_eq!(
        native.turns.iter().map(|t| t.sequence).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(
        native.turns[0]
            .event_meta
            .as_ref()
            .unwrap()
            .message_id
            .as_deref(),
        Some("stable-message-1")
    );
    let restarted = backend::router_with_store("unused-static", root);
    let (status, timeline) = request(
        &restarted,
        "GET",
        &format!("/api/sessions/{child}/reconnect"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(timeline["events"].as_array().unwrap().len(), 4);
    assert_eq!(timeline["events"][3]["evidence_kind"], "failure");
    let (status, sessions) = request(&restarted, "GET", "/api/sessions", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert!(sessions
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["session_id"] == child && row["parent_session_id"] == parent));
}

#[tokio::test]
async fn native_parent_cycle_is_rejected_when_creating_a_child() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join(".session");
    let app = backend::router_with_store("unused-static", root.clone());
    let parent = create(&app, None).await;
    let store = SessionStoreConfig::new(root);
    let mut record = store.read_session(&parent).unwrap();
    record.parent_session_id = Some(parent.clone());
    let paths = store.paths_for(&record).unwrap();
    session_api::SessionStorePlan {
        record,
        paths,
        events: None,
    }
    .persist()
    .unwrap();
    let (status, _) = request(
        &app,
        "POST",
        "/api/sessions",
        json!({"mode":"chat", "worktree_id":"bridge-worktree", "parent_session_id":parent}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn stable_message_identity_cannot_be_reused_for_different_content() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join(".session");
    let app = backend::router_with_store("unused-static", root.clone());
    let id = create(&app, None).await;
    let path = format!("/api/sessions/{id}/messages");
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            json!({"message_id":"stable","content":"first"})
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            json!({"message_id":"stable","content":"different"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        SessionStoreConfig::new(root)
            .read_session(&id)
            .unwrap()
            .turns
            .len(),
        4
    );
}

#[tokio::test]
async fn missing_parent_is_rejected_without_writing_a_session() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join(".session");
    let app = backend::router_with_store("unused-static", root);
    let (status, _) = request(
        &app,
        "POST",
        "/api/sessions",
        json!({
        "mode":"chat", "worktree_id":"bridge-worktree",
        "parent_session_id":"10000000-0000-0000-0000-000000000099"}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, sessions) = request(&app, "GET", "/api/sessions", Value::Null).await;
    assert!(sessions.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn concurrent_distinct_messages_have_contiguous_durable_sequences() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join(".session");
    let app = backend::router_with_store("unused-static", root.clone());
    let id = create(&app, None).await;
    let path = format!("/api/sessions/{id}/messages");
    let (first, second) = tokio::join!(
        request(
            &app,
            "POST",
            &path,
            json!({"message_id":"a","content":"first"})
        ),
        request(
            &app,
            "POST",
            &path,
            json!({"message_id":"b","content":"second"})
        )
    );
    assert_eq!(first.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(second.0, StatusCode::SERVICE_UNAVAILABLE);
    let native = SessionStoreConfig::new(root).read_session(&id).unwrap();
    assert_eq!(
        native
            .turns
            .iter()
            .map(|turn| turn.sequence)
            .collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>()
    );
}
