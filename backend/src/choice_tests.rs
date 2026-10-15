use super::*;
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn call(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
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
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn preambles_persist_validate_and_only_enabled_sources_are_used() {
    let root = std::env::temp_dir().join(format!("ca-preamble-{}", uuid::Uuid::new_v4()));
    let app = router_with_store("", root.clone());
    let config = json!([
        {"id":"PREAMBLE_ENABLED_29AD","text":"enabled instruction","enabled":true},
        {"id":"PREAMBLE_DISABLED_29AD","text":"disabled instruction","enabled":false}
    ]);
    assert_eq!(
        call(&app, "PUT", "/api/preambles", config.clone()).await.0,
        StatusCode::OK
    );
    let restored = router_with_store("", root.clone());
    assert_eq!(
        call(&restored, "GET", "/api/preambles", Value::Null)
            .await
            .1,
        config
    );
    assert_eq!(
        call(
            &app,
            "PUT",
            "/api/preambles",
            json!([
                {"id":"duplicate","text":"one","enabled":true},
                {"id":"duplicate","text":"two","enabled":true}
            ])
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(&restored, "GET", "/api/preambles", Value::Null)
            .await
            .1,
        config
    );
    let (_, session) = call(
        &app,
        "POST",
        "/api/sessions",
        json!({"mode":"chat","worktree_id":"test"}),
    )
    .await;
    let id = session["session_id"].as_str().unwrap();
    call(
        &app,
        "POST",
        &format!("/api/sessions/{id}/messages"),
        json!({"message_id":"config-test","content":"hello"}),
    )
    .await;
    let (_, timeline) = call(
        &app,
        "GET",
        &format!("/api/sessions/{id}/timeline"),
        Value::Null,
    )
    .await;
    assert_eq!(
        timeline["events"][1]["prompt_sources"][0]["source_id"],
        "PREAMBLE_ENABLED_29AD"
    );
    assert!(!timeline["events"][1]
        .to_string()
        .contains("PREAMBLE_DISABLED_29AD"));
    assert_eq!(
        preambles::assembled(
            &serde_json::from_value::<Vec<preambles::PreambleBlock>>(config).unwrap()
        ),
        "[preamble:PREAMBLE_ENABLED_29AD]\nenabled instruction"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn stable_choice_rejects_invalid_and_repeated_answers_and_restores_native_state() {
    let root = std::env::temp_dir().join(format!("ca-choice-{}", uuid::Uuid::new_v4()));
    let app = router_with_store("", root.clone());
    let (_, session) = call(
        &app,
        "POST",
        "/api/sessions",
        json!({"mode":"chat","worktree_id":"test"}),
    )
    .await;
    let id = session["session_id"].as_str().unwrap();
    let path = format!("/api/sessions/{id}/messages");
    let (status, initial) = call(
        &app,
        "POST",
        &path,
        json!({"message_id":"initial","content":"lookup","agent_template_id":"choice-mcp"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let widget = &initial["assistant_message"]["blocks"][1];
    assert_eq!(widget["type"], "multiple_choice");
    let widget_id = widget["widget_id"].as_str().unwrap();
    let answer = |choice: &str| {
        json!({"message_id":"answer","content":format!("Selected {choice}. Read the authorized ticket again."),
        "agent_template_id":"choice-answer","choice_answer":{"widget_id":widget_id,"choice":choice}})
    };
    assert_eq!(
        call(&app, "POST", &path, answer("invalid")).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(&app, "POST", &path, answer("lookup-a")).await.0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, "POST", &path, answer("lookup-a")).await.0,
        StatusCode::CONFLICT
    );
    let restored = router_with_store("", root.clone());
    let (_, timeline) = call(
        &restored,
        "GET",
        &format!("/api/sessions/{id}/reconnect"),
        Value::Null,
    )
    .await;
    let events = timeline["events"].as_array().unwrap();
    let blocks = events
        .iter()
        .filter_map(|event| event["message"]["blocks"].as_array())
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(
        blocks
            .iter()
            .filter(|block| block["type"] == "multiple_choice")
            .count(),
        1
    );
    assert_eq!(
        blocks
            .iter()
            .filter(|block| block["type"] == "choice_answer")
            .count(),
        1
    );
    assert!(blocks
        .iter()
        .any(|block| block["choice"] == "lookup-a" && block["widget_id"] == widget_id));
    std::fs::remove_dir_all(root).unwrap();
}
