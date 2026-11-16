//! 验证 MCP 服务器配置、运行状态、预览区、标签与删除确认层。
//! 非法 JSON、缺失检查结果和各连接状态用于约束解析失败与显示回退。

// 宿主契约：view 注入 catalog/bindings/resync/diagnostics/tools 五个跨域面板；
// 错误横幅、删除浮层优先级及标签消息经 App::update 验证，因此留在宿主。
use super::{mcp_tab_labels, view, view_overlays};
use crate::app::Message;
use crate::app::message::settings::{McpMessage, SettingsMessage};
use crate::app::state::{McpNewServerTransport, McpSettingsTab};
use iced::widget::text_editor;
use serde_json::json;
use vw_api_mcp_inspect::{
    McpConnectionStatusDto, McpInspectResponse, McpServerInspectDto,
};

// MCP fixture 只填充展示与状态匹配所需字段，连接行为不在 UI 单测中模拟。
fn inspected_server(name: &str, status: McpConnectionStatusDto) -> McpServerInspectDto {
    McpServerInspectDto {
        name: name.to_string(),
        transport: "stdio".to_string(),
        enabled: status != McpConnectionStatusDto::Disabled,
        required: false,
        connection_status: status,
        timeout_ms: Some(30_000),
        allowed_tools: Vec::new(),
        denied_tools: Vec::new(),
        tool_count: Some(0),
        resource_count: Some(0),
        resource_template_count: Some(0),
        prompt_count: Some(0),
        tools: Vec::new(),
        resources: Vec::new(),
        resource_templates: Vec::new(),
        prompts: Vec::new(),
        tools_error: None,
        resources_error: None,
        resource_templates_error: None,
        prompts_error: None,
        error: None,
    }
}

#[test]
fn view_builds_servers_runtime_and_json_tabs_with_state_variants() {
    let (mut app, _) = crate::app::App::new();
    app.mcp_settings.editor = text_editor::Content::with_text(
        r#"[{"name":"demo","type":"stdio","enabled":true,"required":true}]"#,
    );
    app.mcp_settings.new_server_expanded = true;
    app.mcp_settings.status_message = Some("saved".to_string());
    app.mcp_settings.save_error = Some("save failed".to_string());
    app.mcp_settings.inspect_error = Some("inspect failed".to_string());
    app.mcp_settings.expanded_previews.insert("global:demo".to_string());
    app.mcp_settings.inspect = Some(McpInspectResponse {
        servers: vec![inspected_server("demo", McpConnectionStatusDto::Connected)],
    });

    for transport in
        [McpNewServerTransport::Stdio, McpNewServerTransport::Http, McpNewServerTransport::Sse]
    {
        app.mcp_settings.active_tab = McpSettingsTab::Servers;
        app.mcp_settings.new_server.transport = transport;
        assert!(!view(&app).as_widget().children().is_empty());
    }

    app.mcp_settings.active_tab = McpSettingsTab::Runtime;
    app.mcp_settings.expanded_previews.insert("runtime:demo".to_string());
    assert!(!view(&app).as_widget().children().is_empty());

    app.mcp_settings.active_tab = McpSettingsTab::Json;
    app.mcp_settings.loading = true;
    app.mcp_settings.inspect_loading = true;
    app.mcp_settings.saving = true;
    assert!(!view(&app).as_widget().children().is_empty());
}

#[test]
fn mcp_error_banner_follows_mcp_tabs() {
    let (mut app, _) = crate::app::App::new();

    for (_, tab) in mcp_tab_labels() {
        app.mcp_settings.active_tab = *tab;
        app.mcp_settings.save_error = None;
        let clean_children = view(&app).as_widget().children().len();

        app.mcp_settings.save_error = Some("mcp save failed".to_string());
        assert_eq!(view(&app).as_widget().children().len(), clean_children + 1);
    }
}

#[test]
fn delete_confirmation_uses_shared_modal_overlay_and_only_shows_on_servers_tab() {
    let (mut app, _) = crate::app::App::new();
    app.mcp_settings.editor =
        text_editor::Content::with_text(r#"[{"name":"demo","type":"stdio","command":"npx"}]"#);
    app.mcp_settings.delete_pending = Some(0);

    let base: iced::Element<'_, Message> = iced::widget::text("base").into();
    let overlay = view_overlays(&app, base);
    assert_eq!(overlay.as_widget().children().len(), 2);
    drop(overlay);

    app.mcp_settings.active_tab = McpSettingsTab::Json;
    let base: iced::Element<'_, Message> = iced::widget::text("base").into();
    let unchanged = view_overlays(&app, base);
    assert!(unchanged.as_widget().children().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_capabilities_appearance_mcp() {
    use crate::app::App;
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::settings_extensions::text as tr;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_capabilities_appearance_mcp"),
        || App::new().0,
        |scene, app| {
            app.mcp_settings.active_tab = McpSettingsTab::Servers;
            app.mcp_settings.save_error = Some("fixture MCP error".into());
            let locale = app.desktop_locale;
            // Match the agents page's real viewport, scrolled to the legacy controls.
            let page = iced::widget::scrollable(iced::widget::container(view(app)))
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
                .anchor_bottom()
                .height(iced::Length::Fill);
            let tab = scene.check(
                app,
                page.into(),
                &["全局服务器", "运行时 MCP", "高级 JSON"].map(|label| tr(locale, label)),
                2,
            );
            assert!(matches!(
                tab,
                SettingsMessage::Mcp(McpMessage::TabSelected(McpSettingsTab::Json))
            ));
            let _ = app.update(Message::Settings(tab));
            assert_eq!(app.mcp_settings.active_tab, McpSettingsTab::Json);
            let labels = scene.labels(view(app));
            assert!(!labels.contains(tr(locale, "当前没有全局 MCP 服务器。")));
            assert!(labels.contains("fixture MCP error"));
        },
    )
    .await;
}
