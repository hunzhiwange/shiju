//! Untrusted launch payloads only become inert catalog drafts after explicit handoff.
use crate::app::{App, Message, deep_links, message::settings::{GeneralSettingsMessage, SettingsMessage, mcp_management_import}};
use base64::{Engine as _, engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}};
use iced::{Element, Task, widget::{button, column, row, text}};
use std::collections::BTreeMap;
use vw_gateway_client::vw_api_types::mcp_management::*;

pub(crate) fn parse(raw: &str, host: &str) -> Result<Vec<Result<McpCreateRequest, String>>, &'static str> {
    let invalid = "MCP 链接格式无效";
    if raw.len() > 16 * 1024 || raw.chars().any(char::is_control) { return Err(invalid); }
    let url = reqwest::Url::parse(raw).map_err(|_| invalid)?;
    if !matches!(url.scheme(), "ccswitch" | "shiju") || url.host_str() != Some("v1")
        || url.path() != "/import" || url.fragment().is_some() || url.port().is_some()
        || !url.username().is_empty() || url.password().is_some() { return Err(invalid); }
    let mut fields = BTreeMap::new();
    for (key, value) in url.query_pairs() {
        if !matches!(key.as_ref(), "resource" | "apps" | "config" | "enabled")
            || value.chars().any(char::is_control)
            || fields.insert(key.into_owned(), value.into_owned()).is_some() { return Err(invalid); }
    }
    if fields.get("resource").map(String::as_str) != Some("mcp") { return Err(invalid); }
    let apps = fields.get("apps").ok_or(invalid)?;
    for app in apps.split(',').map(str::trim) {
        if !matches!(app, "claude" | "codex" | "gemini" | "agy" | "grokbuild" | "grok"
            | "opencode" | "openclaw" | "hermes" | "mcode" | "pi" | "kimi" | "dsh" | "omp") { return Err(invalid); }
    }
    if fields.get("enabled").is_some_and(|v| v != "true" && v != "false") { return Err(invalid); }
    let config = fields.get("config").ok_or(invalid)?;
    let bytes = STANDARD.decode(config).or_else(|_| URL_SAFE_NO_PAD.decode(config)).map_err(|_| invalid)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| invalid)?;
    let servers = value.get("mcpServers").and_then(|v| v.as_object()).filter(|s| !s.is_empty() && s.len() <= 100).ok_or(invalid)?;
    Ok(servers.iter().map(|(id, value)| {
        connection(value.clone()).map(|connection| McpCreateRequest {
            target_host_id: host.into(), project_id: None, id: id.clone(), name: None,
            description: None, homepage: None, documentation: None, tags: vec![], connection,
        }).map_err(|_| "失败：定义格式不支持，未保存".into())
    }).collect())
}

fn connection(mut value: serde_json::Value) -> Result<McpConnectionDefinition, ()> {
    let object = value.as_object_mut().ok_or(())?;
    let transport = match object.remove("type").and_then(|v| v.as_str().map(str::to_owned)).as_deref() {
        Some("stdio") => "stdio",
        Some("http" | "streamable_http") => "streamable_http",
        Some("sse") => "sse",
        None if object.contains_key("command") => "stdio",
        None if object.contains_key("url") => "streamable_http",
        _ => return Err(()),
    };
    object.insert("transport".into(), transport.into());
    // Literal values are still subject to the gateway's credential policy.
    for key in ["env", "headers"] {
        if let Some(values) = object.get_mut(key) {
            for value in values.as_object_mut().ok_or(())?.values_mut() {
                let literal = value.as_str().ok_or(())?.to_owned();
                *value = serde_json::json!({"kind":"literal", "value":literal});
            }
        }
    }
    serde_json::from_value(value).map_err(|_| ())
}

pub(crate) fn pending() -> Option<deep_links::PendingLink> {
    deep_links::pending().ok()?.into_iter().find(|link| reqwest::Url::parse(&link.url).ok()
        .is_some_and(|url| url.query_pairs().any(|(k, v)| k == "resource" && v == "mcp")))
}

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    let Some(link) = pending().filter(|l| l.id == id) else { return Task::none() };
    let drafts = match parse(&link.url, &app.apps_catalog.host) {
        Ok(drafts) => drafts,
        Err(error) => return app.show_error_toast(error),
    };
    if !mcp_management_import::open_link(app, drafts) {
        return app.show_error_toast("请先连接目标主机并关闭已有 MCP 导入弹窗");
    }
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Agents;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Mcp;
    let _ = deep_links::acknowledge(id);
    Task::none()
}

pub(crate) fn overlay<'a>(app: &App, base: Element<'a, Message>) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::*;
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(GeneralSettingsMessage::McpLinkRejected(link.id)));
    let parsed = parse(&link.url, &app.apps_catalog.host);
    let valid = parsed.is_ok();
    let body = column![text(match parsed {
        Ok(entries) => format!("MCP 定义 {} 项，内容已隐藏。继续后须再次确认保存，逐项报告成功或失败。", entries.len()),
        Err(error) => error.to_owned(),
    }), text("链接的 apps 与 enabled 不授予权限；不会执行命令、连接服务或分发配置。")].spacing(12);
    let footer = row![button("继续预览").padding([8, 10]).style(primary_action_btn_style)
        .on_press_maybe(valid.then_some(Message::Settings(SettingsMessage::General(GeneralSettingsMessage::McpLinkPreview(link.id))))),
        button("拒绝链接").padding([8, 10]).style(rounded_action_btn_style).on_press(close.clone())].spacing(8);
    settings_modal_overlay(Some(base), close.clone(), settings_modal_layout(
        settings_modal_header(text("MCP 链接预览"), settings_close_button(close)), body, Some(footer.into())
    ).style(settings_modal_card_style).width(640).height(360))
}
