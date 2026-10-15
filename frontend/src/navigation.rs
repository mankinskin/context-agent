use super::*;

pub(super) async fn refresh(
    mut sessions: Signal<Vec<SessionSummary>>,
    mut active: Signal<Option<String>>,
    mut timeline: Signal<Option<Result<TimelineResponse, String>>>,
) {
    match list_sessions().await {
        Ok(list) => {
            let selected = active()
                .filter(|id| list.iter().any(|entry| &entry.session_id == id))
                .or_else(|| list.first().map(|entry| entry.session_id.clone()));
            sessions.set(list);
            active.set(selected.clone());
            if let Some(id) = selected {
                shell::remember_session(&id);
                let loaded = load_timeline(&id).await;
                if active().as_deref() == Some(id.as_str()) {
                    timeline.set(Some(loaded));
                }
            } else {
                timeline.set(None);
            }
        }
        Err(error) => timeline.set(Some(Err(error))),
    }
}

pub(super) fn tree(sessions: &[SessionSummary]) -> Vec<(SessionSummary, usize)> {
    fn children(
        sessions: &[SessionSummary],
        parent: Option<&str>,
        depth: usize,
        output: &mut Vec<(SessionSummary, usize)>,
    ) {
        for session in sessions
            .iter()
            .filter(|session| session.parent_session_id.as_deref() == parent)
        {
            output.push((session.clone(), depth));
            children(sessions, Some(&session.session_id), depth + 1, output);
        }
    }
    let mut output = Vec::with_capacity(sessions.len());
    children(sessions, None, 0, &mut output);
    output
}
