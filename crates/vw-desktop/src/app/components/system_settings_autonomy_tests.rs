//! 验证自主运行设置的权限选项、边界值、帮助层与视图状态。
//! 默认拒绝、完全访问和数值极限分别覆盖安全级别的显式切换。

use super::*;
use crate::app::state::{AutonomyListKind, AutonomySettingsTab};
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::{text, text_input};
use vw_config_types::security::{
    AutonomyLevel, NonCliNaturalLanguageApprovalMode, ShellRedirectPolicy,
};

// 自主运行权限在测试内显式设置，避免继承本地高权限配置。
fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn tab_button_builds_active_and_inactive_states() {
    keep_element(tab_button("只读", true, Message::GatewayHealthTick).into());
    keep_element(tab_button("完全", false, Message::GatewayHealthTick).into());
}

#[test]
fn field_row_builds_populated_and_empty_content() {
    keep_element(field_row("级别", "决定代理默认可执行的动作范围。", text("只读")));
    keep_element(field_row("", "", text_input("", "")));
}

#[test]
fn text_row_builds_populated_and_empty_values() {
    keep_element(text_row(
        "允许命令",
        "Shell 白名单。",
        "git, cargo",
        "git\ncargo",
        |_| Message::GatewayHealthTick,
    ));
    keep_element(text_row("", "", "", "", |_| Message::GatewayHealthTick));
}

#[test]
fn bool_row_builds_checked_and_unchecked_states() {
    keep_element(bool_row(
        "工作区限制",
        "仅允许在工作区路径内操作。",
        true,
        "仅限工作区",
        |_| Message::GatewayHealthTick,
    ));
    keep_element(bool_row(
        "高风险阻断",
        "对高风险命令直接阻断。",
        false,
        "阻断高风险",
        |_| Message::GatewayHealthTick,
    ));
}

#[test]
fn view_builds_each_autonomy_level() {
    let mut app = test_app();

    for level in [AutonomyLevel::ReadOnly, AutonomyLevel::Supervised, AutonomyLevel::Full] {
        app.autonomy_settings.level = level;
        keep_element(view(&app));
    }
}

#[test]
fn autonomy_save_error_banner_follows_tabs() {
    let mut app = test_app();
    assert_eq!(Tree::new(view(&app).as_widget()).children.len(), 4);

    app.autonomy_settings.save_error = Some("autonomy save failed".to_string());
    assert_eq!(Tree::new(view(&app).as_widget()).children.len(), 5);
}

#[test]
fn view_builds_each_redirect_and_natural_language_mode_combination() {
    let mut app = test_app();

    for redirect_policy in
        [ShellRedirectPolicy::Block, ShellRedirectPolicy::Strip, ShellRedirectPolicy::Allow]
    {
        for approval_mode in [
            NonCliNaturalLanguageApprovalMode::Disabled,
            NonCliNaturalLanguageApprovalMode::RequestConfirm,
            NonCliNaturalLanguageApprovalMode::Direct,
        ] {
            app.autonomy_settings.shell_redirect_policy = redirect_policy;
            app.autonomy_settings.non_cli_natural_language_approval_mode = approval_mode;
            keep_element(view(&app));
        }
    }
}

#[test]
fn view_builds_each_boolean_policy_combination() {
    let mut app = test_app();

    for workspace_only in [false, true] {
        for require_medium_approval in [false, true] {
            for block_high_risk in [false, true] {
                app.autonomy_settings.workspace_only = workspace_only;
                app.autonomy_settings.require_approval_for_medium_risk = require_medium_approval;
                app.autonomy_settings.block_high_risk_commands = block_high_risk;
                keep_element(view(&app));
            }
        }
    }
}

#[test]
fn view_builds_budget_input_boundaries() {
    let mut app = test_app();

    for (max_actions, max_cost) in [(1, 1), (1_000_000, 100_000_000)] {
        app.autonomy_settings.max_actions_per_hour = max_actions;
        app.autonomy_settings.max_cost_per_day_cents = max_cost;
        keep_element(view(&app));
    }
}

#[test]
fn view_builds_empty_and_unicode_list_items() {
    let mut app = test_app();
    for kind in AutonomyListKind::ALL {
        app.autonomy_settings.list_mut(kind).clear();
    }
    app.autonomy_settings.allowed_commands = vec!["cargo test".to_string()];
    app.autonomy_settings.forbidden_paths = vec!["/工作区/私密 目录".to_string()];
    app.autonomy_settings.auto_approve = vec!["retired_tool".to_string()];
    app.autonomy_settings.active_tab = AutonomySettingsTab::ToolsAndPaths;
    keep_element(view(&app));

    app.autonomy_settings.non_cli_excluded_tools = vec!["未知工具".to_string()];
    app.autonomy_settings.active_tab = AutonomySettingsTab::NonCli;
    keep_element(view(&app));
}

#[test]
fn list_editor_renders_one_row_per_saved_item() {
    let items = vec!["cargo".to_string(), "git status".to_string(), "未知工具".to_string()];
    let element =
        list_editor(AutonomyListKind::AllowedCommands, "允许命令", "说明", &items, "", None, &[]);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 3);
    let saved_item_rows = &tree.children[2].children[0];
    assert_eq!(saved_item_rows.children.len(), items.len());
    let source = include_str!("system_settings_autonomy.rs");
    assert!(source.contains("LIST_EDITOR_MAX_HEIGHT"));
    assert!(source.contains("Scrollbar::new().width(4).scroller_width(4)"));
    assert!(source.contains("settings_pick_list_style"));
    assert!(source.contains("settings_pick_list_menu_style"));
    assert!(source.contains("right: 12.0"));
    assert!(!source.contains("Font::MONOSPACE"));
}

#[test]
fn overlays_return_dialog_or_build_help_modal() {
    let mut app = test_app();
    let base = iced::widget::container(text("dialog")).into();
    let unchanged = view_overlays(&app, base);
    keep_element(unchanged);

    app.autonomy_settings.show_help_modal = true;
    let base = iced::widget::container(text("dialog")).into();
    let help = view_overlays(&app, base);
    keep_element(help);
}

#[test]
fn autonomy_help_example_keeps_agent_tool_available() {
    let source = include_str!("system_settings_autonomy.rs");

    assert!(!source.contains("\"AgentTool\","));
}

#[test]
fn view_renders_only_selected_autonomy_tab() {
    let mut app = test_app();

    for (tab, panel_children) in [
        (AutonomySettingsTab::Permissions, 11),
        (AutonomySettingsTab::Budget, 3),
        (AutonomySettingsTab::ToolsAndPaths, 11),
        (AutonomySettingsTab::NonCli, 5),
    ] {
        app.autonomy_settings.active_tab = tab;
        let element = view(&app);
        let tree = Tree::new(element.as_widget());

        assert_eq!(tree.children.len(), 4);
        assert_eq!(tree.children[3].children.len(), panel_children);
    }
}
