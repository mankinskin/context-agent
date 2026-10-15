use super::*;

#[component]
pub(super) fn PreambleEditor() -> Element {
    let mut blocks = use_signal(Vec::<PreambleBlock>::new);
    let mut loaded = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut text = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    use_future(move || async move {
        match request_json("GET", "/api/preambles", None).await {
            Ok(value) => {
                blocks.set(value);
                loaded.set(true);
            }
            Err(value) => error.set(Some(value)),
        }
    });
    let save = move |value: Vec<PreambleBlock>| async move {
        busy.set(true);
        let result = match serde_json::to_string(&value) {
            Ok(body) => request_json("PUT", "/api/preambles", Some(body)).await,
            Err(value) => Err(value.to_string()),
        };
        match result {
            Ok(value) => {
                blocks.set(value);
                error.set(None);
            }
            Err(value) => error.set(Some(value)),
        }
        busy.set(false);
    };
    rsx! {
        details { class: "preamble-editor",
            summary { "Global preambles" }
            p { "Enabled blocks apply to every new model request. Tool grants are unchanged." }
            for block in blocks().into_iter() {
                {
                    let id = block.id.clone();
                    let toggle_id = id.clone();
                    rsx! {
                        label { class: "preamble-row", key: "{id}",
                            input { r#type: "checkbox", aria_label: "Enable {id}", checked: block.enabled,
                                disabled: busy(), onchange: move |_| {
                                    let mut next = blocks();
                                    if let Some(block) = next.iter_mut().find(|block| block.id == toggle_id) { block.enabled = !block.enabled; }
                                    save(next)
                                }
                            }
                            span { "{block.id}" }
                        }
                    }
                }
            }
            label { "Preamble id"
                input { aria_label: "Preamble id", value: "{name}", oninput: move |event| name.set(event.value()) }
            }
            label { "Preamble text"
                textarea { aria_label: "Preamble text", value: "{text}", oninput: move |event| text.set(event.value()) }
            }
            button { class: "secondary-button", disabled: busy() || !loaded(), onclick: move |_| {
                let mut next = blocks();
                next.push(PreambleBlock { id: name(), text: text(), enabled: false });
                async move { save(next).await; }
            }, "Add preamble" }
            if let Some(value) = error() { p { role: "alert", "{value}" } }
            if loaded() && !busy() { p { role: "status", "Preambles saved" } }
        }
    }
}
