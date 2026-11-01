Synthetic fixtures authored for this repository under its license. No credentials.

Reference: codex-rs/hooks/src/events/pre_tool_use.rs and engine/output_parser.rs,
parse_pre_tool_use: updatedInput applies only with permissionDecision allow.
Tests generate trusted command packages in temporary directories. tool_name=marker,
tool_input={value:original}; allow rewrites value to rewritten and retains
additionalContext in the owning turn; deny leaves the marker absent. Non-object
updatedInput is rejected. Missing allow never applies an update.

Safety difference: malformed updates fail closed; deny is honored even without a
reason. Host validation and permissions run again after changes. This preserves
existing stricter host behavior rather than granting hook permission authority.
