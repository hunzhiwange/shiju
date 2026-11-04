// 验证 ACP 代理排序、配置刷新与命令复制反馈状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{new_agent_request, sort_acp_agent_names, update};
use crate::app::config::AcpSettingsSnapshot;
use crate::app::message::settings::{AcpMessage, SettingsMessage};
use crate::app::state::AcpDiagnosticStatus;
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

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
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

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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
    app.acp_settings.editor_args = vec!["-y".into(), "local-acp".into()];
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
    assert_eq!(app.acp_settings.status_message.as_deref(), Some("local 已保存"));
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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

#[test]
fn acp_editor_validates_and_saves_fields() {
    let (mut app, _) = crate::app::App::new();
    let spec = AcpAgentConfig {
        command: "命令".into(),
        args: vec!["a,b".into(), " spaced ".into(), "".into()],
        env: [("TOKEN".into(), " x=y,中文 ".into())].into(),
    };
    app.acp_settings.catalog.insert("fixture".into(), spec.clone());
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::EditAgent("fixture".into())));
    assert_eq!(
        serde_json::to_value(new_agent_request(&app).unwrap().1).unwrap(),
        serde_json::to_value(&spec).unwrap()
    );
    let _ =
        update(&mut app, SettingsMessage::Acp(AcpMessage::NewAgentNameChanged("renamed".into())));
    assert_eq!(new_agent_request(&app).unwrap().0, "fixture");
    app.acp_settings.editor_env.push(("TOKEN".into(), "duplicate".into()));
    assert!(new_agent_request(&app).is_err());
    app.acp_settings.editor_env.pop();
    app.acp_settings.editor_env.push(("".into(), "value".into()));
    assert!(new_agent_request(&app).is_err());
    app.acp_settings.editor_env.pop();
    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::AddAgentCompleted {
            agent: "fixture".into(),
            result: Err("failed".into()),
        }),
    );
    assert!(app.acp_settings.new_agent_expanded);
    assert_eq!(
        serde_json::to_value(new_agent_request(&app).unwrap().1).unwrap(),
        serde_json::to_value(&spec).unwrap()
    );
    let (_, saved) = new_agent_request(&app).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    serde_json::to_writer(file.as_file(), &saved).unwrap();
    let loaded: AcpAgentConfig =
        serde_json::from_reader(std::fs::File::open(file.path()).unwrap()).unwrap();
    assert_eq!(loaded.args, spec.args);
    assert_eq!(loaded.env, spec.env);
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::CancelEditor));
    assert!(!app.acp_settings.new_agent_expanded);
    assert_eq!(
        serde_json::to_value(&app.acp_settings.catalog["fixture"]).unwrap(),
        serde_json::to_value(&spec).unwrap()
    );
}

#[test]
fn acp_diagnostic_ignores_stale_results_after_edit_and_cancel() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.new_agent_name_input = "fixture".into();
    app.acp_settings.new_agent_command_input = "draft-command".into();
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::StartDiagnostic));
    let id = app.acp_settings.diagnostic_id.clone().unwrap();
    let _ =
        update(&mut app, SettingsMessage::Acp(AcpMessage::NewAgentCommandChanged("edited".into())));
    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::DiagnosticCompleted {
            id,
            result: Ok("connected".into()),
        }),
    );
    assert!(app.acp_settings.diagnostic_status.is_none());
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::StartDiagnostic));
    let id = app.acp_settings.diagnostic_id.clone().unwrap();
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::CancelDiagnostic));
    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::DiagnosticCompleted {
            id,
            result: Ok("connected".into()),
        }),
    );
    assert_eq!(app.acp_settings.diagnostic_status, Some(AcpDiagnosticStatus::Cancelled));
}

#[test]
fn acp_diagnostic_ignores_stale_results_retry_accepts_only_current_request() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.new_agent_name_input = "fixture".into();
    app.acp_settings.new_agent_command_input = "draft-command".into();
    app.acp_settings.editor_args = vec!["a,b".into(), "with spaces".into()];
    app.acp_settings.editor_env = vec![("TOKEN".into(), "draft".into())];
    let (_, spec) = new_agent_request(&app).unwrap();
    assert_eq!(spec.args, ["a,b", "with spaces"]);
    assert_eq!(spec.env["TOKEN"], "draft");
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::StartDiagnostic));
    let old = app.acp_settings.diagnostic_id.clone().unwrap();
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::StartDiagnostic));
    let current = app.acp_settings.diagnostic_id.clone().unwrap();
    assert_ne!(old, current);
    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::DiagnosticCompleted {
            id: old,
            result: Ok("startup_failed".into()),
        }),
    );
    assert_eq!(app.acp_settings.diagnostic_id.as_deref(), Some(current.as_str()));
    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::DiagnosticCompleted {
            id: current,
            result: Ok("connected".into()),
        }),
    );
    assert_eq!(app.acp_settings.diagnostic_status, Some(AcpDiagnosticStatus::Connected));
    assert!(app.acp_settings.diagnostic_id.is_none());
}

