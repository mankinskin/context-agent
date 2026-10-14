use super::*;

#[component]
pub(super) fn SessionTab(
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
pub(super) fn MessageBlock(block: RichTextBlock) -> Element {
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
pub(super) fn ConversationEntry(message: ConversationMessage, sequence: u64) -> Element {
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
