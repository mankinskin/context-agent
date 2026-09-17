use dioxus::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response as WebResponse};

fn main() {
    dioxus::launch(App);
}

#[derive(Serialize)]
struct CreateSessionRequest {
    mode: String,
    worktree_id: String,
}

#[derive(Deserialize)]
struct CreateSessionResponse {
    session_id: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct SessionSummary {
    session_id: String,
    state: String,
    worktree_id: String,
    event_count: usize,
    latest_message: Option<String>,
}

#[derive(Serialize)]
struct MessageRequest {
    message_id: String,
    content: String,
    agent_template_id: Option<String>,
    #[serde(default)]
    prompt_sources: Vec<PromptSource>,
}

#[derive(Deserialize, Clone, Debug)]
struct MessageResponse {
    #[serde(rename = "assistant_message")]
    _assistant_message: ConversationMessage,
}

#[derive(Deserialize, Clone)]
struct TimelineResponse {
    #[serde(rename = "session_id")]
    _session_id: String,
    events: Vec<TimelineEvent>,
}

#[derive(Deserialize, Clone)]
struct CreditUsage {
    input_tokens: u64,
    output_tokens: u64,
    missing_telemetry: Vec<String>,
}

#[derive(Deserialize, Clone)]
struct TimelineEvent {
    sequence: u64,
    message: Option<ConversationMessage>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ConversationMessage {
    role: String,
    blocks: Vec<RichTextBlock>,
    tool_calls: Vec<ToolCall>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RichTextBlock {
    Paragraph { text: String },
    Heading { level: u8, text: String },
    BulletList { items: Vec<String> },
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct ToolCall {
    tool_call_id: String,
    tool_name: String,
    status: String,
    summary: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct PromptSource {
    source_kind: String,
    source_id: String,
    revision: Option<String>,
    path: Option<String>,
    inclusion_reason: String,
}

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
        return Err(format!("request failed: {}", resp.status()));
    }

    let json_value = JsFuture::from(resp.json().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text = js_sys::JSON::stringify(&json_value)
        .map(|s| s.as_string().unwrap_or_default())
        .map_err(|e| format!("{e:?}"))?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

async fn list_sessions() -> Result<Vec<SessionSummary>, String> {
    request_json("GET", "/api/sessions", None).await
}

async fn create_session() -> Result<String, String> {
    let body = serde_json::to_string(&CreateSessionRequest {
        mode: "chat".to_string(),
        worktree_id: "browser-default".to_string(),
    })
    .map_err(|e| e.to_string())?;
    Ok(
        request_json::<CreateSessionResponse>("POST", "/api/sessions", Some(body))
            .await?
            .session_id,
    )
}

async fn send_message(session_id: &str, content: String) -> Result<MessageResponse, String> {
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

async fn load_timeline(session_id: &str) -> Result<TimelineResponse, String> {
    request_json("GET", &format!("/api/sessions/{session_id}/timeline"), None).await
}

async fn load_credits(session_id: &str) -> Result<CreditUsage, String> {
    request_json("GET", &format!("/api/sessions/{session_id}/credits"), None).await
}

async fn lifecycle(session_id: &str) -> Result<String, String> {
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

#[component]
fn SessionTab(
    session: SessionSummary,
    is_active: bool,
    on_select: EventHandler<String>,
) -> Element {
    let session_id_for_button = session.session_id.clone();
    let session_number = session_id_for_button
        .rsplit('-')
        .next()
        .unwrap_or(&session_id_for_button);
    let session_label = format!("Session {session_number}");
    let session_title = format!("{} · {}", session.worktree_id, session_id_for_button);
    let key_id = session_id_for_button.clone();
    let click_id = session_id_for_button.clone();
    rsx! {
        button {
            key: "{key_id}",
            class: if is_active { "session-tab is-active" } else { "session-tab" },
            title: "{session_title}",
            aria_pressed: is_active,
            onclick: move |_| {
                on_select.call(click_id.clone());
            },
            span { class: "status-dot" }
            span { class: "session-tab-label", "{session_label}" }
            if is_active {
                span { class: "active-indicator", "Active" }
            }
            span { class: "session-count", "{session.event_count}" }
        }
    }
}

#[component]
fn MessageBlock(block: RichTextBlock) -> Element {
    match block {
        RichTextBlock::Paragraph { text } => rsx! { p { "{text}" } },
        RichTextBlock::Heading { level, text } => match level {
            1 => rsx! { h1 { "{text}" } },
            2 => rsx! { h2 { "{text}" } },
            _ => rsx! { h3 { "{text}" } },
        },
        RichTextBlock::BulletList { items } => rsx! {
            ul { for item in items { li { "{item}" } } }
        },
    }
}

#[component]
fn ConversationEntry(message: ConversationMessage, sequence: u64) -> Element {
    let is_user = message.role == "user";
    let role_label = if is_user { "You" } else { "Weather Editor" };
    rsx! {
        article { class: if is_user { "message message-user" } else { "message message-assistant" },
            div { class: "message-avatar", if is_user { "Y" } else { "W" } }
            div { class: "message-body",
                header { class: "message-meta",
                    strong { "{role_label}" }
                    span { "#{sequence}" }
                }
                div { class: "rich-text",
                    for block in message.blocks { MessageBlock { block } }
                }
                for tool in message.tool_calls {
                    div { class: "tool-call",
                        div { class: "tool-call-head",
                            span { class: "tool-icon", "↗" }
                            code { "{tool.tool_name}" }
                            span { class: "signal signal-success", "{tool.status}" }
                        }
                        p { "{tool.summary}" }
                        span { class: "tool-call-id", "{tool.tool_call_id}" }
                    }
                }
            }
        }
    }
}

#[component]
fn App() -> Element {
    let mut input = use_signal(String::new);
    let session_id = use_signal(|| None::<String>);
    let sessions = use_signal(Vec::<SessionSummary>::new);
    let timeline = use_signal(|| None::<Result<TimelineResponse, String>>);
    let credits = use_signal(|| None::<Result<CreditUsage, String>>);
    let lifecycle_status = use_signal(|| None::<Result<String, String>>);
    let action_session_id = session_id.clone();
    let mut action_result = timeline.clone();
    let session_list = sessions.clone();

    let refresh_sessions = move |_| {
        let mut session_list = session_list.clone();
        let mut active_session = action_session_id.clone();
        async move {
            match list_sessions().await {
                Ok(list) => {
                    let selected = active_session()
                        .or_else(|| list.first().map(|entry| entry.session_id.clone()));
                    if let Some(id) = selected.clone() {
                        active_session.set(Some(id.clone()));
                    }
                    session_list.set(list);
                }
                Err(err) => action_result.set(Some(Err(err))),
            }
        }
    };

    let create_new_session = move |_| {
        let mut session_list = session_list.clone();
        let mut active_session = action_session_id.clone();
        let mut timeline_result = action_result.clone();
        async move {
            match create_session().await {
                Ok(id) => {
                    active_session.set(Some(id.clone()));
                    timeline_result.set(None);
                    match list_sessions().await {
                        Ok(list) => session_list.set(list),
                        Err(_) => {}
                    }
                }
                Err(error) => timeline_result.set(Some(Err(error))),
            }
        }
    };

    let send = move |_| {
        let message = input();
        let mut session = action_session_id.clone();
        let mut result = action_result.clone();
        async move {
            let current_session = match session() {
                Some(id) => id,
                None => match create_session().await {
                    Ok(id) => {
                        session.set(Some(id.clone()));
                        id
                    }
                    Err(error) => {
                        result.set(Some(Err(error)));
                        return;
                    }
                },
            };
            match send_message(&current_session, message).await {
                Ok(_response) => {
                    result.set(Some(
                        load_timeline(&current_session)
                            .await
                            .map(|timeline| timeline),
                    ));
                    let mut session_list = session_list.clone();
                    match list_sessions().await {
                        Ok(list) => session_list.set(list),
                        Err(_) => {}
                    }
                }
                Err(error) => result.set(Some(Err(error))),
            }
        }
    };

    rsx! {
        div { class: "app-shell",
            header { class: "titlebar",
                div { class: "brand-mark", "CA" }
                div { class: "titlebar-copy",
                    strong { "Context Agent" }
                    span { "Weather workspace" }
                }
                div { class: "titlebar-actions",
                    span { class: "signal signal-success", "Connected" }
                    button { class: "icon-button", aria_label: "Refresh sessions", title: "Refresh sessions", onclick: refresh_sessions, "↻" }
                }
            }
            main { class: "workspace",
                aside { class: "session-rail",
                    div { class: "pane-heading",
                        span { "SESSIONS" }
                        button { class: "icon-button", aria_label: "New session", title: "New session", onclick: create_new_session, "+" }
                    }
                    div { class: "session-list",
                        for session in sessions.read().iter().cloned() {
                            SessionTab {
                                session: session.clone(),
                                is_active: action_session_id() == Some(session.session_id.clone()),
                                on_select: move |session_id: String| {
                                    let mut selected = action_session_id.clone();
                                    let mut result = action_result.clone();
                                    async move {
                                        selected.set(Some(session_id.clone()));
                                        result.set(Some(load_timeline(&session_id).await));
                                    }
                                },
                            }
                        }
                    }
                    div { class: "agent-card",
                        span { class: "eyebrow", "DEFAULT AGENT" }
                        strong { "Weather Editor" }
                        p { "Forecasts with concise, dry humor." }
                    }
                }
                section { class: "editor-pane",
                    div { class: "editor-tabbar",
                        div { class: "editor-tabs",
                            for session in sessions.read().iter().cloned() {
                                SessionTab {
                                    session: session.clone(),
                                    is_active: action_session_id() == Some(session.session_id.clone()),
                                    on_select: move |session_id: String| {
                                        let mut selected = action_session_id.clone();
                                        let mut result = action_result.clone();
                                        async move {
                                            selected.set(Some(session_id.clone()));
                                            result.set(Some(load_timeline(&session_id).await));
                                        }
                                    },
                                }
                            }
                        }
                        button {
                            class: "parallel-session-button",
                            aria_label: "Create parallel session",
                            title: "Create a parallel session",
                            onclick: create_new_session,
                            span { class: "parallel-session-icon", "+" }
                            span { "Parallel session" }
                        }
                    }
                    div { class: "conversation-scroll",
                        div { class: "conversation-header",
                            span { class: "eyebrow", "ACTIVE THREAD" }
                            h1 { "Münster weather" }
                            p { "Live conditions, tool activity, and response history." }
                        }
                        match timeline.read().clone() {
                            Some(Ok(timeline)) => rsx! {
                                div { class: "message-stream",
                                    if timeline.events.iter().all(|event| event.message.is_none()) {
                                        div { class: "empty-state", p { "No messages in this session yet." } }
                                    }
                                    for event in timeline.events.iter() {
                                        if let Some(message) = event.message.clone() {
                                            ConversationEntry { message, sequence: event.sequence }
                                        }
                                    }
                                }
                            },
                            Some(Err(err)) => rsx! { div { class: "error-banner", strong { "Request failed" } p { "{err}" } } },
                            None => rsx! {
                                div { class: "empty-state",
                                    span { class: "empty-icon", "☀" }
                                    h2 { "Start with the weather" }
                                    p { "Ask for current conditions in Münster to see a tool-backed response." }
                                }
                            },
                        }
                    }
                    div { class: "composer",
                        div { class: "composer-input",
                            input {
                                aria_label: "Message",
                                placeholder: "How is the weather in Münster today?",
                                value: "{input}",
                                oninput: move |evt| input.set(evt.value()),
                            }
                            button { class: "send-button", onclick: send,
                                span { "Send" }
                                span { "↗" }
                            }
                        }
                        div { class: "composer-meta",
                            span { "Weather Editor" }
                            span { "Open-Meteo · live tool" }
                        }
                    }
                }
                aside { class: "inspector-pane",
                    div { class: "pane-heading", span { "RUN INSPECTOR" } }
                    section { class: "inspector-section",
                        span { class: "eyebrow", "SESSION" }
                        dl {
                            dt { "State" } dd { span { class: "signal signal-success", "Interactive" } }
                            dt { "Agent" } dd { "Weather Editor" }
                            dt { "Tool" } dd { code { "fetch_weather" } }
                        }
                    }
                    section { class: "inspector-section",
                        span { class: "eyebrow", "TELEMETRY" }
                        match credits.read().clone() {
                            Some(Ok(usage)) => {
                                let missing = usage.missing_telemetry.join(", ");
                                rsx! {
                                    dl {
                                        dt { "Input" } dd { "{usage.input_tokens}" }
                                        dt { "Output" } dd { "{usage.output_tokens}" }
                                        dt { "Missing" } dd { "{missing}" }
                                    }
                                }
                            },
                            Some(Err(error)) => rsx! { p { class: "error-text", "{error}" } },
                            None => rsx! { p { class: "muted", "No credit telemetry yet." } },
                        }
                        button {
                            class: "secondary-button",
                            onclick: move |_| {
                                let id = action_session_id();
                                let mut credits = credits.clone();
                                async move {
                                    if let Some(id) = id { credits.set(Some(load_credits(&id).await)); }
                                }
                            },
                            "Load credits"
                        }
                    }
                    section { class: "inspector-section",
                        span { class: "eyebrow", "MCP SERVER" }
                        match lifecycle_status.read().clone() {
                            Some(Ok(status)) => rsx! { p { "{status}" } },
                            Some(Err(error)) => rsx! { p { class: "error-text", "{error}" } },
                            None => rsx! { p { class: "muted", "Not started" } },
                        }
                        button {
                            class: "secondary-button",
                            onclick: move |_| {
                                let id = action_session_id();
                                let mut lifecycle_status = lifecycle_status.clone();
                                async move {
                                    if let Some(id) = id { lifecycle_status.set(Some(lifecycle(&id).await)); }
                                }
                            },
                            "Start MCP"
                        }
                    }
                }
            }
            footer { class: "statusbar",
                span { "main" }
                span { "UTF-8" }
                span { "Context Agent ready" }
            }
        }
    }
}
