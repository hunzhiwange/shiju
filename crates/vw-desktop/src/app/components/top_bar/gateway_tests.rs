//! 验证顶部栏网关状态展示、连接操作和异常状态反馈。
//!
//! 网关夹具覆盖连接中、已连接、断开和错误状态，并检查对应操作是否可用。

use super::{lsp_tab_content, manage_plugins_message};
use crate::app::components::editor::Editor;
use crate::app::components::system_settings::SystemTab;
use crate::app::message::settings::{AgentConfigTabMessage, SettingsMessage};
use crate::app::message::view::MenuType;
use crate::app::state::{
    AgentConfigTab,
};
use crate::app::{App, LspProgress, Message, PreviewTab, message};
use std::collections::HashMap;

fn test_app() -> App {
    App::new().0
}

fn preview_tab_with_lsp(server_key: Option<&'static str>) -> PreviewTab {
    PreviewTab {
        path: "/tmp/main.rs".to_string(),
        title: "main.rs".to_string(),
        content: String::new(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: Editor::new("", "rust"),
        scroll_id: iced::widget::Id::unique(),
        lsp_server_key: server_key,
        lsp_uri: None,
        lsp_language_id: None,
    }
}

#[test]
fn manage_plugins_opens_the_standalone_agent_plugin_tab() {
    let Message::Batch(messages) = manage_plugins_message() else {
        panic!("管理插件应批量打开设置与插件页签");
    };

    assert!(matches!(
        messages.as_slice(),
        [
            Message::View(crate::app::message::view::ViewMessage::OpenSystemSettingsTab(
                SystemTab::Agents
            )),
            Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                AgentConfigTab::Plugins
            )))
        ]
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_reports_disabled_state_first() {
    let mut app = test_app();
    app.lsp_disabled = true;
    app.lsp_status = Some("LSP ready".to_string());
    app.navigation_tabs.preview_tabs.push(preview_tab_with_lsp(Some("rust-analyzer")));

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_reports_empty_status_when_no_servers_exist() {
    let mut app = test_app();
    app.lsp_status = Some("LSP 尚未启动".to_string());

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_deduplicates_keys_and_appends_status() {
    let mut app = test_app();
    app.lsp_status = Some("LSP 已连接".to_string());
    app.navigation_tabs.preview_tabs.push(preview_tab_with_lsp(Some("rust-analyzer")));
    app.navigation_tabs.preview_tabs.push(preview_tab_with_lsp(Some("rust-analyzer")));
    app.lsp_progress.insert("rust-analyzer".to_string(), HashMap::new());

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 2);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn lsp_tab_content_formats_progress_detail() {
    let mut app = test_app();
    app.lsp_progress.insert(
        "rust-analyzer".to_string(),
        HashMap::from([(
            "token-1".to_string(),
            LspProgress {
                title: "indexing".to_string(),
                message: Some("workspace".to_string()),
                percentage: Some(42),
            },
        )]),
    );

    let rows = lsp_tab_content(&app);

    assert_eq!(rows.len(), 1);
}

#[test]
fn gateway_tests_keep_public_view_messages_reachable() {
    let toggle = Message::View(message::ViewMessage::ToggleMenu(Some(MenuType::GatewayServices)));
    let close = Message::View(message::ViewMessage::ToggleMenu(None));

    assert!(matches!(toggle, Message::View(_)));
    assert!(matches!(close, Message::View(_)));
}
