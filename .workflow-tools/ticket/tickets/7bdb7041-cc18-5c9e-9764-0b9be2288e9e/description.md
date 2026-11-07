## Motivation
Explicit feedback was recorded against `ce://meta-workspace/file/context-agent-readme` during session `29ad8d96-9400-4627-a032-e02a3d8bd971` (tool call `call_dxrr1ay0ZsCtExyRqvlV6mKN`).

## Feedback
- Rating: `not-helpful`
- Note: Context-agent README still calls the app an unmodified /api/echo starter. Actual backend/lib.rs has session/timeline/message/evidence/credits/weather/MCP lifecycle routes; frontend is Weather workspace. browser smoke script comments retain echo/auth assumptions; smoke-container.sh must be checked against new APIs. MCP lifecycle returns accepted/pending provider integration and reconnect aliases in-memory timeline, while related CA-03/CA-07 tickets are done and acceptance part read-backs empty. Planning must use current code as baseline and not claim live provider or durability proof. Deferred fixes scoped into testing/guidance work, not product rewrite.

## Backtrace
- Session: `29ad8d96-9400-4627-a032-e02a3d8bd971`
- Tool call: `call_dxrr1ay0ZsCtExyRqvlV6mKN`
- Event id: `5b1e141d-d9d6-47f3-858b-1aef3ef87557`
- Dedupe key: `feedback-followup/29ad8d96-9400-4627-a032-e02a3d8bd971/call_dxrr1ay0ZsCtExyRqvlV6mKN`
- FeedbackEntry: the live `feedback_ingest` call already persisted its own entry for `ce://meta-workspace/file/context-agent-readme`; cross-reference it via `feedback_inbox`/`entries_for(target)` filtered to this session and tool call (today's `feedback_ingest` transport does not yet echo back the created entry's id for direct linking here).

## Verification
Record a validation execution (test-api) confirming the reported issue is addressed before moving this ticket past `in-review`.
