use agent_security::SecurityPolicy;
use serde_json::Value;
use agent_security::shell::permissions::{Permission, PermissionContext, PermissionMode};
use tool_context::PermissionDecision;
use tool_context::decision::mark_input_as_approved;
pub fn shell_security_decision(
    tool_name: &str,
    input: Value,
    security: &SecurityPolicy,
    command: &str,
    approved: bool,
) -> PermissionDecision {
    let context = PermissionContext {
        autonomy: security.autonomy,
        in_sandbox: false,
        mode: PermissionMode::Normal,
        approved,
        workspace_dir: security.workspace_dir.clone(),
        allowed_roots: security.allowed_roots.clone(),
    };

    let updated_input = if approved { mark_input_as_approved(tool_name, input) } else { input };

    match security.check_shell_permission(command, &context).permission {
        Some(Permission::Allow) | None => PermissionDecision::allow(updated_input),
        Some(Permission::Ask { reason, warning }) => {
            PermissionDecision::ask(reason, updated_input).with_warning(warning)
        }
        Some(Permission::Deny { reason }) => {
            PermissionDecision::deny(reason).with_updated_input(updated_input)
        }
    }
}
