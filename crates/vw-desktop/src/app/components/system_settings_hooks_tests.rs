//! 验证钩子设置的开关消息、信任操作、审计区与视图组合。
//! 已信任和待审查目录项分开构造，确保危险操作映射到目标钩子。

use super::*;
use crate::app::App;
use iced::advanced::widget::Tree;

const SOURCE: &str = include_str!("system_settings_hooks.rs");

// 钩子目录与审计记录由测试逐项注入，默认应用不携带外部插件状态。
fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn tree_node_count(tree: &Tree) -> usize {
    1 + tree.children.iter().map(tree_node_count).sum::<usize>()
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

fn hooks_message(message: Message) -> HooksMessage {
    match message {
        Message::Settings(message::SettingsMessage::Hooks(hooks)) => hooks,
        _ => panic!("expected hooks settings message"),
    }
}

#[test]
fn system_settings_hooks_tests_are_wired() {
    assert!(module_path!().contains("system_settings_hooks_tests"));
}

#[test]
fn field_row_accepts_hooks_control() {
    keep_element(field_row("钩子", "钩子说明", text("control")));
}

#[test]
fn enabled_toggle_maps_both_values_to_hooks_messages() {
    for expected in [false, true] {
        match hooks_message(enabled_toggled(expected)) {
            HooksMessage::EnabledToggled(value) => assert_eq!(value, expected),
            _ => panic!("expected hooks enabled message"),
        }
    }
}

#[test]
fn command_logger_toggle_maps_both_values_to_hooks_messages() {
    for expected in [false, true] {
        match hooks_message(command_logger_toggled(expected)) {
            HooksMessage::CommandLoggerToggled(value) => assert_eq!(value, expected),
            _ => panic!("expected command logger message"),
        }
    }
}

#[test]
fn hooks_view_renders_plugin_review_and_audit_sections() {
    let empty_app = test_app();
    let empty_tree = Tree::new(view(&empty_app).as_widget());

    let mut app = test_app();
    app.app_theme = iced::Theme::Dark;
    app.hooks_settings.catalog = vec![catalog_entry(false), catalog_entry(true)];
    app.hooks_settings.audit = vec![audit_entry()];
    let populated = view(&app);
    let populated_tree = Tree::new(populated.as_widget());

    assert!(tree_node_count(&populated_tree) > tree_node_count(&empty_tree));
    keep_element(populated);
}

#[test]
fn pending_hook_actions_map_to_trust_messages() {
    let entry = catalog_entry(false);
    match hooks_message(trust_requested(&entry)) {
        HooksMessage::TrustRequested { plugin_id, hook_hash } => {
            assert_eq!(plugin_id, "reviewed-plugin");
            assert_eq!(hook_hash, "sha256:reviewed");
        }
        _ => panic!("expected hook trust request"),
    }

    let entry = catalog_entry(true);
    match hooks_message(revoke_requested(&entry)) {
        HooksMessage::RevokeRequested { plugin_id, hook_hash } => {
            assert_eq!(plugin_id, "reviewed-plugin");
            assert_eq!(hook_hash, "sha256:reviewed");
        }
        _ => panic!("expected hook revoke request"),
    }
}

#[test]
fn hooks_view_has_no_custom_placeholder() {
    assert!(!SOURCE.contains("预留"));
    assert!(SOURCE.contains("插件 hooks"));
    assert!(SOURCE.contains("审计记录"));
    assert!(SOURCE.contains("Scrollbar::new().width(4).scroller_width(4)"));
    assert!(!SOURCE.contains("Font::MONOSPACE"));
}

#[test]
fn view_builds_all_hook_toggle_combinations() {
    for (enabled, command_logger) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut app = test_app();
        app.hooks_settings.enabled = enabled;
        app.hooks_settings.command_logger = command_logger;
        keep_element(view(&app));
    }
}
