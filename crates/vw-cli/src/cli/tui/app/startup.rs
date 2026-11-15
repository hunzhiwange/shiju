//! 运行启动与会话引导。

use super::{
    DirectorySwitchBootstrap,
    GatewayUiRuntime,
    Path,
    Result,
    TerminalMode,
    TuiState,
    V3GatewayApp,
    anyhow,
    load_integrations_blocking,
    load_orchestration_blocking,
    load_session_capabilities_blocking,
    next_stream_request_id,
    run_event_loop,
    run_terminal,
};

pub(crate) fn run_tui(runtime: GatewayUiRuntime, model_name: &str) -> Result<()> {
    runtime.ensure_local_gateway_ready_blocking().map_err(|error| anyhow!(error))?;

    let mut state = initial_tui_state(&runtime);
    let loaded = load_session_capabilities_blocking(&runtime);
    let selected_model = if loaded.matrix.models.iter().any(|model| model == model_name) {
        Some(model_name.to_owned())
    } else {
        loaded
            .default_model
            .clone()
            .or_else(|| loaded.matrix.models.first().cloned())
            .or_else(|| Some(model_name.to_owned()))
    };
    state.session_capabilities = loaded.matrix;
    state.settings_warnings = loaded.warnings;
    state.session_options.model.clone_from(&selected_model);
    state.status.model_name = selected_model;
    let integrations = load_integrations_blocking(&runtime);
    state.integration_catalog = integrations.items;
    state.integration_warnings = integrations.warnings;
    state.orchestration = load_orchestration_blocking(&runtime);
    let app = V3GatewayApp::new_with_terminal_mode(runtime, state, TerminalMode::Alternate);
    let terminal_mode = app.terminal_mode();

    run_terminal(terminal_mode, || run_event_loop(app)).map_err(Into::into)
}

pub(crate) fn initial_tui_state(runtime: &GatewayUiRuntime) -> TuiState {
    let mut state = TuiState::default();
    state.session.scope = runtime.scope().map(ToOwned::to_owned);
    state.project.workspace_root = Some(runtime.directory().to_path_buf());
    state
}

/// 恢复显式指定的会话；普通启动保留空白草稿。
pub(crate) fn restore_or_create_session_state(
    runtime: &mut GatewayUiRuntime,
) -> Result<TuiState, String> {
    let Some(session_id) = runtime.session_id() else {
        return Ok(initial_tui_state(runtime));
    };
    let scope = runtime.scope().map(ToOwned::to_owned);
    let snapshot = runtime.session_ui_resume_blocking(session_id)?;

    let mut state = TuiState::from_chat_session(&snapshot);
    state.session.scope = scope;
    state.project.workspace_root = Some(runtime.directory().to_path_buf());
    let title = (!state.session.title.trim().is_empty()).then(|| state.session.title.clone());
    runtime.bind_session_seed(state.session.session_id.clone(), state.session.scope.clone(), title);
    Ok(state)
}

/// 在临时 runtime 上解析并完整启动目标目录；调用方只提交成功结果。
pub(crate) fn bootstrap_directory_switch(
    mut runtime: GatewayUiRuntime,
    requested_path: &Path,
) -> Result<DirectorySwitchBootstrap, String> {
    let requested_path = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        runtime.directory().join(requested_path)
    };
    let project = runtime.resolve_project_directory_blocking(requested_path.as_path(), false)?;
    runtime.rebind_validated_directory(Path::new(project.directory.as_str()))?;

    let mut state = initial_tui_state(&runtime);
    let capabilities = load_session_capabilities_blocking(&runtime);
    if !capabilities.warnings.is_empty() {
        return Err(format!(
            "target capability bootstrap failed: {}",
            capabilities.warnings.join("; ")
        ));
    }
    state.status.model_name =
        capabilities.default_model.clone().or_else(|| capabilities.matrix.models.first().cloned());
    state.session_options.model.clone_from(&state.status.model_name);
    state.session_capabilities = capabilities.matrix;
    state.settings_warnings = capabilities.warnings;

    let integrations = load_integrations_blocking(&runtime);
    if !integrations.warnings.is_empty() {
        return Err(format!(
            "target integration bootstrap failed: {}",
            integrations.warnings.join("; ")
        ));
    }
    state.integration_catalog = integrations.items;
    state.integration_warnings = integrations.warnings;

    let orchestration = load_orchestration_blocking(&runtime);
    if !orchestration.warnings.is_empty() {
        return Err(format!(
            "target orchestration bootstrap failed: {}",
            orchestration.warnings.join("; ")
        ));
    }
    state.orchestration = orchestration;
    Ok(DirectorySwitchBootstrap::new(runtime, state))
}

pub(super) fn new_draft_id() -> String {
    use vw_shared::id::{Prefix, descending};

    descending(Prefix::Session, None)
        .unwrap_or_else(|_| format!("ses_{}", next_stream_request_id()))
}
