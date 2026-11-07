## Goal

Render every agent-driven interaction kind in the Dioxus WASM browser chat UI (the OpenCode-style target surface), and send user responses back over the websocket.

## Requirements

- Render Q&A widgets, input requests (text/choice/etc.), microphone toggle control, agent text output, image display, and presentation control (UI scene, colors, chat-message format).
- Wire user interactions to the response envelope and emit them over the existing agent-server websocket.
- Gate rendering on the protocol version marker.

## Depends on

- agent-shared interaction protocol (UI1).

## Acceptance criteria

- Each interaction kind renders and is interactive in the Dioxus WASM client.
- Responses round-trip to agent-core within a session.
- Playwright e2e coverage for the interaction surface; browser verification in an external fullscreen Chromium-family browser with screenshots (record window/display resolution).