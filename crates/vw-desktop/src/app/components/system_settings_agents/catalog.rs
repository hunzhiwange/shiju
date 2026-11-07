//! 两个目录共用网关配置快照；停用的自定义项仍可管理。
use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::acp_agent_icon;
use crate::app::components::system_settings_common::*;
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{AgentsSettingsState, DelegateAgentSettingsEntry};
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length, Theme};
use vw_config_types::agent::AgentRuntime;

pub(super) fn entries<'a>(
    settings: &'a AgentsSettingsState,
    official: bool,
    query: &str,
) -> Vec<&'a DelegateAgentSettingsEntry> {
    let query = query.trim().to_lowercase();
    settings
        .entries
        .iter()
        .filter(|entry| {
            (if official { entry.builtin } else { entry.enabled || !entry.builtin })
                && [&entry.key, &entry.label, &entry.description]
                    .iter()
                    .any(|value| value.to_lowercase().contains(&query))
        })
        .collect()
}

fn message(value: AgentsMessage) -> Message {
    Message::Settings(SettingsMessage::Agents(value))
}

pub(super) fn controls(app: &App) -> Element<'_, Message> {
    let settings = &app.agents_settings;
    let label = |source| tr(app.desktop_locale, source);
    let tab = |official, label| {
        super::shared::agent_tab_button(
            label,
            settings.catalog_official == official,
            message(AgentsMessage::CatalogTabSelected(official)),
        )
    };
    settings_group_card(
        container(
            column![
                row![tab(false, label("已启用智能体")), tab(true, label("官方智能体"))]
                    .spacing(8)
                    .wrap(),
                settings_control_with_hint(
                    text_input(label("搜索名称、描述或标识"), &settings.catalog_query)
                        .on_input(|value| message(AgentsMessage::CatalogQueryChanged(value)))
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_input_style),
                    label("自定义项停用后仍保留在管理区。启用与 CLI 安装状态独立。"),
                ),
            ]
            .spacing(12),
        )
        .padding([14, 0]),
    )
    .padding([0, 16])
    .into()
}

pub(super) fn cards(app: &App) -> Vec<Element<'_, Message>> {
    let settings = &app.agents_settings;
    let label = |source| tr(app.desktop_locale, source);
    let busy = settings.catalog_busy
        || settings.save_in_flight
        || settings.profile_saving
        || settings.dirty;
    entries(settings, settings.catalog_official, &settings.catalog_query)
        .into_iter()
        .map(|entry| {
            let engine = match &entry.runtime {
                AgentRuntime::Native => label("识矩原生").to_string(),
                AgentRuntime::Acp { backend } => format!("ACP · {backend}"),
            };
            let status = match &entry.runtime {
                AgentRuntime::Native => label("原生运行引擎").to_string(),
                AgentRuntime::Acp { backend } => app
                    .acp_settings
                    .diagnostic_results
                    .get(backend)
                    .map(|status| match status {
                        crate::app::state::AcpDiagnosticStatus::Connected => {
                            label("已安装 · 已连接").to_string()
                        }
                        crate::app::state::AcpDiagnosticStatus::AuthenticationRequired => {
                            label("已安装 · 需要认证").to_string()
                        }
                        _ => label(status.description()).to_string(),
                    })
                    .unwrap_or_else(|| label("网关安装 / 认证状态待检测").into()),
            };
            let action = |label, value| {
                button(text(label).size(12))
                    .padding([8, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!busy).then_some(message(value)))
            };
            let mut actions = row![
                action(label("编辑配置"), AgentsMessage::ProfileOpen(entry.key.clone())),
                action(label("复制"), AgentsMessage::CatalogCopy(entry.key.clone())),
            ]
            .spacing(6);
            if entry.can_disable {
                actions = actions.push(action(
                    label(if entry.enabled { "停用智能体" } else { "启用智能体" }),
                    AgentsMessage::CatalogToggle(entry.key.clone(), !entry.enabled),
                ));
            }
            if let AgentRuntime::Acp { backend } = &entry.runtime {
                actions = actions.push(
                    button(text(label("运行设置")).size(12))
                        .padding([8, 10])
                        .style(rounded_action_btn_style)
                        .on_press(message(AgentsMessage::CatalogRuntimeSettings(backend.clone()))),
                );
            }
            if entry.can_start_session() {
                actions = actions.push(
                    button(text(label("开始会话")).size(12))
                        .padding([8, 10])
                        .style(primary_action_btn_style)
                        .on_press_maybe(
                            (!busy && !app.new_session_creating)
                                .then(|| message(AgentsMessage::CatalogLaunch(entry.key.clone()))),
                        ),
                );
            }
            let identity_icon: Element<'_, Message> = match &entry.runtime {
                AgentRuntime::Native => icon_svg(Icon::Hexagon, 20.0)
                    .width(Length::Fixed(20.0))
                    .height(Length::Fixed(20.0))
                    .style(|theme: &Theme, _| iced::widget::svg::Style {
                        color: Some(theme.palette().text),
                    })
                    .into(),
                AgentRuntime::Acp { backend } => acp_agent_icon(backend, 20.0),
            };
            let body = column![
                button(
                    row![
                        identity_icon,
                        column![
                            text(&entry.label).size(14).wrapping(text::Wrapping::WordOrGlyph),
                            text(&entry.key)
                                .size(12)
                                .wrapping(text::Wrapping::WordOrGlyph)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(4)
                        .width(Length::Fill)
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .width(Length::Fill)
                )
                .padding([8, 10])
                .width(Length::Fill)
                .style(move |theme, status| super::shared::agent_sidebar_button_style(
                    theme,
                    status,
                    settings.selected_agent == entry.key
                ))
                .on_press(message(AgentsMessage::ProfileOpen(entry.key.clone()))),
                text(&entry.description)
                    .size(12)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .style(settings_muted_text_style),
                text(format!(
                    "{engine} · {} · {}",
                    label(super::shared::entry_kind_label(entry)),
                    label(if entry.enabled { "已启用" } else { "已停用" })
                ))
                .size(12)
                .wrapping(text::Wrapping::WordOrGlyph),
                text(status)
                    .size(12)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .style(settings_muted_text_style),
                actions.wrap(),
            ]
            .spacing(10);
            settings_panel(body).padding(12).into()
        })
        .collect()
}

/// 每行固定三列；末行补齐空位，避免卡片被拉伸。
pub(super) fn grid(app: &App) -> Element<'_, Message> {
    let cards = cards(app);
    if cards.is_empty() {
        return settings_panel(
            text(tr(app.desktop_locale, "没有匹配的智能体")).style(settings_muted_text_style),
        )
        .padding(24)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into();
    }
    let mut rows = column![].spacing(12);
    let mut cards = cards.into_iter().peekable();
    while cards.peek().is_some() {
        let mut line = row![].spacing(12);
        for _ in 0..3 {
            line = line.push(cards.next().unwrap_or_else(|| {
                container(iced::widget::Space::new()).width(Length::Fill).into()
            }));
        }
        rows = rows.push(line);
    }
    rows.width(Length::Fill).into()
}
