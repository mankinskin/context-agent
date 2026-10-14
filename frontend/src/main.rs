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
                        Err(error) => timeline_result.set(Some(Err(error))),
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
                        Err(error) => result.set(Some(Err(error))),
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

mod models;
use models::*;
mod client;
use client::*;
mod conversation;
use conversation::*;
