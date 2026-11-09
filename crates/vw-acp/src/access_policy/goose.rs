//! Goose ACP server v1.57.0 exposes session-local tool approval modes.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};

pub(crate) const RESTRICTED_MODE: &str = "approve";

pub(crate) fn restricted_mode(mode: &str) -> bool {
    matches!(mode, RESTRICTED_MODE | "chat")
}

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

/// Goose refreshes available model choices when it changes mode.
pub(crate) fn same_selected_settings(
    previous: &[SessionConfigOption],
    current: &[SessionConfigOption],
) -> bool {
    let mut previous = previous.iter().filter(|option| option.id.0.as_ref() != "mode");
    let current = current.iter().filter(|option| option.id.0.as_ref() != "mode");
    previous.clone().count() == current.clone().count()
        && previous.all(|before| {
            current.clone().any(|after| {
                before.id == after.id
                    && match (&before.kind, &after.kind) {
                        (SessionConfigKind::Select(before), SessionConfigKind::Select(after)) => {
                            before.current_value == after.current_value
                        }
                        (before, after) => before == after,
                    }
            })
        })
}

#[cfg(test)]
#[path = "goose_tests.rs"]
mod tests;
