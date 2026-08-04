use super::{new_agent_request, sort_acp_agent_names, update};
use crate::app::config::AcpSettingsSnapshot;
use crate::app::message::settings::{AcpMessage, SettingsMessage};
use std::collections::BTreeMap;
use vw_config_types::config::AcpAgentConfig;

fn acp_config(command: &str) -> AcpAgentConfig {
    AcpAgentConfig { command: command.to_string(), ..AcpAgentConfig::default() }
}

fn snapshot(entries: &[(&str, bool)]) -> AcpSettingsSnapshot {
    let mut catalog = BTreeMap::new();
    let mut enabled = BTreeMap::new();

    for (agent, is_enabled) in entries {
        let config = acp_config(&format!("{agent}-command"));
        catalog.insert((*agent).to_string(), config.clone());
        if *is_enabled {
            enabled.insert((*agent).to_string(), config);
        }
    }

    AcpSettingsSnapshot { catalog, enabled }
}

#[test]
fn acp_settings_message_tests_are_wired() {
    assert!(module_path!().contains("acp_tests"));
}

#[test]
fn sort_acp_agent_names_keeps_codex_first_then_lexicographic_order() {
    let sorted = sort_acp_agent_names(vec![
        "opencode".to_string(),
        "claude".to_string(),
        "codex".to_string(),
        "gemini".to_string(),
    ]);

    assert_eq!(sorted, vec!["codex", "claude", "gemini", "opencode"]);
}

#[test]
fn sort_acp_agent_names_handles_empty_and_missing_codex_lists() {
    assert!(sort_acp_agent_names(Vec::new()).is_empty());

    let sorted =
        sort_acp_agent_names(vec!["zeta".to_string(), "alpha".to_string(), "openclaw".to_string()]);

    assert_eq!(sorted, vec!["alpha", "openclaw", "zeta"]);
}

#[test]
fn sort_acp_agent_names_is_stable_for_duplicate_names() {
    let sorted =
        sort_acp_agent_names(vec!["codex".to_string(), "codex".to_string(), "agent".to_string()]);

    assert_eq!(sorted, vec!["codex", "codex", "agent"]);
}

#[test]
fn copied_command_state_only_clears_matching_agent() {
    let (mut app, _) = crate::app::App::new();

    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::CopyCommand {
            agent: "codex".to_string(),
            command: "npx -y @agentclientprotocol/codex-acp@latest".to_string(),
        }),
    );
    assert_eq!(app.acp_settings.copied_agent.as_deref(), Some("codex"));

    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::ClearCopiedCommand("claude".to_string())),
    );
    assert_eq!(app.acp_settings.copied_agent.as_deref(), Some("codex"));

    let _ =
        update(&mut app, SettingsMessage::Acp(AcpMessage::ClearCopiedCommand("codex".to_string())));
    assert_eq!(app.acp_settings.copied_agent, None);
}

#[test]
fn refresh_marks_loading_and_clears_previous_feedback() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.loading = false;
    app.acp_settings.save_error = Some("old error".to_string());
    app.acp_settings.status_message = Some("old status".to_string());

    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::Refresh));

    assert!(app.acp_settings.loading);
    assert!(app.acp_settings.save_error.is_none());
    assert!(app.acp_settings.status_message.is_none());
}

#[test]
fn loaded_snapshot_replaces_state_sorts_enabled_agents_and_clears_disabled_selection() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.loading = true;
    app.acp_settings.save_error = Some("old error".to_string());
    app.acp_settings.catalog.insert("stale".to_string(), acp_config("stale"));
    app.acp_settings.enabled.insert("stale".to_string());
    app.acp_agents = vec!["stale".to_string()];
    app.acp_agent = Some("stale".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::Loaded(Ok(snapshot(&[
            ("opencode", true),
            ("disabled", false),
            ("codex", true),
            ("claude", true),
        ])))),
    );

    assert!(!app.acp_settings.loading);
    assert_eq!(
        app.acp_settings.catalog.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["claude", "codex", "disabled", "opencode"]
    );
    assert_eq!(
        app.acp_settings.enabled.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["claude", "codex", "opencode"]
    );
    assert_eq!(app.acp_agents, vec!["codex", "claude", "opencode"]);
    assert!(app.acp_agent.is_none());
    assert!(app.acp_settings.save_error.is_none());
}

#[test]
fn loaded_snapshot_preserves_selection_when_agent_remains_enabled() {
    let (mut app, _) = crate::app::App::new();
    app.acp_agent = Some("claude".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::Loaded(Ok(snapshot(&[("claude", true)])))),
    );

    assert_eq!(app.acp_agent.as_deref(), Some("claude"));
}

#[test]
fn loaded_error_stops_loading_and_preserves_existing_snapshot() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.loading = true;
    app.acp_settings.catalog.insert("codex".to_string(), acp_config("codex"));
    app.acp_settings.enabled.insert("codex".to_string());
    app.acp_agents = vec!["codex".to_string()];

    let _ =
        update(&mut app, SettingsMessage::Acp(AcpMessage::Loaded(Err("load failed".to_string()))));

    assert!(!app.acp_settings.loading);
    assert_eq!(app.acp_settings.save_error.as_deref(), Some("load failed"));
    assert!(app.acp_settings.catalog.contains_key("codex"));
    assert!(app.acp_settings.enabled.contains("codex"));
    assert_eq!(app.acp_agents, vec!["codex"]);
}

