//! Qoder CLI uses `bypass_permissions`; its ACP config uses `mode=yolo`.
//! Protocol fields were observed in 1.1.83; tool execution remains untested.
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
    matches!(mode, "default" | "dontAsk" | "plan")
}
