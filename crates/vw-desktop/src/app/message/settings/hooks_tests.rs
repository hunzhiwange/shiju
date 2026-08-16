use super::*;
use crate::app::App;
use crate::app::components::system_settings::SystemTab;
use crate::app::state::ToastKind;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

async fn collect_task_messages(task: Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

fn catalog_entry(trusted: bool) -> vw_gateway_client::HookCatalogEntryDto {
    vw_gateway_client::HookCatalogEntryDto {
        plugin_id: "reviewed-plugin".to_string(),
        event: "before_tool_call".to_string(),
        command: "bin/review-hook".to_string(),
        args: vec!["--safe".to_string()],
        working_dir: "/tmp/plugin".to_string(),
        mode: "blocking".to_string(),
        hook_hash: "sha256:reviewed".to_string(),
        trusted,
    }
}

fn audit_entry() -> vw_gateway_client::HookAuditEntryDto {
    vw_gateway_client::HookAuditEntryDto {
        timestamp: "2026-08-16T12:00:00Z".to_string(),
        tool_name: "read_file".to_string(),
        duration_ms: 8,
        success: true,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn hooks_tab_loads_catalog_and_audit() {
    let mut app = app();

    let messages = collect_task_messages(super::super::update(
        &mut app,
        SettingsMessage::SystemTabSelected(SystemTab::Hooks),
    ))
    .await;

    assert_eq!(app.system_settings_tab, SystemTab::Hooks);
    assert_eq!(messages.len(), 2);
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Hooks(HooksMessage::CatalogRefresh))
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Hooks(HooksMessage::AuditRefresh))
    )));

    let catalog_task = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::CatalogRefresh),
    );
    assert!(app.hooks_settings.catalog_loading);
    assert!(app.hooks_settings.catalog_error.is_none());
    assert!(has_scheduled_action(catalog_task));

    let _ = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::CatalogLoaded(Ok(vec![catalog_entry(false)]))),
    );
    assert!(!app.hooks_settings.catalog_loading);
    assert_eq!(app.hooks_settings.catalog, vec![catalog_entry(false)]);

    let audit_task = update(&mut app, SettingsMessage::Hooks(HooksMessage::AuditRefresh));
    assert!(app.hooks_settings.audit_loading);
    assert!(app.hooks_settings.audit_error.is_none());
    assert!(has_scheduled_action(audit_task));

    let _ = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::AuditLoaded(Ok(vec![audit_entry()]))),
    );
    assert!(!app.hooks_settings.audit_loading);
    assert_eq!(app.hooks_settings.audit, vec![audit_entry()]);
}

#[test]
fn plugin_hooks_toggle_persists_config() {
    let mut app = app();
    app.hooks_settings.plugins_enabled = true;
    app.hooks_settings.save_error = Some("old".to_string());

    let task = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::PluginHooksToggled(false)),
    );

    assert!(!app.hooks_settings.plugins_enabled);
    assert!(app.hooks_settings.save_error.is_none());
    assert!(has_scheduled_action(task));
}

#[tokio::test(flavor = "multi_thread")]
async fn hook_trust_result_refreshes_and_toasts() {
    let mut app = app();
    app.hooks_settings.catalog = vec![catalog_entry(false)];
    app.hooks_settings.mutation_loading = true;
    app.hooks_settings.mutation_error = Some("old".to_string());

    let request_task = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::TrustRequested {
            plugin_id: "reviewed-plugin".to_string(),
            hook_hash: "sha256:reviewed".to_string(),
        }),
    );
    assert!(app.hooks_settings.mutation_loading);
    assert!(app.hooks_settings.mutation_error.is_none());
    assert!(has_scheduled_action(request_task));

    let task = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::MutationCompleted(Ok(
            vw_gateway_client::HookMutationResponseDto {
                plugin_id: "reviewed-plugin".to_string(),
                hook_hash: "sha256:reviewed".to_string(),
                trusted: true,
            },
        ))),
    );

    assert!(!app.hooks_settings.mutation_loading);
    assert!(app.hooks_settings.mutation_error.is_none());
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Success));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("信任")));
    let messages = collect_task_messages(task).await;
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Hooks(HooksMessage::CatalogRefresh))
    )));

    let revoke_task = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::RevokeRequested {
            plugin_id: "reviewed-plugin".to_string(),
            hook_hash: "sha256:reviewed".to_string(),
        }),
    );
    assert!(app.hooks_settings.mutation_loading);
    assert!(app.hooks_settings.mutation_error.is_none());
    assert!(has_scheduled_action(revoke_task));
}

