//! Gemini 0.66.0 has no standard mode readback or config control. Keep native
//! permissions restricted: YOLO also rewrites tools and system instructions.
use agent_client_protocol::{
    PermissionOptionKind, RequestPermissionRequest, SessionConfigKind, SessionConfigOption,
    SessionModeState, ToolKind,
};
use std::collections::HashSet;

pub const MANAGED_MCP_METADATA: &str = "shiju.gemini/mcp-v4";

/// Call only for a verified Linux v4 process in its current, active prompt.
/// The adapter records the source in its session configuration and consumes the
/// reply at the MCP execution boundary. The caller must obtain explicit consent.
pub fn validate_managed_mcp_request(
    request: &RequestPermissionRequest,
    mounted_servers: &HashSet<String>,
) -> bool {
    let Some(metadata) = request.meta.as_ref().and_then(|meta| meta.get(MANAGED_MCP_METADATA))
    else {
        return false;
    };
    let (Some(server), Some(tool)) = (metadata["server"].as_str(), metadata["tool"].as_str())
    else {
        return false;
    };
    let source_matches = match metadata["origin"].as_str() {
        Some("mounted") => mounted_servers.contains(server),
        Some("native") => !mounted_servers.contains(server),
        _ => false,
    };
    request.tool_call.fields.kind == Some(ToolKind::Other)
        && !request.tool_call.tool_call_id.0.trim().is_empty()
        && !server.trim().is_empty()
        && !tool.trim().is_empty()
        && source_matches
        && request.tool_call.fields.raw_input.as_ref().is_some_and(serde_json::Value::is_object)
        && matches!(request.options.as_slice(), [allow, reject]
            if allow.kind == PermissionOptionKind::AllowOnce
            && reject.kind == PermissionOptionKind::RejectOnce
            && allow.option_id != reject.option_id)
}

pub fn restricted_mode(mode: &str) -> bool {
    matches!(mode, "default" | "plan")
}

pub fn restricted_session(
    modes: Option<&SessionModeState>,
    options: Option<&[SessionConfigOption]>,
) -> bool {
    modes.is_some_and(|state| restricted_mode(state.current_mode_id.0.as_ref()))
        && options.is_none_or(|options| {
            options.iter().all(|option| {
                if option.id.0.as_ref() != "mode" {
                    return true;
                }
                matches!(&option.kind, SessionConfigKind::Select(select)
                    if restricted_mode(select.current_value.0.as_ref()))
            })
        })
}
