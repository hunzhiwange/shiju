//! Host adapter contract: local configuration effects become desktop tasks.
use super::*;
#[test]
fn configuration_refresh_maps_to_desktop_tasks() {
    let mut app = App::new().0;
    let task = update(&mut app, SettingsMessage::Mcp(McpMessage::Refresh));
    assert!(app.mcp_settings.loading);
    assert!(app.mcp_settings.inspect_loading);
    assert!(task.units() > 0);
}

#[test]
fn managed_mcp_completion_preserves_foreign_acp_selection() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let mut app = App::new().0;
    let before = app.acp_settings.enabled.clone();
    let context = super::super::mcp_management_list::context(&app).unwrap();
    let mut state = super::super::mcp_management_import::ImportState::new(context.clone());
    state.importing = true;
    let generation = state.generation;
    app.mcp_settings.import = Some(state);
    let _ = super::super::mcp_management_import::update(
        &mut app,
        super::super::mcp_management_import::ImportMessage::Finished(
            context.clone(),
            generation,
            Ok(vec![]),
        ),
    );
    assert_eq!(app.acp_settings.enabled, before);
    let _ = super::super::mcp_management_bindings::update(
        &mut app,
        super::super::mcp_management_bindings::BindingsMessage::Finished {
            context: context.clone(),
            generation: 0,
            result: Ok(vw_api_mcp_management::McpDistributionResult {
                outcome: vw_api_mcp_management::McpBatchOutcome::Succeeded,
                items: vec![],
            }),
        },
    );
    assert_eq!(app.acp_settings.enabled, before);
    let _ = super::super::mcp_management_resync::update(
        &mut app,
        super::super::mcp_management_resync::ResyncMessage::Finished {
            context,
            generation: 0,
            result: Err(desktop_settings_mcp::management_resync::ResyncFailure {
                label: "fixture".into(),
                uncertain: false,
            }),
        },
    );
    assert_eq!(app.acp_settings.enabled, before);
}

#[test]
fn managed_mcp_feedback_maps_error_effect_to_desktop_toast() {
    let mut app = App::new().0;
    let _ = super::super::mcp_management_tools::apply_effects(
        &mut app,
        iced::Task::none(),
        vec![desktop_settings_mcp::management_effect::Effect::Error("fixture MCP failure".into())],
    );
    assert_eq!(
        app.toast_state.active_toast.as_ref().map(|t| t.kind),
        Some(crate::app::state::ToastKind::Error)
    );
}

#[test]
fn mcp_desktop_routing_invalidates_completed_probe_during_mutation() {
    let mut app = App::new().0;
    let scope = super::super::mcp_management_list::context(&app).unwrap();
    app.mcp_settings.diagnostics.context =
        Some(desktop_settings_mcp::diagnostic_context::Context {
            gateway: scope.endpoint.gateway,
            tenant: scope.endpoint.tenant,
            project_path: scope.endpoint.project,
            credentials_generation: scope.credentials_generation,
            query: vw_api_app_diagnostics::DiagnosticScopeQuery {
                project_id: "fixture".into(),
                target_host_id: scope.host,
                app_id: vw_api_app_identity::AppId::Codex,
            },
            records: vec![],
            connections: vec![],
        });
    app.mcp_settings.diagnostics.targets.push((
        vw_api_mcp_management::McpRecordRef {
            id: vw_api_mcp_management::McpRecordId("fixture".into()),
            revision: 1,
        },
        desktop_apps_ui::agent_apps_diagnostics::mcp_state::Phase::Succeeded,
    ));
    app.mcp_settings.saving = true;
    let _ = app.update(Message::Settings(SettingsMessage::Mcp(McpMessage::TabSelected(
        crate::app::state::McpSettingsTab::Servers,
    ))));
    assert!(app.mcp_settings.diagnostics.targets.is_empty());
}