#[test]
fn hook_load_failure_preserves_last_good_data() {
    let mut app = app();
    app.hooks_settings.catalog = vec![catalog_entry(true)];
    app.hooks_settings.audit = vec![audit_entry()];
    app.hooks_settings.catalog_loading = true;
    app.hooks_settings.audit_loading = true;

    let _ = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::CatalogLoaded(Err(
            "catalog unavailable".to_string(),
        ))),
    );
    assert_eq!(app.hooks_settings.catalog, vec![catalog_entry(true)]);
    assert!(!app.hooks_settings.catalog_loading);
    assert_eq!(app.hooks_settings.catalog_error.as_deref(), Some("catalog unavailable"));
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Error));

    let _ = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::AuditLoaded(Err(
            "audit unavailable".to_string(),
        ))),
    );
    assert_eq!(app.hooks_settings.audit, vec![audit_entry()]);
    assert!(!app.hooks_settings.audit_loading);
    assert_eq!(app.hooks_settings.audit_error.as_deref(), Some("audit unavailable"));
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Error));

    let catalog_snapshot = app.hooks_settings.catalog.clone();
    app.hooks_settings.mutation_loading = true;
    let _ = update(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::MutationCompleted(Err(
            "mutation unavailable".to_string(),
        ))),
    );
    assert_eq!(app.hooks_settings.catalog, catalog_snapshot);
    assert!(!app.hooks_settings.mutation_loading);
    assert_eq!(app.hooks_settings.mutation_error.as_deref(), Some("mutation unavailable"));
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Error));
}

#[test]
fn enabled_toggle_updates_only_enabled_and_clears_save_error() {
    let mut app = app();
    app.hooks_settings.enabled = true;
    app.hooks_settings.command_logger = true;
    app.hooks_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::Hooks(HooksMessage::EnabledToggled(false)));

    assert!(!app.hooks_settings.enabled);
    assert!(app.hooks_settings.command_logger);
    assert!(app.hooks_settings.save_error.is_none());
}

#[test]
fn command_logger_toggle_updates_only_command_logger_and_clears_save_error() {
    let mut app = app();
    app.hooks_settings.enabled = false;
    app.hooks_settings.command_logger = false;
    app.hooks_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::Hooks(HooksMessage::CommandLoggerToggled(true)));

    assert!(!app.hooks_settings.enabled);
    assert!(app.hooks_settings.command_logger);
    assert!(app.hooks_settings.save_error.is_none());
}

#[test]
fn unrelated_message_preserves_all_hooks_state() {
    let mut app = app();
    app.hooks_settings.enabled = false;
    app.hooks_settings.command_logger = true;
    app.hooks_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::HeartbeatHelpOpen);

    assert!(!app.hooks_settings.enabled);
    assert!(app.hooks_settings.command_logger);
    assert_eq!(app.hooks_settings.save_error.as_deref(), Some("old"));
}

#[test]
fn hooks_edits_schedule_persistence_while_unrelated_messages_do_not() {
    let mut app = app();

    let enabled_task = update(&mut app, SettingsMessage::Hooks(HooksMessage::EnabledToggled(true)));
    assert!(has_scheduled_action(enabled_task));

    let logger_task =
        update(&mut app, SettingsMessage::Hooks(HooksMessage::CommandLoggerToggled(true)));
    assert!(has_scheduled_action(logger_task));

    let unrelated_task = update(&mut app, SettingsMessage::HeartbeatHelpOpen);
    assert!(!has_scheduled_action(unrelated_task));
}
