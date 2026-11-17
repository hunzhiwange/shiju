// 验证自主模式配置解析、字段更新与风险选项处理。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;

use desktop_settings_security::app_init::state::{AutonomyListKind, AutonomySettingsTab};
use config_security::{
    AutonomyLevel, NonCliNaturalLanguageApprovalMode, ShellRedirectPolicy,
};

fn app() -> AutonomySettingsState { AutonomySettingsState::default() }

fn apply(app: &mut AutonomySettingsState, message: crate::autonomy_message::Message) {
    let _ = update(app, message);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn parses_non_cli_mode_by_channel_with_aliases() {
    let parsed = parse_non_cli_mode_by_channel(
        " Telegram : direct, slack:request-confirm\nDiscord:disabled\nbad-entry\nqq:unknown",
    );

    assert_eq!(parsed.get("telegram"), Some(&NonCliNaturalLanguageApprovalMode::Direct));
    assert_eq!(parsed.get("slack"), Some(&NonCliNaturalLanguageApprovalMode::RequestConfirm));
    assert_eq!(parsed.get("discord"), Some(&NonCliNaturalLanguageApprovalMode::Disabled));
    assert!(!parsed.contains_key("qq"));
}

#[test]
fn parses_non_cli_mode_by_channel_returns_empty_for_blank_and_invalid_entries() {
    assert!(parse_non_cli_mode_by_channel("  ,\n\t ").is_empty());
    assert!(parse_non_cli_mode_by_channel("missing-mode, :direct, slack:prompt").is_empty());
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn update_tracks_field_changes_and_help_modal() {
    let mut app = app();
    app.save_error = Some("old".to_string());

    let _ = update(&mut app, crate::autonomy_message::Message::AutonomyLevelChanged(AutonomyLevel::Full));
    let _ = update(&mut app, crate::autonomy_message::Message::AutonomyWorkspaceOnlyToggled(true));
    let _ = update(&mut app, crate::autonomy_message::Message::AutonomyMaxActionsPerHourChanged(0));
    let _ = update(&mut app, crate::autonomy_message::Message::AutonomyMaxCostPerDayCentsChanged(u32::MAX));
    let _ = update(
        &mut app,
        crate::autonomy_message::Message::AutonomyShellRedirectPolicyChanged(ShellRedirectPolicy::Strip),
    );
    let _ = update(
        &mut app,
        crate::autonomy_message::Message::AutonomyNonCliNaturalLanguageApprovalModeChanged(
            NonCliNaturalLanguageApprovalMode::RequestConfirm,
        ),
    );
    let _ = update(
        &mut app,
        crate::autonomy_message::Message::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(
            "slack:direct".to_string(),
        ),
    );

    assert_eq!(app.level, AutonomyLevel::Full);
    assert!(app.workspace_only);
    assert_eq!(app.max_actions_per_hour, 1);
    assert_eq!(app.max_cost_per_day_cents, 100_000_000);
    assert_eq!(app.shell_redirect_policy, ShellRedirectPolicy::Strip);
    assert_eq!(
        app.non_cli_natural_language_approval_mode,
        NonCliNaturalLanguageApprovalMode::RequestConfirm
    );
    assert_eq!(
        app.non_cli_natural_language_approval_mode_by_channel_input,
        "slack:direct"
    );
    assert!(app.save_error.is_none());

    let _ = update(&mut app, crate::autonomy_message::Message::AutonomyHelpOpen);
    assert!(app.show_help_modal);
    let _ = update(&mut app, crate::autonomy_message::Message::AutonomyHelpClose);
    assert!(!app.show_help_modal);
}

#[test]
fn update_tracks_risk_and_legacy_text_field_changes() {
    let mut app = app();
    app.save_error = Some("old".to_string());

    apply(&mut app, crate::autonomy_message::Message::AutonomyRequireApprovalForMediumRiskToggled(false));
    apply(&mut app, crate::autonomy_message::Message::AutonomyBlockHighRiskCommandsToggled(false));
    apply(
        &mut app,
        crate::autonomy_message::Message::AutonomyNonCliApprovalApproversChanged("alice\nbob".to_string()),
    );

    assert!(!app.require_approval_for_medium_risk);
    assert!(!app.block_high_risk_commands);
    assert_eq!(app.non_cli_approval_approvers_input, "alice\nbob");
    assert!(app.save_error.is_none());
}

fn list_snapshots(app: &AutonomySettingsState) -> [Vec<String>; 7] {
    [AutonomyListKind::AllowedCommands, AutonomyListKind::ForbiddenPaths, AutonomyListKind::ShellEnvPassthrough, AutonomyListKind::AutoApprove, AutonomyListKind::AlwaysAsk, AutonomyListKind::AllowedRoots, AutonomyListKind::NonCliExcludedTools].map(|kind| app.list(kind).to_vec())
}

#[test]
fn each_autonomy_list_adds_trimmed_values_and_removes_by_index_in_isolation() {
    let cases = [
        (AutonomyListKind::AllowedCommands, "  cargo test  "),
        (AutonomyListKind::ForbiddenPaths, "  /工作区/私密  "),
        (AutonomyListKind::ShellEnvPassthrough, "  RUST_LOG  "),
        (AutonomyListKind::AutoApprove, "  file_read  "),
        (AutonomyListKind::AlwaysAsk, "  shell  "),
        (AutonomyListKind::AllowedRoots, "  /tmp/safe  "),
        (AutonomyListKind::NonCliExcludedTools, "  legacy_tool  "),
    ];

    for (kind, input) in cases {
        let mut app = app();
        for list_kind in [AutonomyListKind::AllowedCommands, AutonomyListKind::ForbiddenPaths, AutonomyListKind::ShellEnvPassthrough, AutonomyListKind::AutoApprove, AutonomyListKind::AlwaysAsk, AutonomyListKind::AllowedRoots, AutonomyListKind::NonCliExcludedTools] {
            app.list_mut(list_kind).clear();
        }
        let before = list_snapshots(&app);

        apply(&mut app, crate::autonomy_message::Message::AutonomyListDraftChanged(kind, input.to_string()));
        apply(&mut app, crate::autonomy_message::Message::AutonomyListAdd(kind));

        let after_add = list_snapshots(&app);
        assert_eq!(app.list(kind), &[input.trim().to_string()]);
        for other in [AutonomyListKind::AllowedCommands, AutonomyListKind::ForbiddenPaths, AutonomyListKind::ShellEnvPassthrough, AutonomyListKind::AutoApprove, AutonomyListKind::AlwaysAsk, AutonomyListKind::AllowedRoots, AutonomyListKind::NonCliExcludedTools] {
            if other != kind {
                assert_eq!(after_add[other.index()], before[other.index()]);
            }
        }
        assert!(app.list_editor.draft(kind).is_empty());

        apply(&mut app, crate::autonomy_message::Message::AutonomyListRemoveConfirmed(kind, 0));
        assert_eq!(app.list(kind).len(), 1);

        apply(&mut app, crate::autonomy_message::Message::AutonomyListRemoveRequested(kind, 0));
        assert!(app.list_editor.is_remove_pending(kind, 0));
        assert_eq!(app.list(kind).len(), 1);
        apply(&mut app, crate::autonomy_message::Message::AutonomyListRemoveCanceled);
        assert!(!app.list_editor.is_remove_pending(kind, 0));

        apply(&mut app, crate::autonomy_message::Message::AutonomyListRemoveRequested(kind, 0));
        apply(&mut app, crate::autonomy_message::Message::AutonomyListRemoveConfirmed(kind, 0));
        assert!(app.list(kind).is_empty());
    }
}

#[test]
fn autonomy_list_add_rejects_blank_duplicate_and_invalid_environment_names() {
    let mut app = app();
    app.allowed_commands = vec!["cargo".to_string()];

    for input in [" ", " cargo "] {
        apply(
            &mut app,
            crate::autonomy_message::Message::AutonomyListDraftChanged(
                AutonomyListKind::AllowedCommands,
                input.to_string(),
            ),
        );
        apply(&mut app, crate::autonomy_message::Message::AutonomyListAdd(AutonomyListKind::AllowedCommands));
        assert_eq!(app.allowed_commands, ["cargo"]);
        assert!(app.save_error.is_some());
    }

    for invalid in ["1PATH", "BAD-NAME", "中文变量"] {
        apply(
            &mut app,
            crate::autonomy_message::Message::AutonomyListDraftChanged(
                AutonomyListKind::ShellEnvPassthrough,
                invalid.to_string(),
            ),
        );
        apply(&mut app, crate::autonomy_message::Message::AutonomyListAdd(AutonomyListKind::ShellEnvPassthrough));
        assert!(app.shell_env_passthrough.is_empty());
        assert!(app.save_error.is_some());
    }
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn structured_lists_are_cloned_directly_into_persisted_autonomy_config() {
    let mut app = app();
    app.allowed_commands = vec!["cargo test --workspace".to_string()];
    app.forbidden_paths = vec!["/私密 目录".to_string()];
    app.shell_env_passthrough = vec!["RUST_LOG".to_string()];
    app.auto_approve = vec!["file_read".to_string()];
    app.always_ask = vec!["shell".to_string()];
    app.allowed_roots = vec!["/工作区".to_string()];
    app.non_cli_excluded_tools = vec!["retired_tool".to_string()];
    let mut config = config_security::AutonomyConfig::default();

    apply_autonomy_settings_to_config(&app, &mut config);

    for kind in [AutonomyListKind::AllowedCommands, AutonomyListKind::ForbiddenPaths, AutonomyListKind::ShellEnvPassthrough, AutonomyListKind::AutoApprove, AutonomyListKind::AlwaysAsk, AutonomyListKind::AllowedRoots, AutonomyListKind::NonCliExcludedTools] {
        assert_eq!(config_list(&config, kind), app.list(kind));
    }
}

#[test]
fn update_clamps_both_resource_bounds_and_save_clears_error() {
    let mut app = app();

    apply(&mut app, crate::autonomy_message::Message::AutonomyMaxActionsPerHourChanged(u32::MAX));
    assert_eq!(app.max_actions_per_hour, 1_000_000);

    apply(&mut app, crate::autonomy_message::Message::AutonomyMaxCostPerDayCentsChanged(0));
    assert_eq!(app.max_cost_per_day_cents, 1);

    app.save_error = Some("retry".to_string());
    apply(&mut app, crate::autonomy_message::Message::AutonomySave);
    assert!(app.save_error.is_none());
}

#[test]
fn update_switches_tab_without_persisting() {
    let mut app = app();
    app.save_error = Some("keep".to_string());

    apply(&mut app, crate::autonomy_message::Message::AutonomyTabSelected(AutonomySettingsTab::NonCli));

    assert_eq!(app.active_tab, AutonomySettingsTab::NonCli);
    assert_eq!(app.save_error.as_deref(), Some("keep"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn update_ignores_non_autonomy_messages() {
    let mut app = app();
    app.save_error = Some("unchanged".to_string());
    let before = app.clone();

    apply(&mut app, crate::autonomy_message::Message::None);

    assert_eq!(app.level, before.level);
    assert_eq!(app.workspace_only, before.workspace_only);
    assert_eq!(app.save_error, before.save_error);
    assert_eq!(app.show_help_modal, before.show_help_modal);
}
