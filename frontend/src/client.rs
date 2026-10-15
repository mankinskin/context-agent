use super::*;

async fn request_json<T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<String>,
) -> Result<T, String> {
    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::SameOrigin);
    if let Some(body) = body {
        opts.set_body(&JsValue::from_str(&body));
    }

    let request = Request::new_with_str_and_init(path, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or("no window")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: WebResponse = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        let body = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?;
        let body = body.as_string().ok_or("error response was not text")?;
        return Err(format!("request failed: {}: {body}", resp.status()));
    }

    let json_value = JsFuture::from(resp.json().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text = js_sys::JSON::stringify(&json_value)
        .map(|s| s.as_string().unwrap_or_default())
        .map_err(|e| format!("{e:?}"))?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub(super) async fn list_sessions() -> Result<Vec<SessionSummary>, String> {
    request_json("GET", "/api/sessions", None).await
}

pub(super) async fn create_session(parent_session_id: Option<String>) -> Result<String, String> {
    let body = serde_json::to_string(&CreateSessionRequest {
        mode: "chat".to_string(),
        worktree_id: "browser-default".to_string(),
        parent_session_id,
    })
    .map_err(|e| e.to_string())?;
    Ok(
        request_json::<CreateSessionResponse>("POST", "/api/sessions", Some(body))
            .await?
            .session_id,
    )
}

pub(super) async fn send_message(
    session_id: &str,
    content: String,
) -> Result<MessageResponse, String> {
    let body = serde_json::to_string(&MessageRequest {
        message_id: format!("browser-message-{content}"),
        content,
        agent_template_id: Some("weather-editor".to_string()),
        prompt_sources: vec![PromptSource {
            source_kind: "skill".to_string(),
            source_id: "rust-best-practices".to_string(),
            revision: None,
            path: Some(".agents/skills/rust-best-practices/SKILL.md".to_string()),
            inclusion_reason: "selected for Rust implementation".to_string(),
        }],
    })
    .map_err(|e| e.to_string())?;
    request_json(
        "POST",
        &format!("/api/sessions/{session_id}/messages"),
        Some(body),
    )
    .await
}

pub(super) async fn load_timeline(session_id: &str) -> Result<TimelineResponse, String> {
    request_json("GET", &format!("/api/sessions/{session_id}/timeline"), None).await
}

pub(super) async fn load_credits(session_id: &str) -> Result<CreditUsage, String> {
    request_json("GET", &format!("/api/sessions/{session_id}/credits"), None).await
}

pub(super) async fn lifecycle(session_id: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct LifecycleRequest {
        action: String,
    }
    #[derive(Deserialize)]
    struct LifecycleResponse {
        state: String,
        diagnostic: Option<String>,
    }
    let body = serde_json::to_string(&LifecycleRequest {
        action: "start".to_string(),
    })
    .map_err(|e| e.to_string())?;
    let response: LifecycleResponse = request_json(
        "POST",
        &format!("/api/sessions/{session_id}/mcp-servers/default/lifecycle"),
        Some(body),
    )
    .await?;
    Ok(response.diagnostic.unwrap_or(response.state))
}
