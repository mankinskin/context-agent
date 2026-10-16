use super::*;

#[derive(Deserialize)]
struct EditorFile {
    path: String,
    content: String,
}

#[derive(Deserialize)]
struct EditorDiff {
    diff: String,
}

#[component]
pub(super) fn RootedEditor() -> Element {
    let mut path = use_signal(|| "notes/demo.txt".to_string());
    let mut content = use_signal(String::new);
    let diff = use_signal(String::new);
    let status = use_signal(|| None::<Result<String, String>>);
    let mut busy = use_signal(|| false);
    let mut open = move |_| {
        if busy() {
            return None;
        }
        busy.set(true);
        let requested = path();
        let mut content = content;
        let mut diff = diff;
        let mut status = status;
        Some(async move {
            let result = async {
                let encoded = js_sys::encode_uri_component(&requested)
                    .as_string()
                    .ok_or_else(|| "editor_path_encoding_failed".to_string())?;
                let file: EditorFile =
                    request_json("GET", &format!("/api/editor/file?path={encoded}"), None).await?;
                let changes: EditorDiff = request_json("GET", "/api/editor/diff", None).await?;
                Ok::<_, String>((file, changes))
            }
            .await;
            match result {
                Ok((file, changes)) => {
                    path.set(file.path);
                    content.set(file.content);
                    diff.set(changes.diff);
                    status.set(None);
                }
                Err(error) => status.set(Some(Err(error))),
            }
            busy.set(false);
        })
    };
    let mut save = move |_| {
        if busy() {
            return None;
        }
        busy.set(true);
        let requested = path();
        let draft = content();
        let mut diff = diff;
        let mut status = status;
        Some(async move {
            let result = async {
                let body = serde_json::json!({"path": requested, "content": draft}).to_string();
                let file: EditorFile = request_json("PUT", "/api/editor/file", Some(body)).await?;
                let changes: EditorDiff = request_json("GET", "/api/editor/diff", None).await?;
                Ok::<_, String>((file, changes))
            }
            .await;
            match result {
                Ok((file, changes)) => {
                    diff.set(changes.diff);
                    status.set(Some(Ok(format!("Saved {}", file.path))));
                }
                Err(error) => status.set(Some(Err(error))),
            }
            busy.set(false);
        })
    };
    rsx! {
        details { class: "rooted-editor",
            summary { "Rooted file editor" }
            label { "File path"
                input { aria_label: "File path", disabled: busy(), value: "{path}", oninput: move |event| path.set(event.value()) }
            }
            div { class: "editor-actions",
                button { onclick: move |event| { if let Some(task) = open(event) { spawn(task); } }, disabled: busy(), "Open file" }
                button { onclick: move |event| { if let Some(task) = save(event) { spawn(task); } }, disabled: busy(), "Save file" }
            }
            label { "File content"
                textarea { aria_label: "File content", disabled: busy(), value: "{content}", oninput: move |event| content.set(event.value()) }
            }
            if let Some(result) = status() {
                match result {
                    Ok(message) => rsx! { p { class: "editor-status", "{message}" } },
                    Err(error) => rsx! { p { role: "alert", class: "error-text", "{error}" } },
                }
            }
            pre { class: "editor-diff", role: "region", aria_label: "Git diff", "{diff}" }
        }
    }
}
