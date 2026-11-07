## Summary

Extend the agent harness so the agent can **actively drive the shared chat UI** ("UI sandbox") — the common interface between agent and user. The agent controls a virtual world through the chat surface: rendering UI scenes, displaying images, setting colors, and shaping the format of chat messages, plus interactive elements such as question/answer widgets, arbitrary input requests, microphone toggling, and targeted text output.

The deliverable is twofold:
1. A typed **agent -> UI interaction protocol** (in `agent-shared`) enumerating every interaction the agent may perform in the UI sandbox.
2. A **skill** that documents which interactions the agent can invoke and how, so any agent loop can reason about and use the UI sandbox.

This objective is anchored into the agent-harness epic goal so the harness converges on a home-grown, OpenCode-style chat UI.

## Motivation

Today the chat UI is a passive transcript. To build our own agent-facing chat experience the agent must be a first-class driver of the interface: it should be able to request structured input, open interactive controls, control presentation (scene/colors/message format), and manage device affordances (microphone). A stable, typed protocol plus a discoverable skill turns "the chat UI" into a controllable virtual world shared with the user.

## Requirements

- Define a typed, `serde`-tagged interaction protocol in `agent-shared` covering at minimum:
  - Question/answer UI elements (structured prompts with selectable/typed responses).
  - Generic input requests (text, choice, and other input kinds).
  - Microphone control (enable/disable capture).
  - Targeted text output / message emission.
  - Image display.
  - Presentation control: UI scene selection, colors, and chat-message format.
- The protocol travels over the existing `agent-server` broadcast/websocket channel and is shared by all frontends.
- Both chat UI surfaces render/handle the interactions: the **Dioxus WASM** browser client and the **Ratatui TUI** (with graceful degradation where a terminal cannot render an interaction, e.g. images).
- The `agent-core` loop can emit these interaction directives as part of its normal turn, and receive user responses back through the same session.
- A skill documents the available interactions, their payloads, when to use them, and terminal-degradation behavior.

## Non-goals

- Full multimodal media pipeline beyond displaying images the agent references.
- Voice transcription/ASR implementation itself (only the microphone enable/disable affordance and its event signaling).
- Replacing the existing message/event schema; this extends it.

## Acceptance criteria

- `agent-shared` exposes a versioned interaction protocol enum with round-trip `serde` coverage.
- Agent loop in `agent-core` can emit each interaction kind and correlate user responses within one session.
- Dioxus WASM chat UI renders every interaction kind; Ratatui TUI renders each or degrades explicitly and observably.
- The UI-sandbox skill exists and enumerates every supported interaction with payload shape and usage guidance.
- Browser verification with screenshots for the Dioxus surface; TUI verification captured for the terminal surface.

## Traceability

- Feature ticket: 5bb96360 [agent-harness] Agent-driven interactive chat UI (UI sandbox interaction protocol + skill)
- Child tickets: b947e0d3 (UI1 protocol), 3dd84de0 (UI2 agent-core), 11db5933 (UI3 Dioxus), cdf1e535 (UI4 Ratatui), f834173b (UI5 skill)
- Parent epic: 0f4b3c5b [agent-harness] Unified minimal interface for on-demand chat + long-running agent loops
