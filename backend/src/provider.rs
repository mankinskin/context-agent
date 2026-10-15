use super::*;

/// Construct the published provider bridge only when a live configuration was
/// explicitly supplied. Its template authorizes the owned MCP/read tools and
/// the request has a bounded three-turn observation budget. Offline requests
/// fail closed rather than returning a fabricated assistant response.
#[cfg(not(test))]
pub(super) async fn live_agent_response(
    content: &str,
    preambles: &[preambles::PreambleBlock],
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
    let mut policy = ProviderExecutionPolicy::for_fixture(
        PathBuf::from(ledger_path),
        PathBuf::from(evidence_root),
        session_id,
    );
    if std::env::var_os("CONTEXT_AGENT_OBSERVE_PREAMBLES").is_some() && !preambles.is_empty() {
        use agent_builder::{OutboundPreambleObservation, PreambleProbe};
        let probes = preambles
            .iter()
            .map(|block| PreambleProbe::new(&block.id, format!("[preamble:{}]", block.id)))
            .collect::<Result<Vec<_>, _>>();
        let observation = probes
            .and_then(OutboundPreambleObservation::new)
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "preamble_observation_invalid: repair server observation configuration".into(),
                )
            })?;
        policy = policy.with_outbound_preamble_observation(observation);
    }
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
    let enabled = preambles::assembled(preambles);
    let executor = if enabled.is_empty() {
        executor
    } else {
        executor.with_preamble(format!("{}\n\n{enabled}", template.preamble))
    };
    let (response, observed) = executor
        .execute_observed_with_endpoint(
            AgentRequest::new(content, 3),
            "https://api.githubcopilot.com",
        )
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
        usage: observed.usage.map(|usage| Usage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            ..Usage::default()
        }),
        role: "assistant".to_string(),
        blocks: vec![RichTextBlock::Paragraph {
            text: redact_content(&response.text),
        }],
        tool_calls: observed
            .tool_calls
            .into_iter()
            .map(|call| ToolCall {
                tool_call_id: call.id,
                tool_name: "get_ticket".to_string(),
                status: if call.result.is_some() {
                    "completed"
                } else {
                    "incomplete"
                }
                .to_string(),
                summary: call
                    .result
                    .map(|result| redact_content(&result.to_string()))
                    .unwrap_or_else(|| "No result observed".to_string()),
            })
            .collect(),
    })
}