#[test]
fn set_enabled_marks_agent_saving_and_clears_previous_feedback() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.catalog.insert("codex".to_string(), acp_config("codex"));
    app.acp_settings.save_error = Some("old error".to_string());
    app.acp_settings.status_message = Some("old status".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::SetEnabled { agent: "codex".to_string(), enabled: true }),
    );

    assert_eq!(app.acp_settings.saving_agent.as_deref(), Some("codex"));
    assert!(app.acp_settings.save_error.is_none());
    assert!(app.acp_settings.status_message.is_none());
}

#[test]
fn set_enabled_completed_success_applies_snapshot_and_reports_requested_action() {
    for (enabled, expected_status) in [(true, "codex 已启用"), (false, "codex 已禁用")] {
        let (mut app, _) = crate::app::App::new();
        app.acp_settings.saving_agent = Some("codex".to_string());
        app.acp_settings.save_error = Some("old error".to_string());

        let _ = update(
            &mut app,
            SettingsMessage::Acp(AcpMessage::SetEnabledCompleted {
                agent: "codex".to_string(),
                enabled,
                result: Ok(snapshot(&[("codex", enabled)])),
            }),
        );

        assert!(app.acp_settings.saving_agent.is_none());
        assert_eq!(app.acp_settings.enabled.contains("codex"), enabled);
        assert!(app.acp_settings.save_error.is_none());
        assert_eq!(app.acp_settings.status_message.as_deref(), Some(expected_status));
    }
}

#[test]
fn set_enabled_completed_error_clears_saving_and_preserves_existing_state() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.saving_agent = Some("codex".to_string());
    app.acp_settings.catalog.insert("codex".to_string(), acp_config("codex"));
    app.acp_settings.enabled.insert("codex".to_string());
    app.acp_settings.status_message = Some("keep status".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::SetEnabledCompleted {
            agent: "codex".to_string(),
            enabled: false,
            result: Err("save failed".to_string()),
        }),
    );

    assert!(app.acp_settings.saving_agent.is_none());
    assert_eq!(app.acp_settings.save_error.as_deref(), Some("save failed"));
    assert!(app.acp_settings.catalog.contains_key("codex"));
    assert!(app.acp_settings.enabled.contains("codex"));
    assert_eq!(app.acp_settings.status_message.as_deref(), Some("keep status"));
}

#[test]
fn new_agent_input_messages_update_form_state() {
    let (mut app, _) = crate::app::App::new();

    for message in [
        AcpMessage::NewAgentPanelToggled,
        AcpMessage::NewAgentNameChanged(" local ".to_string()),
        AcpMessage::NewAgentCommandChanged(" npx ".to_string()),
        AcpMessage::NewAgentArgsChanged("-y, local-acp".to_string()),
    ] {
        let _ = update(&mut app, SettingsMessage::Acp(message));
    }

    assert!(app.acp_settings.new_agent_expanded);
    assert_eq!(app.acp_settings.new_agent_name_input, " local ");
    assert_eq!(app.acp_settings.new_agent_command_input, " npx ");
    assert_eq!(app.acp_settings.new_agent_args_input, "-y, local-acp");
}

#[test]
fn new_agent_request_validates_inputs_and_rejects_existing_names() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.catalog.insert("codex".to_string(), acp_config("codex"));

    assert_eq!(new_agent_request(&app).expect_err("blank name should fail"), "ACP 名称不能为空");

    app.acp_settings.new_agent_name_input = "codex".to_string();
    app.acp_settings.new_agent_command_input = "codex-acp".to_string();
    assert_eq!(
        new_agent_request(&app).expect_err("existing name should fail"),
        "ACP 名称已存在: codex"
    );

    app.acp_settings.new_agent_name_input = " local ".to_string();
    app.acp_settings.new_agent_command_input = " npx ".to_string();
    app.acp_settings.new_agent_args_input = "-y,\nlocal-acp".to_string();
    let (name, spec) = new_agent_request(&app).expect("valid input should build a request");
    assert_eq!(name, "local");
    assert_eq!(spec.command, "npx");
    assert_eq!(spec.args, vec!["-y", "local-acp"]);
}

#[test]
fn add_agent_tracks_progress_and_success_resets_form() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.new_agent_expanded = true;
    app.acp_settings.new_agent_name_input = "local".to_string();
    app.acp_settings.new_agent_command_input = "local-acp".to_string();
    app.acp_settings.new_agent_args_input = "--stdio".to_string();

    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::AddAgent));
    assert!(app.acp_settings.adding_agent);
    assert!(app.acp_settings.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::AddAgentCompleted {
            agent: "local".to_string(),
            result: Ok(snapshot(&[("local", true)])),
        }),
    );

    assert!(!app.acp_settings.adding_agent);
    assert!(!app.acp_settings.new_agent_expanded);
    assert!(app.acp_settings.new_agent_name_input.is_empty());
    assert!(app.acp_settings.new_agent_command_input.is_empty());
    assert!(app.acp_settings.new_agent_args_input.is_empty());
    assert!(app.acp_settings.catalog.contains_key("local"));
    assert_eq!(app.acp_settings.status_message.as_deref(), Some("local 已新增并启用"));
}

#[test]
fn add_agent_validation_error_keeps_form_editable() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.new_agent_expanded = true;

    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::AddAgent));

    assert!(!app.acp_settings.adding_agent);
    assert_eq!(app.acp_settings.save_error.as_deref(), Some("ACP 名称不能为空"));
    assert!(app.acp_settings.new_agent_expanded);
}

#[test]
fn update_ignores_non_acp_settings_messages() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.loading = true;
    app.acp_settings.saving_agent = Some("codex".to_string());
    app.acp_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.acp_settings.loading);
    assert_eq!(app.acp_settings.saving_agent.as_deref(), Some("codex"));
    assert_eq!(app.acp_settings.save_error.as_deref(), Some("unchanged"));
}
