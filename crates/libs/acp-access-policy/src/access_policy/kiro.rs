//! V3 session presets are in-memory grants, not sandbox or enterprise overrides.
use acp_types::PermissionMode;

pub fn session_policy(
    full_access: bool,
    permission_mode: PermissionMode,
) -> serde_json::Value {
    let presets: &[&str] = if full_access && permission_mode == PermissionMode::ApproveAll {
        &["allow-all"]
    } else {
        &[]
    };
    serde_json::json!({"policyPreset": presets})
}
