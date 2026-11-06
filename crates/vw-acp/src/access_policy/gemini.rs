//! Gemini 0.66.0 has no standard mode readback or config control. Keep native
//! permissions restricted: YOLO also rewrites tools and system instructions.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption, SessionModeState};

pub(crate) fn restricted_mode(mode: &str) -> bool {
    matches!(mode, "default" | "plan")
}

pub(crate) fn restricted_session(
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
