use super::*;

/// Construct the published provider bridge only when a live configuration was
/// explicitly supplied. Its template authorizes the owned MCP/read tools and
/// the request has a bounded three-turn observation budget. Offline requests
/// fail closed rather than returning a fabricated assistant response.
#[cfg(not(test))]
pub(super) async fn live_agent_response(
    content: &str,
) -> Result<ConversationMessage, (StatusCode, String)> {
    let config_path = std::env::var_os("CONTEXT_AGENT_CONFIG").ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "offline_ai_unavailable: no model executor is configured".to_string(),
    ))?;
    let config = Config::load(std::path::Path::new(&config_path)).map_err(|_| {
        tracing::error!("provider configuration could not be loaded");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_configuration_unavailable: repair the server-only configuration".to_string(),
        )
    })?;
    let template = AgentTemplate::load(&config.template_path()).map_err(|_| {
        tracing::error!("provider template could not be loaded");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_template_unavailable: repair the server-only template".to_string(),
        )
    })?;
    let ledger_path = std::env::var_os("CONTEXT_AGENT_BUDGET_LEDGER").ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "provider_budget_required: configure the permanent budget ledger".to_string(),
    ))?;
    let evidence_root = std::env::var_os("CONTEXT_AGENT_EVIDENCE_ROOT").ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "provider_evidence_required: configure an owned evidence directory".to_string(),
    ))?;
    let session_id = std::env::var("CONTEXT_AGENT_PROVIDER_SESSION_ID").map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_identity_required: configure the owning session UUID".to_string(),
        )
    })?;
    if uuid::Uuid::parse_str(&session_id).is_err()
        || template.model != "gpt-5-mini"
        || config
            .model
            .as_deref()
            .is_some_and(|model| model != "gpt-5-mini")
    {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_policy_invalid: require an owning UUID and gpt-5-mini".to_string(),
        ));
    }
    let policy = ProviderExecutionPolicy::for_fixture(
        PathBuf::from(ledger_path),
        PathBuf::from(evidence_root),
        session_id,
    );
    let executor = RigAgentExecutor::from_template_with_provider_execution(
        &config,
        &template,
        |tools| tools,
        policy,
    )
    .await
    .map_err(|_| {
        tracing::error!("bounded provider construction failed");
        (
            StatusCode::BAD_GATEWAY,
            "provider_bridge_unavailable: inspect the owned MCP lifecycle receipts".to_string(),
        )
    })?;
    let response = executor
        .execute(AgentRequest::new(content, 3))
        .await
        .map_err(|_| {
            tracing::error!("bounded provider execution failed");
            (
                StatusCode::BAD_GATEWAY,
                "provider_execution_failed: inspect the redacted owned provider evidence"
                    .to_string(),
            )
        })?;
    Ok(ConversationMessage {
        role: "assistant".to_string(),
        blocks: vec![RichTextBlock::Paragraph {
            text: redact_content(&response.text),
        }],
        tool_calls: Vec::new(),
    })
}
