use crate::help::{settings_group_heading_with_help};
use crate::message::{Event, AgentsMessage};
use crate::state::*;
// 智能体的新会话 MCP 选择；连接与凭据仍由外层 MCP 页面管理。
use desktop_ui_controls::system_settings_common::*;
use desktop_i18n::{I18n, settings_extensions::text as tr};
use iced::{
    Alignment, Element,
    widget::{button, checkbox, column, row, text},
};
use config_agent_profile::{AgentDefault, AgentDefaults};

pub fn toggle_message(
    defaults: &AgentDefaults,
    id: &str,
    checked: bool,
    available: bool,
) -> Event {
    let mut next = defaults.clone();
    if let AgentDefault::Fixed(ids) = &mut next.mcp {
        ids.retain(|value| value != id);
        if checked && available {
            ids.push(id.to_owned());
        }
    }
    Event::Agents(AgentsMessage::ProfileDefaultsChanged(next))
}

pub fn view<'a>(
    input: Input<'a>,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    let settings = &input;
    let ready =
        settings.inspect.is_some() && !settings.inspect_loading && settings.inspect_error.is_none();
    let servers =
        settings.inspect.map(|inspect| inspect.servers.as_slice()).unwrap_or_default();
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
    let locale = input.locale;
    let label = |source| tr(locale, source);
    let original = draft.defaults.clone();
    let mode = super::defaults::selection_mode(locale, fixed, move |fixed| {
        let mut next = original.clone();
        next.mcp = if fixed { AgentDefault::Fixed(vec![]) } else { AgentDefault::Inherit };
        Event::Agents(AgentsMessage::ProfileDefaultsChanged(next))
    });
    let i18n = I18n::new(locale);
    let count = if fixed {
        desktop_i18n::__!(i18n, "已选 %u 个服务", selected.len())
    } else {
        desktop_i18n::__!(i18n, "已选 %u 个服务（继承预览，以会话工作区为准）", selected.len())
    }
    .expect("valid MCP count template");
    let mut body = column![
        mode,
        row![
            settings_value_badge(count),
            button(text(label("刷新服务目录")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(Event::RefreshMcp),
            button(text(label("前往 MCP 管理")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(Event::OpenMcp),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(12);
    if settings.inspect_loading {
        body = body.push(
            text(label("正在加载服务目录；保留已有选择。"))
                .size(12)
                .style(settings_muted_text_style),
        );
    } else if let Some(error) = &settings.inspect_error {
        body = body.push(settings_error_banner(
            desktop_i18n::__!(i18n, "服务目录加载失败：%s。已有选择保持不变。", error.as_str())
                .expect("valid MCP directory error template"),
        ));
    } else if settings.inspect.is_none() {
        body = body.push(
            text(label("服务目录尚未加载，请刷新。")).size(12).style(settings_muted_text_style),
        );
    }
    let mut ids = servers.iter().map(|server| server.name.clone()).collect::<Vec<_>>();
    for id in &selected {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    if !ids.is_empty() {
        body = body.push(settings_divider());
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
        let server_label = format!("{id}（{}）", label(status));
        let defaults = draft.defaults.clone();
        body = body.push(
            checkbox(checked)
                .label(server_label)
                .on_toggle_maybe(
                    (fixed && (checked || enabled))
                        .then(|| move |checked| toggle_message(&defaults, &id, checked, enabled)),
                )
                .style(settings_checkbox_style),
        );
    }
    column![
        settings_group_heading_with_help(input.help,
            i18n,
            input.theme,
            label("默认 MCP"),
            label("继承当前会话工作区启用的服务；固定空列表不启用默认 MCP。这里只选择服务，不配置连接或凭据。"),
        ),
        settings_group_card(body).padding([14, 16]),
        super::defaults::validation_error(&input.defaults, draft).map(settings_error_banner),
    ]
    .spacing(8)
    .into()
}

pub struct Input<'a> {
    pub help: crate::help::Renderer,
    pub locale: config_ui::DesktopLocale,
    pub theme: &'a iced::Theme,
    pub inspect: Option<&'a vw_api_mcp_inspect::McpInspectResponse>,
    pub inspect_loading: bool,
    pub inspect_error: Option<&'a String>,
    pub defaults: super::defaults::Input<'a>,
}
