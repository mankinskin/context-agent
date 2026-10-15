use super::*;

#[derive(Clone, Copy)]
pub(super) struct ChoiceContext {
    pub session: Signal<Option<String>>,
    pub timeline: Signal<Option<Result<TimelineResponse, String>>>,
}

#[component]
pub(super) fn ChoiceStart() -> Element {
    let mut context = use_context::<ChoiceContext>();
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        button { class: "secondary-button", disabled: busy(), onclick: move |_| async move {
            busy.set(true);
            let id = match (context.session)() {
                Some(id) => Ok(id),
                None => create_session(None).await,
            };
            match id {
                Ok(id) => {
                    shell::remember_session(&id);
                    context.session.set(Some(id.clone()));
                    match send_choice(&id, None).await {
                        Ok(_) => { context.timeline.set(Some(load_timeline(&id).await)); error.set(None); }
                        Err(value) => error.set(Some(value)),
                    }
                }
                Err(value) => error.set(Some(value)),
            }
            busy.set(false);
        }, "Start lookup choice" }
        if let Some(value) = error() { p { role: "alert", class: "error-text", "{value}" } }
    }
}

#[component]
pub(super) fn ChoiceWidget(widget_id: String, options: Vec<String>) -> Element {
    let mut context = use_context::<ChoiceContext>();
    let mut selected = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let stored = context
        .timeline
        .read()
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .and_then(|timeline| {
            timeline
                .events
                .iter()
                .filter_map(|event| event.message.as_ref())
                .flat_map(|message| &message.blocks)
                .find_map(|block| match block {
                    RichTextBlock::ChoiceAnswer {
                        widget_id: id,
                        choice,
                    } if id == &widget_id => Some(choice.clone()),
                    _ => None,
                })
        });
    let submitted = stored.is_some();
    let checked = stored.clone().or(selected());
    let submit_id = widget_id.clone();
    rsx! {
        fieldset { class: "choice-widget", "data-widget-id": "{widget_id}", disabled: busy() || submitted,
            legend { "Choose a lookup" }
            for option in options {
                {
                    let choice = option.clone();
                    rsx! {
                        label { class: "choice-option", key: "{option}",
                            input { r#type: "radio", name: "{widget_id}", value: "{option}",
                                checked: checked.as_ref() == Some(&option),
                                onchange: move |_| selected.set(Some(choice.clone()))
                            }
                            "{option}"
                        }
                    }
                }
            }
            button { class: "secondary-button", disabled: selected().is_none(), onclick: move |_| {
                let widget_id = submit_id.clone();
                async move {
                    let (Some(id), Some(choice)) = ((context.session)(), selected()) else {
                        error.set(Some("Select a session and lookup option before submitting.".into()));
                        return;
                    };
                    busy.set(true);
                    let result = send_choice(&id, Some(ChoiceAnswer { widget_id, choice })).await;
                    context.timeline.set(Some(load_timeline(&id).await));
                    error.set(result.err());
                    busy.set(false);
                }
            }, "Submit choice" }
        }
        if let Some(choice) = stored { p { role: "status", "Submitted: {choice}" } }
        if let Some(value) = error() { p { role: "alert", class: "error-text", "{value}" } }
    }
}
