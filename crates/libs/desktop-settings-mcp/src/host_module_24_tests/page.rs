static THEME: iced::Theme = iced::Theme::Dark;
use crate::page::mcp_tab_labels;
use crate::state::{McpNewServerTransport, McpSettingsTab};
use iced::widget::text_editor;
use vw_api_mcp_inspect::{McpConnectionStatusDto, McpInspectResponse, McpServerInspectDto};
fn help<'a>(
    _: desktop_i18n::I18n,
    _: &'a iced::Theme,
    _: &'a str,
    label: iced::widget::Text<'a>,
    _: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, crate::page::McpMessage> {
    iced::widget::row![label]
}
fn view(s: &crate::state::State) -> iced::Element<'_, crate::page::McpMessage> {
    crate::page::view(
        std::convert::identity,
        s,
        config_ui::DesktopLocale::SimplifiedChinese,
        &THEME,
        std::array::from_fn(|_| iced::widget::column![].into()),
        help,
    )
}
fn view_overlays<'a>(
    s: &'a crate::state::State,
    base: iced::Element<'a, crate::page::McpMessage>,
) -> iced::Element<'a, crate::page::McpMessage> {
    crate::page::view_overlays(
        std::convert::identity,
        s,
        config_ui::DesktopLocale::SimplifiedChinese,
        &THEME,
        help,
        base,
    )
}
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
    let mut app = crate::state::State::default();
    app.editor = text_editor::Content::with_text(
        r#"[{"name":"demo","type":"stdio","enabled":true,"required":true}]"#,
    );
    app.new_server_expanded = true;
    app.status_message = Some("saved".to_string());
    app.save_error = Some("save failed".to_string());
    app.inspect_error = Some("inspect failed".to_string());
    app.expanded_previews.insert("global:demo".to_string());
    app.inspect = Some(McpInspectResponse {
        servers: vec![inspected_server("demo", McpConnectionStatusDto::Connected)],
    });

    for transport in
        [McpNewServerTransport::Stdio, McpNewServerTransport::Http, McpNewServerTransport::Sse]
    {
        app.active_tab = McpSettingsTab::Servers;
        app.new_server.transport = transport;
        assert!(!view(&app).as_widget().children().is_empty());
    }

    app.active_tab = McpSettingsTab::Runtime;
    app.expanded_previews.insert("runtime:demo".to_string());
    assert!(!view(&app).as_widget().children().is_empty());

    app.active_tab = McpSettingsTab::Json;
    app.loading = true;
    app.inspect_loading = true;
    app.saving = true;
    assert!(!view(&app).as_widget().children().is_empty());
}

#[test]
fn mcp_error_banner_follows_mcp_tabs() {
    let mut app = crate::state::State::default();

    for (_, tab) in mcp_tab_labels() {
        app.active_tab = *tab;
        app.save_error = None;
        let clean_children = view(&app).as_widget().children().len();

        app.save_error = Some("mcp save failed".to_string());
        assert_eq!(view(&app).as_widget().children().len(), clean_children + 1);
    }
}

#[test]
fn delete_confirmation_uses_shared_modal_overlay_and_only_shows_on_servers_tab() {
    let mut app = crate::state::State::default();
    app.editor =
        text_editor::Content::with_text(r#"[{"name":"demo","type":"stdio","command":"npx"}]"#);
    app.delete_pending = Some(0);

    let base: iced::Element<'_, crate::page::McpMessage> = iced::widget::text("base").into();
    let overlay = view_overlays(&app, base);
    assert_eq!(overlay.as_widget().children().len(), 2);
    drop(overlay);

    app.active_tab = McpSettingsTab::Json;
    let base: iced::Element<'_, crate::page::McpMessage> = iced::widget::text("base").into();
    let unchanged = view_overlays(&app, base);
    assert!(unchanged.as_widget().children().is_empty());
}
