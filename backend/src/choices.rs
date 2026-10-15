use super::*;

pub(super) fn validate_answer(
    session: &Session,
    answer: &ChoiceAnswer,
) -> Result<(), (StatusCode, String)> {
    let blocks = session
        .events
        .iter()
        .filter_map(|event| event.message.as_ref())
        .flat_map(|message| &message.blocks)
        .collect::<Vec<_>>();
    let exists = blocks.iter().any(|block| {
        matches!(block,
        RichTextBlock::MultipleChoice { widget_id, .. } if widget_id == &answer.widget_id)
    });
    if !exists || !matches!(answer.choice.as_str(), "lookup-a" | "lookup-b") {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid_choice: choose lookup-a or lookup-b from an existing widget".into(),
        ));
    }
    if blocks.iter().any(|block| {
        matches!(block,
        RichTextBlock::ChoiceAnswer { widget_id, .. } if widget_id == &answer.widget_id)
    }) {
        return Err((
            StatusCode::CONFLICT,
            "choice_already_submitted: the stored answer cannot be submitted twice".into(),
        ));
    }
    Ok(())
}
