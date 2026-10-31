# Task 24 form fixture

Repository-authored, MIT licensed. `server.py` uses the frozen public
MCP 2025-06-18 `elicitation/create` form envelope over real stdio.
It records only synthetic user values in its tool result. No accounts,
hosted connectors, tokens or model-provided approvals are involved.

`http_server.py` runs a loopback Streamable HTTP server: a tool response
SSE stream carries the form request, and the user's answer returns in a
separate real HTTP POST. No public service or UI rendering is claimed here.

Reference: `codex-rs/codex-mcp/src/elicitation.rs`,
`ElicitationRequestRouter::request_user_interaction`: source/request authority,
pending responders and abandonment. User responses are the only test boundary
substitution; transport, process, connection and form routing are real.

The host integration API is `vw_agent::plugins::McpFormSession`. Register,
subscribe, reconcile `pending()` after receiver lag, and call `respond()`
under the same trusted tenant scope (`session::ui_store::with_tenant_scope`).
Keep the handler lease while that UI is available; dropping its last clone
cancels waiting forms. Registration and replies are host APIs, not model
tools. A live subscriber must exist before connection negotiation, and
adding/removing a handler renegotiates the source-bound connection.

The supported schema is the frozen object with primitive string, number,
integer, boolean and string enum fields, required fields, bounds, lengths,
enum labels, defaults as hints, and email/URI/date/date-time formats.
Unknown or null constraints, nested types, remote references and URL/auth
elicitation fail explicitly. Answers must contain only declared fields;
no default or empty confirmation is ever accepted without a user's reply.
Limits are 64 fields, 64 enum values, 16 pending forms, 64KiB request/answer
payloads and a 120-second form deadline, within the existing operation timeout.

Pending authority includes the tenant, workspace, session, configured server,
connection generation, exact remote request ID and opaque public request ID.
Remote metadata never changes it. Duplicate unacknowledged wire IDs are
rejected; reservations end at transport response acknowledgement. Invalid
user input keeps its responder for correction. Cancel, disconnect, operation
end, handler removal and expiry release pending state and reject late input.

Raw schemas are checked before the SDK can drop enum/unknown fields. User
answers bypass the SDK's typed debug/trace result model and are restored only
at the owned transport send boundary, with authority checked again. The trace
fixture suppresses server echo to isolate this client-generated answer path;
it does not claim to change arbitrary server tool-result logging. Raw logs
and run records remain local and ignored. Desktop controls/rendering are task25.
