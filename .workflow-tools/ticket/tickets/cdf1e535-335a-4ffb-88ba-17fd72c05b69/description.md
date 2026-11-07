## Goal

Handle every agent-driven interaction kind in the Ratatui TUI chat surface, with explicit, observable graceful degradation where the terminal cannot render an interaction (e.g. images, rich scene/colors).

## Requirements

- Render Q&A widgets, input requests, microphone toggle, agent text output, and message-format hints in the terminal.
- For interactions a terminal cannot fully render (images, rich presentation), degrade explicitly and observably (e.g. placeholder + reference) rather than silently dropping.
- Send user responses back over the websocket using the response envelope; preserve 60 FPS input-thread responsiveness.

## Depends on

- agent-shared interaction protocol (UI1).

## Acceptance criteria

- Each interaction kind is rendered or explicitly degraded in the Ratatui TUI.
- Responses round-trip to agent-core within a session.
- Terminal verification captured for the interaction surface.