#[test]
fn list_diagnostic_starts_without_opening_or_changing_editor() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.catalog.insert("codex".into(), acp_config("catalog-command"));
    app.acp_settings.new_agent_name_input = "unsaved draft".into();
    app.acp_settings.new_agent_command_input = "draft-command".into();
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::TestAgent("codex".into())));
    assert!(app.acp_settings.diagnostic_id.is_some());
    assert_eq!(app.acp_settings.diagnostic_results["codex"], AcpDiagnosticStatus::Running);
    assert_eq!(app.acp_settings.diagnostic_agent.as_deref(), Some("codex"));
    assert!(!app.acp_settings.new_agent_expanded);
    assert_eq!(app.acp_settings.new_agent_name_input, "unsaved draft");
    assert_eq!(app.acp_settings.new_agent_command_input, "draft-command");
}

#[test]
fn list_diagnostic_rejects_missing_agent() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.catalog.clear();
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::TestAgent("missing".into())));
    assert!(app.acp_settings.diagnostic_id.is_none());
}

#[test]
fn diagnostic_results_stay_with_each_agent_and_retry_replaces_only_its_result() {
    let (mut app, _) = crate::app::App::new();
    for agent in ["codex", "claude"] {
        app.acp_settings.catalog.insert(agent.into(), acp_config(agent));
    }
    for (agent, result, expected) in [
        ("codex", "connected", AcpDiagnosticStatus::Connected),
        ("claude", "handshake_failed", AcpDiagnosticStatus::HandshakeFailed),
        ("claude", "authentication_required", AcpDiagnosticStatus::AuthenticationRequired),
        ("claude", "startup_failed", AcpDiagnosticStatus::StartupFailed),
        ("claude", "timed_out", AcpDiagnosticStatus::TimedOut),
        ("claude", "unknown", AcpDiagnosticStatus::Unavailable),
    ] {
        let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::TestAgent(agent.into())));
        assert_eq!(app.acp_settings.diagnostic_results[agent], AcpDiagnosticStatus::Running);
        let id = app.acp_settings.diagnostic_id.clone().unwrap();
        let _ = update(
            &mut app,
            SettingsMessage::Acp(AcpMessage::DiagnosticCompleted { id, result: Ok(result.into()) }),
        );
        assert_eq!(app.acp_settings.diagnostic_results[agent], expected);
        assert_eq!(app.acp_settings.diagnostic_results["codex"], AcpDiagnosticStatus::Connected);
        assert!(app.acp_settings.diagnostic_agent.is_none());
    }
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::TestAgent("claude".into())));
    let id = app.acp_settings.diagnostic_id.clone().unwrap();
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::CancelDiagnostic));
    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::DiagnosticCompleted {
            id,
            result: Ok("connected".into()),
        }),
    );
    assert_eq!(app.acp_settings.diagnostic_results["claude"], AcpDiagnosticStatus::Cancelled);
    assert_eq!(app.acp_settings.diagnostic_results["codex"], AcpDiagnosticStatus::Connected);
}

#[test]
fn saved_configuration_change_invalidates_previous_connection_result() {
    let (mut app, _) = crate::app::App::new();
    let original = snapshot(&[("codex", true), ("claude", true)]);
    super::apply_snapshot(&mut app, original.clone());
    for agent in ["codex", "claude"] {
        app.acp_settings.diagnostic_results.insert(agent.into(), AcpDiagnosticStatus::Connected);
    }
    super::apply_snapshot(&mut app, original);
    assert_eq!(app.acp_settings.diagnostic_results.len(), 2);
    let mut changed = snapshot(&[("codex", true)]);
    changed.catalog.get_mut("codex").unwrap().args.push("--changed".into());
    super::apply_snapshot(&mut app, changed);
    assert!(app.acp_settings.diagnostic_results.is_empty());
}

#[test]
fn acp_install_tracks_busy_state_and_releases_after_failure() {
    let (mut app, _) = crate::app::App::new();
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::InstallAgent("codex".into())));
    assert_eq!(app.acp_settings.installing_agent.as_deref(), Some("codex"));
    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::InstallAgent("gemini".into())));
    assert_eq!(app.acp_settings.installing_agent.as_deref(), Some("codex"));
    let _ = update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::InstallCompleted {
            agent: "codex".into(),
            result: Err("installation failed".into()),
        }),
    );
    assert!(app.acp_settings.installing_agent.is_none());
}
