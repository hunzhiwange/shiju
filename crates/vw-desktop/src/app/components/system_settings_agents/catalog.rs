//! 两个目录共用网关配置快照；停用的自定义项仍可管理。
use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::acp_agent_icon;
use crate::app::components::system_settings_common::*;
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{AgentsSettingsState, DelegateAgentSettingsEntry};
use crate::app::{App, Message};
use iced::widget::{Grid, button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};
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
                settings_tab_row([
                    tab(false, label("已启用智能体")).into(),
                    tab(true, label("官方智能体")).into(),
                ])
                .wrap(),
                settings_control_with_hint(
                    row![
                        icon_svg(Icon::Search, 16.0).style(|theme: &Theme, _| {
                            iced::widget::svg::Style {
                                color: Some(theme.palette().text.scale_alpha(0.72)),
                            }
                        }),
                        settings_text_input(label("搜索名称、描述或标识"), &settings.catalog_query)
                            .on_input(|value| message(AgentsMessage::CatalogQueryChanged(value)))
                            .width(Length::Fill),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
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

fn cards(app: &App) -> Vec<Element<'_, Message>> {
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
                button(text(label).size(SETTINGS_CONTROL_TEXT_SIZE))
                    .padding([8, 11])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!busy).then_some(message(value)))
            };
            let mut actions = row![
                action(label("编辑配置"), AgentsMessage::ProfileOpen(entry.key.clone())),
                action(label("复制"), AgentsMessage::CatalogCopy(entry.key.clone())),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            if entry.can_disable {
                actions = actions.push(action(
                    label(if entry.enabled { "停用智能体" } else { "启用智能体" }),
                    AgentsMessage::CatalogToggle(entry.key.clone(), !entry.enabled),
                ));
            }
            if let AgentRuntime::Acp { backend } = &entry.runtime {
                actions = actions.push(
                    button(text(label("运行设置")).size(SETTINGS_CONTROL_TEXT_SIZE))
                        .padding([8, 11])
                        .style(rounded_action_btn_style)
                        .on_press(message(AgentsMessage::CatalogRuntimeSettings(backend.clone()))),
                );
            }
            if entry.can_start_session() {
                actions = actions.push(
                    button(text(label("开始会话")).size(SETTINGS_CONTROL_TEXT_SIZE))
                        .padding([8, 11])
                        .style(primary_action_btn_style)
                        .on_press_maybe(
                            (!busy && !app.new_session_creating)
                                .then(|| message(AgentsMessage::CatalogLaunch(entry.key.clone()))),
                        ),
                );
            }
            let identity_icon: Element<'_, Message> = match &entry.runtime {
                AgentRuntime::Native => icon_svg(Icon::Hexagon, 26.0)
                    .style(|theme: &Theme, _| iced::widget::svg::Style {
                        color: Some(theme.palette().text),
                    })
                    .into(),
                AgentRuntime::Acp { backend } => acp_agent_icon(backend, 26.0),
            };
            let header = button(
                row![
                    container(identity_icon)
                        .center_x(44.0)
                        .center_y(44.0)
                        .style(settings_icon_tile_style),
                    column![
                        text(&entry.label).size(16).wrapping(text::Wrapping::WordOrGlyph),
                        text(&entry.key)
                            .size(12)
                            .wrapping(text::Wrapping::WordOrGlyph)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(4)
                    .width(Length::Fill)
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding(6)
            .width(Length::Fill)
            .style(move |theme, status| {
                super::shared::agent_sidebar_button_style(
                    theme,
                    status,
                    settings.selected_agent == entry.key,
                )
            })
            .on_press(message(AgentsMessage::ProfileOpen(entry.key.clone())));
            let metadata = row![
                settings_value_badge(label(if entry.enabled { "已启用" } else { "已停用" })),
                text(format!("{engine} · {}", label(super::shared::entry_kind_label(entry))))
                    .size(12)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap();
            let mut body = column![header, metadata].spacing(10);
            if !entry.description.trim().is_empty() {
                body = body
                    .push(text(&entry.description).size(13).wrapping(text::Wrapping::WordOrGlyph));
            }
            body = body.push(
                text(status)
                    .size(12)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .style(settings_muted_text_style),
            );
            container(body.push(settings_divider()).push(actions.wrap()))
                .padding([14, 16])
                .width(Length::Fill)
                .style(settings_panel_style)
                .into()
        })
        .collect()
}

/// 每行固定两列；末行只有一张卡片时仍保持列宽。
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
    Grid::from_vec(cards).columns(2).height(Length::Shrink).spacing(12).into()
}
