# UserPromptSubmit fixture

Source: frozen Codex `codex-rs/hooks/src/events/user_prompt_submit.rs`
(`UserPromptSubmitRequest`, `parse_completed`) and `engine/output_parser.rs`
(`parse_user_prompt_submit`), Apache-2.0. Synthetic shell commands in
`compat/t40_tests.rs` are project-authored fixtures under the repository license.

Input: host session_id, turn_id, absolute cwd, model, null transcript_path,
permission_mode and only the current prompt. No restored history is submitted.
Expected: decision:block with reason stops before provider; additionalContext
is consumed once by the owning turn; matcher is ignored for this event.
Command markers and host data live outside the package; package remains empty.
No real user content, credentials, remote service or UI is needed.

Safety differences: malformed output, missing blocking reason and exit 2 without
feedback fail closed, consistent with existing host hook safety. Codex may mark
these handler failures without stopping the turn. This evidence does not claim
identical error recovery or full plugin compatibility.
