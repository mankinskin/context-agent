use super::*;

#[derive(Clone)]
pub(super) struct Session {
    pub(super) worktree_id: String,
    pub(super) parent_session_id: Option<String>,
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
    pub(super) events: Vec<TimelineEvent>,
    pub(super) usage: Usage,
}

#[derive(Clone, Default)]
pub(super) struct Usage {
    pub(super) input_tokens: u64,
    pub(super) output_tokens: u64,
    pub(super) cache_read_tokens: Option<u64>,
    pub(super) cache_write_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub mode: SessionMode,
    pub worktree_id: String,
    pub parent_session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    Chat,
    Autonomous,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimelineEvent {
    pub event_id: String,
    pub session_id: String,
    pub run_id: Option<String>,
    pub message_id: Option<String>,
    pub model_request_id: Option<String>,
    pub tool_call_id: Option<String>,
    pub prompt_assembly_id: Option<String>,
    pub provider_model_id: Option<String>,
    pub evidence_kind: EvidenceKind,
    pub sequence: u64,
    pub state: String,
    pub source_id: Option<String>,
    pub prompt_sources: Vec<PromptSource>,
    pub message: Option<ConversationMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ConversationMessage {
    pub role: String,
    pub blocks: Vec<RichTextBlock>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichTextBlock {
    Paragraph { text: String },
    Heading { level: u8, text: String },
    BulletList { items: Vec<String> },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct WeatherObservation {
    pub location: String,
    pub temperature_c: f64,
    pub apparent_temperature_c: f64,
    pub precipitation_mm: f64,
    pub wind_speed_kmh: f64,
    pub weather_code: u16,
    pub observed_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PromptSource {
    pub source_kind: String,
    pub source_id: String,
    pub revision: Option<String>,
    pub path: Option<String>,
    pub inclusion_reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    ChatMessage,
    PromptAssembly,
    ModelRequest,
    ModelResponse,
    ToolCall,
    ReasoningTrace,
    Validation,
    Artifact,
    Failure,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineResponse {
    pub session_id: String,
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub state: String,
    pub worktree_id: String,
    pub parent_session_id: Option<String>,
    pub event_count: usize,
    pub latest_message: Option<String>,
    #[serde(skip_serializing, skip_deserializing, default = "chrono::Utc::now")]
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreditUsage {
    pub provider_model_id: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub known_cost: Option<f64>,
    pub missing_telemetry: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct EvidenceRequest {
    pub evidence_kind: EvidenceKind,
    pub state: String,
    pub source_id: Option<String>,
    pub provider_model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    pub message_id: String,
    pub content: String,
    pub agent_template_id: Option<String>,
    #[serde(default)]
    pub prompt_sources: Vec<PromptSource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message_id: String,
    pub prompt_assembly_id: String,
    pub model_request_id: String,
    pub model_response_id: String,
    pub model_response_text: String,
    pub assistant_message: ConversationMessage,
}

#[derive(Debug, Deserialize)]
pub struct LifecycleRequest {
    pub action: LifecycleAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Start,
    Stop,
    Reload,
}

#[derive(Debug, Serialize)]
pub struct LifecycleResponse {
    pub event_id: String,
    pub state: String,
    pub diagnostic: Option<String>,
}
