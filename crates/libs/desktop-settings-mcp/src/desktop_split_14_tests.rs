use crate::ConfiguredMcpServerRow;
use crate::page::{
    McpMessage as Message, configured_server_panel, new_server_panel, runtime_server_panel,
    view_overlays,
};
use crate::state::{McpNewServerTransport, State};
use serde_json::json;
use vw_api_mcp_inspect::{McpConnectionStatusDto, McpServerInspectDto};
const LOCALE: config_ui::DesktopLocale = config_ui::DesktopLocale::SimplifiedChinese;
fn help<'a>(
    _: desktop_i18n::I18n,
    _: &'a iced::Theme,
    _: &'a str,
    label: iced::widget::Text<'a>,
    _: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, Message> {
    iced::widget::row![label]
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
fn server_panels_build_configured_runtime_and_new_server_states() {
    let mut state = State::default();
    let mut inspect = inspected_server("demo", McpConnectionStatusDto::Connected);
    inspect.required = true;
    inspect.tool_count = Some(3);
    inspect.resource_count = Some(2);
    inspect.prompt_count = Some(1);
    inspect.resources = vec![json!({"name":"Resource","uri":"file:///tmp"})];
    inspect.resource_templates = vec![json!({"uriTemplate":"file:///{path}"})];
    inspect.prompts = vec![json!({"name":"Prompt","description":"Use it"})];

    let row = ConfiguredMcpServerRow {
        index: 0,
        name: "demo".to_string(),
        transport: "stdio".to_string(),
        enabled: true,
        required: true,
    };
    let _: iced::Element<'_, Message> = configured_server_panel(
        std::convert::identity,
        config_ui::DesktopLocale::SimplifiedChinese,
        row.clone(),
        Some(&inspect),
        "global:demo".to_string(),
        true,
        true,
        true,
        false,
    );
    let _: iced::Element<'_, Message> = configured_server_panel(
        std::convert::identity,
        config_ui::DesktopLocale::SimplifiedChinese,
        row,
        None,
        "global:demo".to_string(),
        false,
        false,
        false,
        true,
    );
    let _: iced::Element<'_, Message> = runtime_server_panel(
        std::convert::identity,
        config_ui::DesktopLocale::SimplifiedChinese,
        &inspect,
        "runtime:demo".to_string(),
        true,
        true,
        true,
        false,
    );

    state.new_server_expanded = false;
    let _: iced::Element<'_, Message> = view_overlays(
        std::convert::identity,
        &state,
        LOCALE,
        &iced::Theme::Dark,
        help,
        iced::widget::text("base").into(),
    );
    state.new_server_expanded = true;
    for transport in
        [McpNewServerTransport::Stdio, McpNewServerTransport::Http, McpNewServerTransport::Sse]
    {
        state.new_server.transport = transport;
        let _: iced::Element<'_, Message> =
            new_server_panel(std::convert::identity, &state, LOCALE, &iced::Theme::Dark, help);
    }
}
