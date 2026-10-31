//! 会话模式与模型偏好的读取和写回逻辑。

use std::collections::BTreeMap;

use agent_client_protocol::{
    SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOptions, SessionModelState,
};

use crate::{SessionAcpxState, SessionRecord, SessionStateOptions};

#[cfg(test)]
#[path = "session_mode_preference_tests.rs"]
mod session_mode_preference_tests;

fn empty_session_vwacp_state() -> SessionAcpxState {
    SessionAcpxState {
        current_mode_id: None,
        desired_mode_id: None,
        current_model_id: None,
        available_models: None,
        available_commands: None,
        config_options: None,
        desired_config_options: None,
        session_options: None,
    }
}

fn empty_session_state_options() -> SessionStateOptions {
    SessionStateOptions { model: None, allowed_tools: None, max_turns: None }
}

fn ensure_vwacp_state(record: &mut SessionRecord) -> &mut SessionAcpxState {
    record.vwacp.get_or_insert_with(empty_session_vwacp_state)
}

pub fn normalize_mode_id(mode_id: Option<&str>) -> Option<String> {
    let trimmed = mode_id?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn normalize_model_id(model_id: Option<&str>) -> Option<String> {
    let trimmed = model_id?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub fn get_desired_mode_id(state: Option<&SessionAcpxState>) -> Option<String> {
    normalize_mode_id(state.and_then(|state| state.desired_mode_id.as_deref()))
}

pub fn set_desired_mode_id(record: &mut SessionRecord, mode_id: Option<&str>) {
    let normalized = normalize_mode_id(mode_id);
    let vwacp = ensure_vwacp_state(record);

    vwacp.desired_mode_id = normalized;
}

pub fn get_desired_model_id(state: Option<&SessionAcpxState>) -> Option<String> {
    normalize_model_id(
        state
            .and_then(|state| state.session_options.as_ref())
            .and_then(|options| options.model.as_deref()),
    )
}

pub fn set_desired_model_id(record: &mut SessionRecord, model_id: Option<&str>) {
    let normalized = normalize_model_id(model_id);
    let vwacp = ensure_vwacp_state(record);
    let mut session_options =
        vwacp.session_options.clone().unwrap_or_else(empty_session_state_options);

    session_options.model = normalized;

    if session_options.model.is_some()
        || session_options.allowed_tools.is_some()
        || session_options.max_turns.is_some()
    {
        vwacp.session_options = Some(session_options);
    } else {
        vwacp.session_options = None;
    }
}

pub fn set_current_model_id(record: &mut SessionRecord, model_id: Option<&str>) {
    let normalized = normalize_model_id(model_id);
    let vwacp = ensure_vwacp_state(record);

    vwacp.current_model_id = normalized.clone();
    if let Some(model_id) = normalized {
        for option in vwacp.config_options.iter_mut().flatten() {
            if (option.category == Some(SessionConfigOptionCategory::Model)
                || option.id.0.as_ref() == "model")
                && let SessionConfigKind::Select(select) = &mut option.kind
            {
                select.current_value = model_id.clone().into();
            }
        }
    }
}

pub fn sync_advertised_model_state(record: &mut SessionRecord, models: Option<&SessionModelState>) {
    let Some(models) = models else {
        return;
    };

    let vwacp = ensure_vwacp_state(record);
    vwacp.current_model_id = Some(models.current_model_id.to_string());
    vwacp.available_models =
        Some(models.available_models.iter().map(|model| model.model_id.to_string()).collect());
}

pub fn get_desired_config_options(state: Option<&SessionAcpxState>) -> BTreeMap<String, String> {
    state.and_then(|state| state.desired_config_options.clone()).unwrap_or_default()
}

pub fn set_desired_config_option(record: &mut SessionRecord, config_id: &str, value: &str) {
    let config_id = config_id.trim();
    let value = value.trim();
    if config_id.is_empty() || value.is_empty() {
        return;
    }
    ensure_vwacp_state(record)
        .desired_config_options
        .get_or_insert_with(BTreeMap::new)
        .insert(config_id.to_string(), value.to_string());
}

pub fn sync_config_options(record: &mut SessionRecord, options: Option<Vec<SessionConfigOption>>) {
    if let Some(options) = options {
        ensure_vwacp_state(record).config_options = Some(options);
    }
}

pub fn resolve_model_config_value(
    options: Option<&[SessionConfigOption]>,
    value: &str,
) -> Result<Option<String>, String> {
    let Some(options) = options else {
        return Ok(None);
    };
    let option = options
        .iter()
        .find(|option| option.category == Some(SessionConfigOptionCategory::Model))
        .or_else(|| options.iter().find(|option| option.id.0.as_ref() == "model"));
    option.map(|option| validate_select_value(option, value)).transpose()
}

pub fn resolve_config_value(
    options: Option<&[SessionConfigOption]>,
    requested_id: &str,
    value: &str,
) -> Result<String, String> {
    let options = options.ok_or_else(|| {
        format!("Agent did not advertise session config options; cannot set {requested_id}")
    })?;
    let requested_id = requested_id.trim();
    let is_thought_level = matches!(requested_id, "reasoning_effort" | "thought_level" | "effort");
    let option = if is_thought_level {
        options
            .iter()
            .find(|option| option.category == Some(SessionConfigOptionCategory::ThoughtLevel))
            .or_else(|| {
                options.iter().find(|option| {
                    matches!(option.id.0.as_ref(), "reasoning_effort" | "thought_level" | "effort")
                })
            })
    } else {
        options.iter().find(|option| option.id.0.as_ref() == requested_id)
    }
    .ok_or_else(|| {
        let available =
            options.iter().map(|option| option.id.to_string()).collect::<Vec<_>>().join(", ");
        format!(
            "Agent did not advertise config option {requested_id}; available options: {available}"
        )
    })?;
    validate_select_value(option, value)
}

fn validate_select_value(option: &SessionConfigOption, value: &str) -> Result<String, String> {
    let SessionConfigKind::Select(select) = &option.kind else {
        return Err(format!("Config option {} does not accept named values", option.id));
    };
    let values = match &select.options {
        SessionConfigSelectOptions::Ungrouped(options) => {
            options.iter().map(|item| item.value.to_string()).collect::<Vec<_>>()
        }
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| group.options.iter().map(|item| item.value.to_string()))
            .collect(),
        _ => Vec::new(),
    };
    if values.iter().any(|candidate| candidate == value) {
        return Ok(option.id.to_string());
    }
    Err(format!(
        "Unsupported value {value} for config option {}; available values: {}",
        option.id,
        values.join(", ")
    ))
}
