use dioxus::prelude::*;

pub(super) fn focus_composer() {
    let _ = document::eval(
        "requestAnimationFrame(() => document.querySelector('input[aria-label=\"Message\"]')?.focus());",
    );
}

pub(super) fn install_focus_guard() {
    let _ = document::eval(
        r#"
        const guard = () => requestAnimationFrame(() => {
            const active = document.activeElement;
            if (active && active !== document.body &&
                (!active.getClientRects().length || getComputedStyle(active).visibility === 'hidden'))
                document.querySelector('input[aria-label="Message"]')?.focus();
        });
        window.contextAgentFocusGuard = guard;
        window.addEventListener('resize', guard);
        "#,
    );
}

pub(super) fn remove_focus_guard() {
    let _ = document::eval(
        "window.removeEventListener('resize', window.contextAgentFocusGuard); delete window.contextAgentFocusGuard;",
    );
}

pub(super) fn selected_session() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().hash().ok())
        .and_then(|hash| hash.strip_prefix("#session=").map(str::to_owned))
        .filter(|id| !id.is_empty())
}

pub(super) fn remember_session(id: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(&format!("session={id}"));
    }
}
