#![cfg(not(target_arch = "wasm32"))]
use crate::autonomy_message::Message;
use crate::autonomy_test_support::*;
use crate::system_settings_autonomy::view;
use config_security::{AutonomyLevel, NonCliNaturalLanguageApprovalMode, ShellRedirectPolicy};
use desktop_i18n::I18n;
use desktop_settings_security::app_init::state::{AutonomySettingsState, AutonomySettingsTab};
use iced::{Size, mouse};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_11_permissions_explanations() {
    if !isolated(module_path!(), "settings_help_gap_11_permissions_explanations") {
        return;
    }
    let mut state = AutonomySettingsState::default();
    let tools: Vec<String> = (0..28).map(|index| format!("fixture_tool_{index:02}")).collect();
    state.active_tab = AutonomySettingsTab::Permissions;
    state.require_approval_for_medium_risk = false;
    state.block_high_risk_commands = false;
    let mut renderer = renderer().await;
    for (locale, theme) in HELP_LOCALE_THEMES {
        let app = inputs(&state, &tools, locale, &theme);
        let t = |key| I18n::new(locale).__(key);
        for width in [420, 760] {
            let size = Size::new(width, 1100);
            let mut ui = scene(view(&app), &mut renderer, size);
            for (title, description) in [
                ("重定向策略", "控制 shell 命令中的输出重定向如何处理。"),
                ("级别", "决定代理默认可执行的动作范围。"),
                ("自然语言审批", "控制非 CLI 场景下自然语言请求的执行方式。"),
            ] {
                assert_help_card(
                    &app,
                    &mut ui,
                    &mut renderer,
                    size,
                    t(title),
                    t(description),
                    t(description),
                );
            }
            let positions = geometry(&mut ui, &renderer);
            for hint in [
                "对中风险动作触发审批，而不是直接执行。",
                "对高风险命令直接阻断，不进入执行链路。",
                "仅允许在工作区路径内操作，除非显式加入 allowed_roots。",
            ] {
                positions.label(t(hint));
            }
            for (label, level) in [
                ("只读", AutonomyLevel::ReadOnly),
                ("监督", AutonomyLevel::Supervised),
                ("完全", AutonomyLevel::Full),
            ] {
                assert!(matches!(
                    click(&mut ui, &mut renderer, positions.label(t(label)).center()).as_slice(),
                    [Message::AutonomyLevelChanged(value)]
                        if *value == level
                ));
            }
            let mut click_label =
                |label| click(&mut ui, &mut renderer, positions.label(t(label)).center());
            assert!(matches!(
                click_label("中风险需审批").as_slice(),
                [Message::AutonomyRequireApprovalForMediumRiskToggled(true)]
            ));
            assert!(matches!(
                click_label("阻断高风险").as_slice(),
                [Message::AutonomyBlockHighRiskCommandsToggled(true)]
            ));
            assert!(matches!(
                click_label("移除").as_slice(),
                [Message::AutonomyShellRedirectPolicyChanged(ShellRedirectPolicy::Strip)]
            ));
            assert!(matches!(
                click_label("请求确认").as_slice(),
                [Message::AutonomyNonCliNaturalLanguageApprovalModeChanged(
                    NonCliNaturalLanguageApprovalMode::RequestConfirm
                )]
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_11_channel_formats() {
    if !isolated(module_path!(), "settings_help_gap_11_channel_formats") {
        return;
    }
    let mut state = AutonomySettingsState::default();
    let tools: Vec<String> = (0..28).map(|index| format!("fixture_tool_{index:02}")).collect();
    state.active_tab = AutonomySettingsTab::NonCli;
    state.non_cli_approval_approvers_input = "fixture-channel:private-user".into();
    state.non_cli_natural_language_approval_mode_by_channel_input =
        "fixture-channel:disabled".into();
    state.save_error = Some("fixture channel validation error".into());
    let mut renderer = renderer().await;
    for (locale, theme) in HELP_LOCALE_THEMES {
        let app = inputs(&state, &tools, locale, &theme);
        let t = |key| I18n::new(locale).__(key);
        for width in [420, 760] {
            let size = Size::new(width, 1000);
            let mut ui = scene(view(&app), &mut renderer, size);
            assert_help_card(
                &app,
                &mut ui,
                &mut renderer,
                size,
                t("非 CLI 排除工具"),
                t("从现有工具中选择在非 CLI 通道里默认禁用的工具。"),
                t("在下拉列表中选择工具，再点击添加。"),
            );
            for (title, description, example) in [
                (
                    "审批人列表",
                    "格式为 channel:user，支持逗号或换行分隔。",
                    "telegram:alice, *:bob",
                ),
                (
                    "按通道模式",
                    "格式为 channel:mode；列表字段支持逗号或换行分隔。",
                    "telegram:direct, discord:request_confirm",
                ),
            ] {
                assert_help_card(
                    &app,
                    &mut ui,
                    &mut renderer,
                    size,
                    t(title),
                    t(description),
                    &format!("{}\n{example}", t(description)),
                );
            }
            let positions = geometry(&mut ui, &renderer);
            for hint in ["这些工具在非 CLI 通道中默认禁用。", "按通道填写用户", "按通道填写模式"]
            {
                positions.label(t(hint));
            }
            positions.label("fixture channel validation error");
            let input = positions.input("fixture-channel:disabled").center();
            assert!(click(&mut ui, &mut renderer, input).is_empty());
            assert!(matches!(
                send(
                    &mut ui, &mut renderer, mouse::Cursor::Available(input),
                    &[key(iced::keyboard::key::Named::Backspace)]
                ).as_slice(),
                [Message::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(value)]
                    if value.len() + 1 == "fixture-channel:disabled".len()
            ));
        }
    }
}
