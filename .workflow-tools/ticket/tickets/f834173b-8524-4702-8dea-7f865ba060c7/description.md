## Goal

Author a skill that documents the UI-sandbox interactions the agent can perform, so any agent loop can discover and correctly use the chat-UI interaction protocol.

## Requirements

- Enumerate every supported interaction (Q&A elements, input requests, microphone toggle, text output, image display, scene/color/message-format control) with its payload shape and response shape.
- State when to use each interaction and the expected user-response flow.
- Document terminal-degradation behavior (which interactions degrade in the Ratatui TUI and how).
- Keep the skill in sync with the agent-shared protocol version.

## Depends on

- agent-shared interaction protocol (UI1).
- agent-core emission path (UI2) for accurate usage guidance.

## Acceptance criteria

- Skill file exists under .agents/skills/ and enumerates every interaction with payload/response and usage guidance.
- Cross-referenced to the agent-shared protocol version and the spec agent-harness/interactive-chat-ui.