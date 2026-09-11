use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response as WebResponse};

fn main() {
    dioxus::launch(App);
}

#[derive(Serialize)]
struct EchoRequest {
    message: String,
}

#[derive(Deserialize)]
struct EchoResponse {
    message: String,
}

async fn call_echo(message: String) -> Result<String, String> {
    let body = serde_json::to_string(&EchoRequest { message }).map_err(|e| e.to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::SameOrigin);
    opts.set_body(&JsValue::from_str(&body));

    let request =
        Request::new_with_str_and_init("/api/echo", &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or("no window")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: WebResponse = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("request failed: {}", resp.status()));
    }

    let json_value = JsFuture::from(resp.json().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text = js_sys::JSON::stringify(&json_value)
        .map(|s| s.as_string().unwrap_or_default())
        .map_err(|e| format!("{e:?}"))?;
    let parsed: EchoResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(parsed.message)
}

#[component]
fn App() -> Element {
    let mut input = use_signal(String::new);
    let mut result = use_signal(|| None::<Result<String, String>>);

    rsx! {
        h1 { "template" }
        p { "A minimal Dioxus + Axum starter. Edit " code { "frontend/src/main.rs" } " and " code { "backend/src/lib.rs" } " to build your app." }
        input {
            value: "{input}",
            oninput: move |evt| input.set(evt.value()),
        }
        button {
            onclick: move |_| {
                let message = input();
                async move {
                    result.set(Some(call_echo(message).await));
                }
            },
            "Send to /api/echo"
        }
        match result() {
            Some(Ok(message)) => rsx! { p { "Echoed: {message}" } },
            Some(Err(err)) => rsx! { p { "Error: {err}" } },
            None => rsx! {},
        }
    }
}
