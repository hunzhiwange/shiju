//! iFlow 0.5.19 ACP modes; load omits modes and set_mode only acknowledges.
pub const FULL_MODE: &str = "yolo";
pub const RESTRICTED_MODE: &str = "default";

pub fn restricted_mode(mode: &str) -> bool {
    matches!(mode, RESTRICTED_MODE | "plan")
}

pub const LINUX_TOOL_APPROVAL: &str = "shiju.iflow/tool-v3";

/// Validates the request shape from the reviewed Linux overlay. The caller must
/// bind it to the current prompt and require a fresh host decision.
pub fn valid_linux_tool_approval(
    request: &agent_client_protocol::RequestPermissionRequest,
) -> bool {
    use agent_client_protocol::PermissionOptionKind::{AllowOnce, RejectOnce};

    request
        .meta
        .as_ref()
        .and_then(|meta| meta.get(LINUX_TOOL_APPROVAL))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|tool| !tool.trim().is_empty())
        && !request.tool_call.tool_call_id.0.trim().is_empty()
        && request
            .tool_call
            .fields
            .raw_input
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .is_some_and(|args| !args.contains_key("callId"))
        && matches!(request.options.as_slice(), [allow, reject]
            if allow.kind == AllowOnce && reject.kind == RejectOnce
                && allow.option_id != reject.option_id)
}
