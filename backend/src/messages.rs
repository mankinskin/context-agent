use super::*;

pub(super) async fn create_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<MessageRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    if request.message_id.trim().is_empty() || request.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "message_id_and_content_required".to_string(),
        ));
    }
    hydrate_sessions(&state)?;
    let mut session = state
        .sessions
        .lock()
        .map_err(lock_error)?
        .get(&session_id)
        .cloned()
        .ok_or((StatusCode::NOT_FOUND, "session not found".to_string()))?;
    if let Some(previous) = session.events.iter().find(|event| {
        event.message_id.as_deref() == Some(request.message_id.as_str())
            && event.evidence_kind == EvidenceKind::ChatMessage
    }) {
        let original = previous.message.as_ref().map(message_plain_text);
        let assembly = session.events.iter().find(|event| {
            event.message_id.as_deref() == Some(request.message_id.as_str())
                && event.evidence_kind == EvidenceKind::PromptAssembly
        });
        if original.as_deref() != Some(redact_content(&request.content).as_str())
            || !assembly.is_some_and(|event| {
                event.source_id == request.agent_template_id
                    && event.prompt_sources == request.prompt_sources
            })
        {
            return Err((
                StatusCode::CONFLICT,
                "message_identity_content_conflict".to_string(),
            ));
        }
        let response = session.events.iter().find(|event| {
            event.message_id.as_deref() == Some(request.message_id.as_str())
                && event.evidence_kind == EvidenceKind::ModelResponse
        });
        let response = response.ok_or((StatusCode::SERVICE_UNAVAILABLE,
            "message_already_attempted: inspect the persisted failure or interrupted request; submit a new identity to retry".to_string()))?;
        let assistant_message = response.message.clone().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored_response_incomplete".to_string(),
        ))?;
        return Ok(Json(MessageResponse {
            message_id: request.message_id,
            prompt_assembly_id: response.prompt_assembly_id.clone().ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "stored_response_missing_prompt".to_string(),
            ))?,
            model_request_id: response.model_request_id.clone().ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "stored_response_missing_request".to_string(),
            ))?,
            model_response_id: response.event_id.clone(),
            model_response_text: message_plain_text(&assistant_message),
            assistant_message,
        }));
    }
    let prompt_assembly_id = next_id("prompt");
    let model_request_id = next_id("request");
    let model_response_id = next_id("response");
    let run_id = next_id("run");
    let prompt_sources = request.prompt_sources.clone();
    {
        let mut add_event =
            |kind, message_id, prompt_id, request_id, source_id, sources, message| {
                let sequence = session.events.len() as u64;
                session.events.push(TimelineEvent {
                    event_id: next_id("event"),
                    session_id: session_id.clone(),
                    run_id: Some(run_id.clone()),
                    message_id,
                    model_request_id: request_id,
                    tool_call_id: None,
                    prompt_assembly_id: prompt_id,
                    provider_model_id: None,
                    evidence_kind: kind,
                    sequence,
                    state: "recorded".to_string(),
                    source_id,
                    prompt_sources: sources,
                    message,
                });
            };
        let user_content = redact_content(&request.content);
        add_event(
            EvidenceKind::ChatMessage,
            Some(request.message_id.clone()),
            None,
            None,
            None,
            Vec::new(),
            Some(ConversationMessage {
                role: "user".to_string(),
                blocks: vec![RichTextBlock::Paragraph { text: user_content }],
                tool_calls: Vec::new(),
            }),
        );
        add_event(
            EvidenceKind::PromptAssembly,
            Some(request.message_id.clone()),
            Some(prompt_assembly_id.clone()),
            None,
            request.agent_template_id.clone(),
            prompt_sources.clone(),
            None,
        );
        add_event(
            EvidenceKind::ModelRequest,
            Some(request.message_id.clone()),
            Some(prompt_assembly_id.clone()),
            Some(model_request_id.clone()),
            None,
            Vec::new(),
            None,
        );
    }
    persist_session(&state.store, &session_id, &session)?;
    let is_munster_weather = {
        let content = request.content.to_lowercase();
        content.contains("weather") && (content.contains("munster") || content.contains("münster"))
    };
    let result = if is_munster_weather {
        fetch_weather("Munster")
            .await
            .map(|observation| weather_message(&observation))
            .map_err(|_| {
                (
                    StatusCode::BAD_GATEWAY,
                    "weather_fetch_failed: retry with a new message identity".to_string(),
                )
            })
    } else {
        execute_message(&state, &request.content).await
    };
    let mut completed = session.events.last().cloned().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "stored_request_missing".to_string(),
    ))?;
    completed.event_id = model_response_id.clone();
    completed.sequence = session.events.len() as u64;
    match &result {
        Ok(message) => {
            completed.evidence_kind = EvidenceKind::ModelResponse;
            completed.message = Some(message.clone());
        }
        Err((_, code)) => {
            completed.evidence_kind = EvidenceKind::Failure;
            completed.state = "failed".to_string();
            completed.source_id = Some(code.clone());
        }
    }
    session.events.push(completed);
    persist_session(&state.store, &session_id, &session)?;
    let assistant_message = result?;
    let model_response_text = message_plain_text(&assistant_message);
    Ok(Json(MessageResponse {
        message_id: request.message_id,
        prompt_assembly_id,
        model_request_id,
        model_response_id,
        model_response_text,
        assistant_message,
    }))
}

async fn execute_message(
    state: &AppState,
    content: &str,
) -> Result<ConversationMessage, (StatusCode, String)> {
    #[cfg(test)]
    {
        let _ = state.live;
        Ok(ConversationMessage {
            role: "assistant".to_string(),
            blocks: vec![RichTextBlock::Paragraph {
                text: format!("Test executor response: {}", redact_content(content)),
            }],
            tool_calls: Vec::new(),
        })
    }
    #[cfg(not(test))]
    {
        if !state.live {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "offline_ai_unavailable: no model executor is configured".to_string(),
            ));
        }
        live_agent_response(content).await
    }
}
