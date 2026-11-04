//! 智能体的新会话 MCP 选择；连接与凭据仍由外层 MCP 页面管理。
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::{
    AgentConfigTabMessage, AgentsMessage, McpMessage, SettingsMessage,
};
use crate::app::state::{AgentConfigTab, DelegateAgentSettingsEntry};
use crate::app::{App, Message};
use iced::{
    Element,
    widget::{button, checkbox, column, pick_list, row, text},
};
use vw_config_types::agent::{AgentDefault, AgentDefaults};

pub(super) fn toggle_message(
    defaults: &AgentDefaults,
    id: &str,
    checked: bool,
    available: bool,
) -> Message {
    let mut next = defaults.clone();
    if let AgentDefault::Fixed(ids) = &mut next.mcp {
        ids.retain(|value| value != id);
        if checked && available {
            ids.push(id.to_owned());
        }
    }
    Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileDefaultsChanged(next)))
}

pub(super) fn view<'a>(
    app: &'a App,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let settings = &app.mcp_settings;
    let ready =
        settings.inspect.is_some() && !settings.inspect_loading && settings.inspect_error.is_none();
    let servers =
        settings.inspect.as_ref().map(|inspect| inspect.servers.as_slice()).unwrap_or_default();
    let available = servers
        .iter()
        .filter(|server| server.enabled)
        .map(|server| server.name.clone())
        .collect::<Vec<_>>();
    let fixed = matches!(draft.defaults.mcp, AgentDefault::Fixed(_));
    let selected = match &draft.defaults.mcp {
        AgentDefault::Fixed(ids) => ids.clone(),
        AgentDefault::Inherit => {
            if ready {
                available.clone()
            } else {
                vec![]
            }
        }
    };
    let original = draft.defaults.clone();
    let mode = pick_list(
        vec!["继承", "固定选择"],
        Some(if fixed { "固定选择" } else { "继承" }),
        move |mode| {
            let mut next = original.clone();
            next.mcp = if mode == "固定选择" {
                AgentDefault::Fixed(vec![])
            } else {
                AgentDefault::Inherit
            };
            Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileDefaultsChanged(next)))
        },
    )
    .padding(SETTINGS_CONTROL_PADDING)
    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style);
    let mut body = column![
        text("默认 MCP").size(16),
        text("继承当前会话工作区启用的服务；固定空列表不启用默认 MCP。这里只选择服务，不配置连接或凭据。")
            .size(12).style(settings_muted_text_style),
        mode,
        text(format!("已选 {} 个服务{}", selected.len(), if fixed { "" } else { "（继承预览，以会话工作区为准）" })).size(12),
        row![
            button(text("刷新服务目录")).padding([8, 14]).style(rounded_action_btn_style)
                .on_press(Message::Settings(SettingsMessage::Mcp(McpMessage::InspectRefresh))),
            button(text("前往 MCP 管理")).padding([8, 14]).style(rounded_action_btn_style)
                .on_press(Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Mcp)))),
        ].spacing(8),
    ].spacing(12);
    if settings.inspect_loading {
        body = body.push(text("正在加载服务目录；保留已有选择。").style(settings_muted_text_style));
    } else if let Some(error) = &settings.inspect_error {
        body = body
            .push(settings_error_banner(format!("服务目录加载失败：{error}。已有选择保持不变。")));
    } else if settings.inspect.is_none() {
        body = body.push(text("服务目录尚未加载，请刷新。").style(settings_muted_text_style));
    }
    let mut ids = servers.iter().map(|server| server.name.clone()).collect::<Vec<_>>();
    for id in &selected {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    for id in ids {
        let enabled = ready && available.contains(&id);
        let checked = selected.contains(&id);
        let status = if !ready {
            "待验证"
        } else if available.contains(&id) {
            "已启用"
        } else {
            "不可用"
        };
        let label = format!("{id}（{status}）");
        let defaults = draft.defaults.clone();
        body = body.push(
            checkbox(checked)
                .label(label)
                .on_toggle_maybe(
                    (fixed && (checked || enabled))
                        .then(|| move |checked| toggle_message(&defaults, &id, checked, enabled)),
                )
                .style(settings_checkbox_style),
        );
    }
    if let Some(error) = super::defaults::validation_error(app, draft) {
        body = body.push(settings_error_banner(error));
    }
    body.into()
}
