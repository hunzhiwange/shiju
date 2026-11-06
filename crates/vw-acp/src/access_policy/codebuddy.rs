//! CodeBuddy 2.179.3 publishes `mode=default/bypassPermissions` in ACP.
//! Keep `fullAccess` (which also skips dangerous-command checks) out of automatic grants.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};

pub(crate) const FULL_MODE: &str = "bypassPermissions";
pub(crate) const RESTRICTED_MODE: &str = "default";

pub(crate) fn current_mode(options: Option<&[SessionConfigOption]>) -> Option<&str> {
    options?.iter().find_map(|option| match &option.kind {
        SessionConfigKind::Select(select) if option.id.0.as_ref() == "mode" => {
            Some(select.current_value.0.as_ref())
        }
        _ => None,
    })
}

pub(crate) fn advertises_mode(options: Option<&[SessionConfigOption]>, value: &str) -> bool {
    options.is_some_and(|options| {
        options.iter().any(|option| {
            if option.id.0.as_ref() != "mode" {
                return false;
            }
            match &option.kind {
                SessionConfigKind::Select(select) => match &select.options {
                    agent_client_protocol::SessionConfigSelectOptions::Ungrouped(options) => {
                        options.iter().any(|option| option.value.0.as_ref() == value)
                    }
                    _ => false,
                },
                _ => false,
            }
        })
    })
}

pub(crate) fn restricted_mode(mode: &str) -> bool {
    matches!(mode, "default" | "dontAsk" | "plan")
}

/// CodeBuddy publishes question/plan tool identity in vendor metadata, not ToolKind.
pub(crate) fn is_workflow_request(
    request: &agent_client_protocol::RequestPermissionRequest,
) -> bool {
    request.tool_call.fields.kind == Some(agent_client_protocol::ToolKind::SwitchMode)
        || request
            .tool_call
            .meta
            .as_ref()
            .and_then(|meta| meta.get("codebuddy.ai/toolName"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|name| matches!(name, "AskUserQuestion" | "ExitPlanMode"))
        || request
            .options
            .iter()
            .any(|option| option.option_id.0.as_ref() == "reject_and_exit_plan")
}
