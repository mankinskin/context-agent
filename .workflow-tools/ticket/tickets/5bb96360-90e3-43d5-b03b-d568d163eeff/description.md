## Goal

Make the agent a first-class driver of the shared chat UI ("UI sandbox") — the common interface between agent and user. Through the chat surface the agent controls a virtual world: rendering UI scenes, showing images, setting colors, shaping chat-message format, and driving interactive elements (question/answer widgets, arbitrary input requests, microphone toggle, targeted text output).

Deliver: (1) a typed agent->UI interaction protocol in `agent-shared`, and (2) a skill documenting which interactions the agent can perform. Anchor this into the agent-harness epic goal so the harness converges on a home-grown, OpenCode-style chat UI.

## Scope

Parent feature ticket. Tracks the child tickets:
- Interaction protocol/schema in `agent-shared`.
- Agent-core emission + response correlation.
- Dioxus WASM chat UI rendering.
- Ratatui TUI rendering / graceful degradation.
- UI-sandbox skill documenting the interactions.

## Acceptance criteria

All child tickets complete: typed protocol with serde round-trip, agent loop can emit every interaction and correlate responses within a session, both UI surfaces render (or explicitly degrade), and the skill enumerates every supported interaction. Browser verification + screenshots for Dioxus; terminal verification for TUI.

## Spec

Linked spec (tracked as follow-up until spec-api is reachable): agent-harness/interactive-chat-ui.