//! Vibe's ACP mode selects an entire agent profile, not just approval policy.
//! Builtin names can be overridden locally; never switch profiles implicitly.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption, SessionModeState};

pub(crate) const FULL_MODE: &str = "auto-approve";

pub(crate) fn has_auto_approve(
    modes: Option<&SessionModeState>,
    options: Option<&[SessionConfigOption]>,
) -> bool {
    modes.is_some_and(|modes| modes.current_mode_id.0.as_ref() == FULL_MODE)
        || options.is_some_and(|options| {
            options.iter().any(|option| {
                option.id.0.as_ref() == "mode"
                    && matches!(&option.kind, SessionConfigKind::Select(select)
                        if select.current_value.0.as_ref() == FULL_MODE)
            })
        })
}
