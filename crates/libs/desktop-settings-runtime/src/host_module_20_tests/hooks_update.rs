use crate::{
    hooks::{Effect, update},
    messages::HooksMessage,
    state::HooksSettingsState,
};
#[test]
fn plugin_hooks_toggle_persists_config() {
    let mut state = HooksSettingsState::default();
    state.plugins_enabled = true;
    state.save_error = Some("old".into());
    assert!(matches!(
        update(&mut state, HooksMessage::PluginHooksToggled(false)),
        Effect::Save { plugins_enabled: false, .. }
    ));
    assert!(!state.plugins_enabled);
    assert!(state.save_error.is_none());
}

use crate::messages::SettingsMessage;
fn apply(state: &mut HooksSettingsState, message: SettingsMessage) -> Effect {
    let SettingsMessage::Hooks(message) = message else { panic!("expected hook message") };
    update(state, message)
}
fn catalog_entry(trusted: bool) -> gateway_desktop_settings::HookCatalogEntryDto {
    gateway_desktop_settings::HookCatalogEntryDto {
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

fn audit_entry() -> gateway_desktop_settings::HookAuditEntryDto {
    gateway_desktop_settings::HookAuditEntryDto {
        timestamp: "2026-08-16T12:00:00Z".to_string(),
        tool_name: "read_file".to_string(),
        duration_ms: 8,
        success: true,
    }
}

#[test]
fn hooks_tab_loads_catalog_and_audit() {
    let mut app = HooksSettingsState::default();
    let catalog_task = apply(&mut app, SettingsMessage::Hooks(HooksMessage::CatalogRefresh));
    assert!(app.catalog_loading);
    assert!(app.catalog_error.is_none());
    assert!(matches!(catalog_task, Effect::CatalogRefresh));

    let _ = apply(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::CatalogLoaded(Ok(vec![catalog_entry(false)]))),
    );
    assert!(!app.catalog_loading);
    assert_eq!(app.catalog, vec![catalog_entry(false)]);

    let audit_task = apply(&mut app, SettingsMessage::Hooks(HooksMessage::AuditRefresh));
    assert!(app.audit_loading);
    assert!(app.audit_error.is_none());
    assert!(matches!(audit_task, Effect::AuditRefresh));

    let _ =
        apply(&mut app, SettingsMessage::Hooks(HooksMessage::AuditLoaded(Ok(vec![audit_entry()]))));
    assert!(!app.audit_loading);
    assert_eq!(app.audit, vec![audit_entry()]);
}
#[test]
fn hook_trust_result_refreshes_and_toasts() {
    let mut app = HooksSettingsState::default();
    app.catalog = vec![catalog_entry(false)];
    app.mutation_loading = true;
    app.mutation_error = Some("old".to_string());

    let request_task = apply(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::TrustRequested {
            plugin_id: "reviewed-plugin".to_string(),
            hook_hash: "sha256:reviewed".to_string(),
        }),
    );
    assert!(app.mutation_loading);
    assert!(app.mutation_error.is_none());
    assert!(matches!(request_task, Effect::Trust { .. }));

    let effect = apply(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::MutationCompleted(Ok(
            gateway_desktop_settings::HookMutationResponseDto {
                plugin_id: "reviewed-plugin".to_string(),
                hook_hash: "sha256:reviewed".to_string(),
                trusted: true,
            },
        ))),
    );

    assert!(matches!(effect, Effect::MutationSucceeded { .. }));
    assert!(!app.mutation_loading);
    assert!(app.mutation_error.is_none());
    let revoke_task = apply(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::RevokeRequested {
            plugin_id: "reviewed-plugin".to_string(),
            hook_hash: "sha256:reviewed".to_string(),
        }),
    );
    assert!(app.mutation_loading);
    assert!(app.mutation_error.is_none());
    assert!(matches!(revoke_task, Effect::Revoke { .. }));
}
#[test]
fn hook_load_failure_preserves_last_good_data() {
    let mut app = HooksSettingsState::default();
    app.catalog = vec![catalog_entry(true)];
    app.audit = vec![audit_entry()];
    app.catalog_loading = true;
    app.audit_loading = true;

    let _ = apply(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::CatalogLoaded(Err("catalog unavailable".to_string()))),
    );
    assert_eq!(app.catalog, vec![catalog_entry(true)]);
    assert!(!app.catalog_loading);
    assert_eq!(app.catalog_error.as_deref(), Some("catalog unavailable"));

    let _ = apply(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::AuditLoaded(Err("audit unavailable".to_string()))),
    );
    assert_eq!(app.audit, vec![audit_entry()]);
    assert!(!app.audit_loading);
    assert_eq!(app.audit_error.as_deref(), Some("audit unavailable"));

    let catalog_snapshot = app.catalog.clone();
    app.mutation_loading = true;
    let _ = apply(
        &mut app,
        SettingsMessage::Hooks(HooksMessage::MutationCompleted(Err(
            "mutation unavailable".to_string()
        ))),
    );
    assert_eq!(app.catalog, catalog_snapshot);
    assert!(!app.mutation_loading);
    assert_eq!(app.mutation_error.as_deref(), Some("mutation unavailable"));
}
