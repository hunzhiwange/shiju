use super::*;
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::{text, text_input};
use vw_config_types::security::{
    AutonomyLevel, NonCliNaturalLanguageApprovalMode, ShellRedirectPolicy,
};

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
fn view_builds_each_redirect_and_natural_language_mode_combination() {
    let mut app = test_app();

    for redirect_policy in [ShellRedirectPolicy::Block, ShellRedirectPolicy::Strip] {
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
fn view_builds_budget_slider_boundaries() {
    let mut app = test_app();

    for (max_actions, max_cost) in [(1, 1), (10_000, 1_000_000)] {
        app.autonomy_settings.max_actions_per_hour = max_actions;
        app.autonomy_settings.max_cost_per_day_cents = max_cost;
        keep_element(view(&app));
    }
}

#[test]
fn view_builds_empty_multiline_and_unicode_text_inputs() {
    let mut app = test_app();
    let input_sets = [
        ["", "", "", "", "", "", "", "", ""],
        [
            "git\ncargo",
            "/etc\n/root",
            "RUST_LOG\nPATH",
            "file_read\nmemory_recall",
            "shell\nfile_write",
            "/workspace\n/safe-root",
            "browser\nhttp_request",
            "telegram:alice\ndiscord:bob",
            "telegram:direct\ndiscord:request_confirm",
        ],
        [
            "命令-α",
            "/工作区/私密",
            "环境_变量",
            "自动审批工具",
            "始终询问工具",
            "/允许/根目录",
            "非命令行工具",
            "频道:审批人",
            "频道:disabled",
        ],
    ];

    for inputs in input_sets {
        app.autonomy_settings.allowed_commands_input = inputs[0].to_string();
        app.autonomy_settings.forbidden_paths_input = inputs[1].to_string();
        app.autonomy_settings.shell_env_passthrough_input = inputs[2].to_string();
        app.autonomy_settings.auto_approve_input = inputs[3].to_string();
        app.autonomy_settings.always_ask_input = inputs[4].to_string();
        app.autonomy_settings.allowed_roots_input = inputs[5].to_string();
        app.autonomy_settings.non_cli_excluded_tools_input = inputs[6].to_string();
        app.autonomy_settings.non_cli_approval_approvers_input = inputs[7].to_string();
        app.autonomy_settings.non_cli_natural_language_approval_mode_by_channel_input =
            inputs[8].to_string();
        keep_element(view(&app));
    }
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
fn view_keeps_all_policy_budget_path_and_non_cli_rows() {
    let app = test_app();
    let element = view(&app);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 9);
    assert_eq!(tree.children[2].children.len(), 11);
    assert_eq!(tree.children[4].children.len(), 3);
    assert_eq!(tree.children[6].children.len(), 11);
    assert_eq!(tree.children[8].children.len(), 5);
}
