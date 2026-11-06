//! Kimi Code ACP modes from the official acp-server; CLI flags are a separate boundary.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};

pub(crate) const FULL_MODE: &str = "yolo";
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
    matches!(mode, RESTRICTED_MODE | "plan")
}

/// Question and plan answers share request_permission but are not tool grants.
pub(crate) fn has_workflow_answers(
    request: &agent_client_protocol::RequestPermissionRequest,
) -> bool {
    request.options.iter().any(|option| {
        matches!(
            option.kind,
            agent_client_protocol::PermissionOptionKind::AllowOnce
                | agent_client_protocol::PermissionOptionKind::AllowAlways
        ) && !matches!(
            option.option_id.0.as_ref(),
            "approve_once" | "approve_always" | "approve" | "approve_for_session"
        )
    })
}
