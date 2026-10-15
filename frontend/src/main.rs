use dioxus::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response as WebResponse};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut input = use_signal(String::new);
    let mut panel = use_signal(|| "conversation");
    use_effect(shell::install_focus_guard);
    use_drop(shell::remove_focus_guard);
    let session_id = use_signal(shell::selected_session);
    let sessions = use_signal(Vec::<SessionSummary>::new);
    let timeline = use_signal(|| None::<Result<TimelineResponse, String>>);
    let credits = use_signal(|| None::<Result<CreditUsage, String>>);
    let lifecycle_status = use_signal(|| None::<Result<String, String>>);
    let action_session_id = session_id;
    let action_result = timeline;
    let session_list = sessions;
    let tree = use_memo(move || navigation::tree(&sessions.read()));
    use_future(move || navigation::refresh(session_list, action_session_id, action_result));

    let refresh_sessions =
        move |_| navigation::refresh(session_list, action_session_id, action_result);

    let create_new_session = move |_| {
        let mut session_list = session_list;
        let mut active_session = action_session_id;
        let mut timeline_result = action_result;
        async move {
            match create_session(None).await {
                Ok(id) => {
                    shell::remember_session(&id);
                    active_session.set(Some(id.clone()));
                    timeline_result.set(None);
                    match list_sessions().await {
                        Ok(list) => session_list.set(list),
                        Err(error) => timeline_result.set(Some(Err(error))),
                    }
                }
                Err(error) => timeline_result.set(Some(Err(error))),
            }
        }
    };

    let create_child_session = move |_| {
        let mut session_list = session_list;
        let mut active_session = action_session_id;
        let mut timeline_result = action_result;
        async move {
            let Some(parent) = active_session() else {
                timeline_result.set(Some(Err("select a parent session first".to_string())));
                return;
            };
            match create_session(Some(parent)).await {
                Ok(id) => {
                    shell::remember_session(&id);
                    active_session.set(Some(id.clone()));
                    timeline_result.set(None);
                    match list_sessions().await {
                        Ok(list) => session_list.set(list),
                        Err(error) => timeline_result.set(Some(Err(error))),
                    }
                }
                Err(error) => timeline_result.set(Some(Err(error))),
            }
        }
    };

    let send = move |_| {
        let message = input();
        let mut session = action_session_id;
        let mut result = action_result;
        async move {
            let current_session = match session() {
                Some(id) => id,
                None => match create_session(None).await {
                    Ok(id) => {
                        shell::remember_session(&id);
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
                    result.set(Some(load_timeline(&current_session).await));
                    let mut session_list = session_list;
                    match list_sessions().await {
                        Ok(list) => session_list.set(list),
                        Err(error) => result.set(Some(Err(error))),
                    }
                }
                Err(error) => result.set(Some(Err(error))),
            }
        }
    };

    rsx! {
        div { class: "app-shell panel-{panel}",
            header { class: "titlebar",
                div { class: "brand-mark", "CA" }
                div { class: "titlebar-copy",
                    strong { "Context Agent" }
                    span { "Weather workspace" }
                }
                div { class: "titlebar-actions",
                    button { class: "panel-toggle", aria_label: "Show sessions", aria_controls: "session-navigation",
                        aria_expanded: panel() == "sessions", onclick: move |_| panel.set(if panel() == "sessions" { "conversation" } else { "sessions" }), "Sessions" }
                    button { class: "panel-toggle", aria_label: "Show inspector", aria_controls: "run-inspector",
                        aria_expanded: panel() == "inspector", onclick: move |_| panel.set(if panel() == "inspector" { "conversation" } else { "inspector" }), "Inspector" }
                    span { class: "signal signal-success", "Connected" }
                    button { class: "icon-button", aria_label: "Refresh sessions", title: "Refresh sessions", onclick: refresh_sessions, "↻" }
                }
            }
            main { class: "workspace",
                aside { class: "session-rail", id: "session-navigation", aria_label: "Session navigation",
                    div { class: "pane-heading",
                        span { "SESSIONS" }
                        button { class: "panel-toggle", aria_label: "Close sessions", onclick: move |_| { panel.set("conversation"); shell::focus_composer(); }, "Close" }
                        button { class: "icon-button", aria_label: "New session", title: "New session", onclick: create_new_session, "+" }
                        button { class: "icon-button", aria_label: "New child session", title: "New child session", onclick: create_child_session, "↳" }
                    }
                    div { class: "session-list", role: "tree", aria_label: "Sessions",
                        for (session, depth) in tree.read().iter().cloned() {
                            SessionTab {
                                session: session.clone(),
                                depth: Some(depth),
                                is_active: action_session_id() == Some(session.session_id.clone()),
                                on_select: move |session_id: String| {
                                    let mut selected = action_session_id;
                                    let mut result = action_result;
                                    async move {
                                        shell::remember_session(&session_id);
                                        selected.set(Some(session_id.clone()));
                                        result.set(None);
                                        panel.set("conversation");
                                        shell::focus_composer();
                                        let loaded = load_timeline(&session_id).await;
                                        if selected().as_deref() == Some(session_id.as_str()) {
                                            result.set(Some(loaded));
                                        }
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
                                        let mut selected = action_session_id;
                                        let mut result = action_result;
                                        async move {
                                            shell::remember_session(&session_id);
                                            selected.set(Some(session_id.clone()));
                                            result.set(None);
                                            let loaded = load_timeline(&session_id).await;
                                            if selected().as_deref() == Some(session_id.as_str()) {
                                                result.set(Some(loaded));
                                            }
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
                aside { class: "inspector-pane", id: "run-inspector", aria_label: "Run inspector",
                    div { class: "pane-heading", span { "RUN INSPECTOR" }
                        button { class: "panel-toggle", aria_label: "Close inspector", onclick: move |_| { panel.set("conversation"); shell::focus_composer(); }, "Close" }
                    }
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
                                let input = usage.input_tokens.map(|n| n.to_string()).unwrap_or_else(|| "unknown".to_string());
                                let output = usage.output_tokens.map(|n| n.to_string()).unwrap_or_else(|| "unknown".to_string());
                                rsx! {
                                    dl {
                                        dt { "Input (last observed)" } dd { "{input}" }
                                        dt { "Output (last observed)" } dd { "{output}" }
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
                                let mut credits = credits;
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
                                let mut lifecycle_status = lifecycle_status;
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

mod models;
use models::*;
mod client;
use client::*;
mod conversation;
use conversation::*;
mod navigation;
mod shell;
