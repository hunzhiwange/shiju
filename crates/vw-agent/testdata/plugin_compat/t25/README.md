# MCP desktop forms fixture

Repository authored, MIT licensed. `form.json` uses the task24 frozen primitive
schema. It exercises Chinese labels/descriptions, long content, string, integer,
boolean, raw enum values with separate labels, optional fields and default hints.
The native window harness uses the production App/update/modal, host watcher,
Gateway form routes and a real stdio Python MCP server (task24 fixture).
Only model/provider startup and user input are driven by the explicit harness;
MCP process, routing, validation, HTTP requests and native Iced drawing are real.
Synthetic values only; no tokens or real user content.

Reference snapshots: Codex bottom_pane MCP boolean/text forms, explicit user
input, required unanswered fields, submit and Escape cancellation. Source and
snapshot hashes are recorded in t25.json. Cross-chat sharing and model schema
compression remain separately recorded coverage gaps.

## Explicit native-window verification

While holding a shared Cargo slot, export its absolute `CARGO_TARGET_DIR` and
`CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"`. Build the filtered agent host
lease test and the `mcp-form-window` target, then run `run_window.py <new-name>`.
It selects only executables from that owned build directory, starts the real
loopback Gateway fixture, and tears down its child even if the window fails.
The native daemon uses production `App::update`/`App::view`, the production host
watcher and actual Iced screenshots. User actions are explicit synthetic
messages at the allowed user-input boundary. No model fills the form.

Six scenarios cover accept/decline/Escape cancel in both themes, with top,
filled controls, bottom and finished screenshots. Live values and enum wire
values are checked against the real MCP result. Defaults stay hints and
cancellation returns no answers. Recent project lists are cleared only in this
explicit harness so screenshots contain no personal workspace content.
Selected sanitized screenshots and wire results are archived under
`docs/plugin-compat/evidence/t25/rendered`; raw run files/logs and intermediate
screenshots remain local. This does not claim Linux/Windows rendering or public
remote service acceptance.
