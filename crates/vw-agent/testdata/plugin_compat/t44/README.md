# Stop / Interrupt fixture

Synthetic shell commands and payloads in `t44_tests.rs` are repository-authored,
under the repository license. They use temporary package and state directories,
real trusted command handlers, and no service credentials.

Frozen reference: codex-rs/hooks/src/events/stop.rs (`StopRequest`, `parse_completed`,
`aggregate_results`), interrupt.rs (`InterruptRequest`, `parse_completed`), and
core/src/session/turn.rs (the turn-local `stop_hook_active` flag).

Stop input carries session_id, turn_id, cwd, model, permission_mode,
transcript_path, stop_hook_active and last_assistant_message. Interrupt excludes
the final two Stop-only fields. `decision:block` plus a nonempty reason or exit 2
plus nonempty stderr continues a completed turn. `continue:false` wins over block.
Interrupt is observational. Failure is neither successful completion nor user
interruption; SessionEnd remains the separate close boundary.

Host safety difference: Interrupt command deadlines are bounded to 1–3 seconds,
and cancellation drops an in-flight Stop process through the existing process
termination guard. Raw failed command output is not logged.
