use super::*;

#[derive(Serialize)]
pub(super) struct CreateSessionRequest {
    pub(super) mode: String,
    pub(super) worktree_id: String,
}

#[derive(Deserialize)]
pub(super) struct CreateSessionResponse {
    pub(super) session_id: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub(super) struct SessionSummary {
    pub(super) session_id: String,
    pub(super) state: String,
    pub(super) worktree_id: String,
    pub(super) event_count: usize,
    pub(super) latest_message: Option<String>,
}

#[derive(Serialize)]
pub(super) struct MessageRequest {
    pub(super) message_id: String,
    pub(super) content: String,
    pub(super) agent_template_id: Option<String>,
    #[serde(default)]
    pub(super) prompt_sources: Vec<PromptSource>,
}

#[derive(Deserialize, Clone, Debug)]
pub(super) struct MessageResponse {
    #[serde(rename = "assistant_message")]
    pub(super) _assistant_message: ConversationMessage,
}

#[derive(Deserialize, Clone)]
pub(super) struct TimelineResponse {
    #[serde(rename = "session_id")]
    pub(super) _session_id: String,
    pub(super) events: Vec<TimelineEvent>,
}

#[derive(Deserialize, Clone)]
pub(super) struct CreditUsage {
    pub(super) input_tokens: u64,
    pub(super) output_tokens: u64,
    pub(super) missing_telemetry: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub(super) struct TimelineEvent {
    pub(super) sequence: u64,
    pub(super) message: Option<ConversationMessage>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub(super) struct ConversationMessage {
    pub(super) role: String,
    pub(super) blocks: Vec<RichTextBlock>,
    pub(super) tool_calls: Vec<ToolCall>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum RichTextBlock {
    Paragraph { text: String },
    Heading { level: u8, text: String },
    BulletList { items: Vec<String> },
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub(super) struct ToolCall {
    pub(super) tool_call_id: String,
    pub(super) tool_name: String,
    pub(super) status: String,
    pub(super) summary: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct PromptSource {
    pub(super) source_kind: String,
    pub(super) source_id: String,
    pub(super) revision: Option<String>,
    pub(super) path: Option<String>,
    pub(super) inclusion_reason: String,
}
