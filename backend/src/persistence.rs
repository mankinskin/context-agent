use super::*;

/// Rebuild the in-process read model solely from the durable native store.
/// The JSON in `SessionTurn::content` is an application event envelope; the
/// native turn metadata holds the event and parent identity used by tooling.
pub(super) fn hydrate_sessions(state: &AppState) -> Result<(), (StatusCode, String)> {
    let records = state
        .store
        .query_sessions(&session_api::SessionQuery::default())
        .map_err(store_error)?;
    if !records.diagnostics.is_empty() {
        tracing::error!(
            count = records.diagnostics.len(),
            "native session query has unreadable records"
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "session_store_unreadable".to_string(),
        ));
    }
    let mut restored = HashMap::new();
    for record in records
        .sessions
        .into_iter()
        .filter(|record| record.source == "context-agent")
    {
        let events = record
            .turns
            .iter()
            .map(|turn| serde_json::from_str::<TimelineEvent>(&turn.content).map_err(|_| {
                tracing::error!(session_id = %record.session_id, sequence = turn.sequence, "native application event is malformed");
                (StatusCode::INTERNAL_SERVER_ERROR, "session_event_unreadable".to_string())
            }))
            .collect::<Result<Vec<_>, _>>()?;
        restored.insert(
            record.session_id,
            Session {
                parent_session_id: record.parent_session_id,
                worktree_id: record
                    .track_id
                    .as_deref()
                    .or_else(|| {
                        record
                            .metadata
                            .agent_id
                            .as_deref()
                            .and_then(|value| value.strip_prefix("context-agent:"))
                    })
                    .unwrap_or(&record.metadata.workspace_path)
                    .to_string(),
                created_at: record.started_at,
                events,
                usage: Usage::default(),
            },
        );
    }
    *state.sessions.lock().map_err(lock_error)? = restored;
    Ok(())
}

/// Persist the complete, ordered application timeline before its handler
/// reports success. SessionStorePlan is deliberately constructed from the
/// public native record/paths/events contract instead of any hook payload.
pub(super) fn persist_session(
    store: &SessionStoreConfig,
    session_id: &str,
    session: &Session,
) -> Result<(), (StatusCode, String)> {
    let now = chrono::Utc::now();
    let turns = session
        .events
        .iter()
        .map(|event| {
            let parent_event_id = event.sequence.checked_sub(1).and_then(|previous| {
                session
                    .events
                    .get(previous as usize)
                    .map(|parent| parent.event_id.clone())
            });
            Ok(SessionTurn {
                sequence: event.sequence as usize,
                role: match event.message.as_ref().map(|message| message.role.as_str()) {
                    Some("user") => SessionRole::User,
                    Some("assistant") => SessionRole::Assistant,
                    Some("tool") => SessionRole::Tool,
                    _ => SessionRole::System,
                },
                content: serde_json::to_string(event).map_err(|error| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("session_event_encode_failed: {error}"),
                    )
                })?,
                captured_at: now,
                tool_name: event
                    .message
                    .as_ref()
                    .and_then(|message| message.tool_calls.first())
                    .map(|tool| tool.tool_name.clone()),
                model: event.provider_model_id.clone(),
                event_meta: Some(SessionTurnEventMeta {
                    event_id: Some(event.event_id.clone()),
                    parent_event_id,
                    event_type: Some(format!("{:?}", event.evidence_kind)),
                    turn_id: event.run_id.clone(),
                    message_id: event.message_id.clone(),
                    tool_call_id: event.tool_call_id.clone(),
                    model_id: event.provider_model_id.clone(),
                    ..SessionTurnEventMeta::default()
                }),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let record = SessionRecord {
        schema_version: 1,
        session_id: session_id.to_string(),
        source: "context-agent".to_string(),
        started_at: session.created_at,
        captured_at: now,
        metadata: SessionMetadata {
            workspace_path: store.workspace_path.clone(),
            conversation_id: Some(session_id.to_string()),
            agent_id: Some("context-agent".to_string()),
            ticket_id: None,
            model: None,
            trigger: Some("http".to_string()),
            provisioning: None,
            producer: Some("context-agent".to_string()),
            copilot_version: None,
            vscode_version: None,
            protocol_version: Some(1),
            worktree: None,
        },
        turns,
        links: Default::default(),
        track_id: Some(session.worktree_id.clone()),
        anchor_ticket_id: None,
        parent_session_id: session.parent_session_id.clone(),
        spawned_session_id: None,
        emitted_handoff_ids: Vec::new(),
        picked_up_handoff_ids: Vec::new(),
    };
    let paths = store.paths_for(&record).map_err(store_error)?;
    SessionStorePlan {
        record,
        paths,
        events: None,
    }
    .persist()
    .map_err(store_error)?;
    Ok(())
}

pub(super) fn store_error(error: session_api::SessionError) -> (StatusCode, String) {
    tracing::error!(%error, "native session persistence failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "session_store_unavailable".to_string(),
    )
}
