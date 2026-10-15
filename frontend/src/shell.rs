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
