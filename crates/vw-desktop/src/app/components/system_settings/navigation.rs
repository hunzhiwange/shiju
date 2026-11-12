//! 分组设置菜单、搜索框和结果导航。

use super::{SETTINGS_SCROLL_GUTTER, SystemTab, is_network_tab};
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_btn, icon_svg,
    settings_muted_text_style, settings_text_input_style,
};
use crate::app::components::system_settings_search::{
    SystemSettingsSearchEntry, SystemSettingsSearchTarget, navigation_label, search_results,
};
use crate::app::i18n::I18n;
use crate::app::message::{SettingsMessage, ViewMessage};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, column, container, row, scrollable, stack, svg, text, text_input,
};
use iced::{Alignment, Border, Element, Font, Length, Padding, Theme, font};

pub(super) const GROUPS: &[(&str, &[SystemTab])] = {
    use SystemTab::*;
    &[
        (
            "应用与项目",
            &[
                General,
                Apps,
                #[cfg(not(target_arch = "wasm32"))]
                Auth,
                #[cfg(not(target_arch = "wasm32"))]
                Voice,
                Projects,
                Worktrees,
                ArchivedChats,
            ],
        ),
        ("模型与智能体", &[Providers, Models, Agents, Memory]),
        ("网络与集成", &[GatewayClient, Gateway, WebSearch]),
        ("运行与安全", &[Cron, Security, Autonomy, Hooks, Observability]),
    ]
};

pub(crate) const SETTINGS_NAVIGATION_SCROLL: &str = "system-settings-navigation-scroll";

pub(super) fn view(app: &App, width: f32) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let title = row![
        icon_btn(
            Icon::ArrowLeft,
            i18n.__("返回"),
            Some(Message::View(ViewMessage::ToggleSystemSettings)),
        ),
        text(i18n.__("设置"))
            .size(18)
            .font(Font { weight: font::Weight::Semibold, ..Font::DEFAULT }),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let entries = if app.system_settings_query.trim().is_empty() {
        column(GROUPS.iter().map(|(label, tabs)| {
            let items = tabs.iter().map(|&tab| {
                navigation_entry(
                    app,
                    SystemSettingsSearchEntry::new(
                        navigation_label(app.desktop_locale, tab),
                        None,
                        &[],
                        SystemSettingsSearchTarget::TopLevel(tab),
                    ),
                )
            });
            column![
                container(text(i18n.__(label)).size(12).style(settings_muted_text_style))
                    .padding([3, 8]),
                column(items).spacing(1),
            ]
            .spacing(3)
            .into()
        }))
        .spacing(12)
    } else {
        let results = search_results(app, &app.system_settings_query);
        if results.is_empty() {
            column![
                container(
                    text(i18n.__("没有匹配的配置")).size(12).style(settings_muted_text_style)
                )
                .padding([8, 8])
            ]
        } else {
            column(results.into_iter().map(|entry| navigation_entry(app, entry))).spacing(2)
        }
    };

    let entries =
        scrollable(container(entries).padding([0, SETTINGS_SCROLL_GUTTER]).width(Length::Fill))
            .id(SETTINGS_NAVIGATION_SCROLL)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill);
    container(
        column![
            container(title).padding([0, SETTINGS_SCROLL_GUTTER]),
            container(search_box(app, i18n)).padding([0, SETTINGS_SCROLL_GUTTER]),
            entries,
        ]
        .spacing(10),
    )
    .width(width)
    .height(Length::Fill)
    .padding([10, 0])
    .style(|theme: &Theme| {
        let palette = theme.palette();
        let tint = if theme.extended_palette().is_dark { 0.045 } else { 0.028 };
        container::Style {
            background: Some(palette.text.scale_alpha(tint).into()),
            ..Default::default()
        }
    })
    .into()
}

fn search_box(app: &App, i18n: I18n) -> Element<'_, Message> {
    let query = &app.system_settings_query;
    let [vertical, horizontal] = SETTINGS_CONTROL_PADDING;
    let input = text_input(i18n.__("搜索"), query)
        .on_input(|value| Message::Settings(SettingsMessage::SystemTabQueryChanged(value)))
        .padding(Padding {
            top: vertical,
            bottom: vertical,
            left: horizontal + 16.0,
            right: horizontal + if query.is_empty() { 0.0 } else { 17.0 },
        })
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(|theme: &Theme, status| {
            let mut style = settings_text_input_style(theme, status);
            style.border.radius = 999.0.into();
            if !matches!(status, text_input::Status::Focused { .. }) {
                let palette = theme.palette();
                style.background = palette.text.scale_alpha(0.055).into();
                style.border.width = 0.0;
            }
            style
        });
    let icon = container(icon_svg(Icon::Search, 14.0).style(|theme: &Theme, _| svg::Style {
        color: Some(theme.palette().text.scale_alpha(0.64)),
    }))
    .padding([0, 10]);
    let mut adornments = row![icon, Space::new().width(Length::Fill)]
        .height(Length::Fill)
        .align_y(Alignment::Center);
    if !query.is_empty() {
        adornments = adornments.push(
            container(icon_btn(
                Icon::X,
                i18n.__("清除搜索"),
                Some(Message::Settings(SettingsMessage::SystemTabQueryChanged(String::new()))),
            ))
            .padding([0, 4]),
        );
    }
    stack![input, adornments].into()
}

fn navigation_entry(app: &App, entry: SystemSettingsSearchEntry) -> Element<'static, Message> {
    let active = search_target_is_active(app, entry.target);
    let mut label =
        column![text(entry.label).size(SETTINGS_CONTROL_TEXT_SIZE).wrapping(Wrapping::WordOrGlyph)]
            .spacing(2)
            .width(Length::Fill);
    if let Some(parent) = entry.parent_path() {
        label = label.push(
            text(parent).size(11).wrapping(Wrapping::WordOrGlyph).style(settings_muted_text_style),
        );
    }

    let icon = icon_svg(tab_icon(entry.target.page()), 15.0)
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) });
    let title = row![icon, label].spacing(8).align_y(if entry.parent_label.is_some() {
        Alignment::Start
    } else {
        Alignment::Center
    });
    button(title)
        .width(Length::Fill)
        .padding([6, 8])
        .clip(true)
        .on_press(Message::Settings(SettingsMessage::SystemSearchTargetSelected(entry.target)))
        .style(move |theme: &Theme, status| {
            let palette = theme.palette();
            let tint = match status {
                button::Status::Pressed => 0.15,
                button::Status::Hovered => 0.10,
                _ if active => 0.085,
                _ => 0.0,
            };
            button::Style {
                background: (tint > 0.0).then(|| palette.text.scale_alpha(tint).into()),
                text_color: palette.text,
                border: Border { radius: 8.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .into()
}

fn tab_icon(tab: SystemTab) -> Icon {
    match tab {
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Voice => Icon::Sliders,
        SystemTab::General => Icon::Gear,
        SystemTab::Themes => Icon::Palette,
        SystemTab::Editor => Icon::Code,
        SystemTab::Apps => Icon::Grid1x2,
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Auth => Icon::ShieldLock,
        SystemTab::Projects | SystemTab::Worktrees => Icon::FolderOpen,
        SystemTab::ArchivedChats => Icon::ChatTextFill,
        SystemTab::Providers => Icon::Box,
        SystemTab::Models => Icon::Hexagon,
        SystemTab::Agents => Icon::Robot,
        SystemTab::Channels => Icon::ChatTextFill,
        SystemTab::Cron => Icon::Clock,
        SystemTab::Memory => Icon::Journals,
        SystemTab::GatewayClient => Icon::Link,
        SystemTab::Gateway => Icon::HddNetwork,
        SystemTab::WebSearch | SystemTab::HttpRequest | SystemTab::Browser => Icon::Globe,
        SystemTab::Proxy | SystemTab::Tunnel => Icon::Link45Deg,
        SystemTab::Runtime => Icon::Terminal,
        SystemTab::Security => Icon::ShieldLock,
        SystemTab::Autonomy => Icon::Sliders,
        SystemTab::Hooks => Icon::GitBranch,
        SystemTab::Observability => Icon::Speedometer2,
    }
}

fn search_target_is_active(app: &App, target: SystemSettingsSearchTarget) -> bool {
    match target {
        SystemSettingsSearchTarget::GeneralField(_) => false,
        SystemSettingsSearchTarget::TopLevel(tab) => {
            tab == app.system_settings_tab
                || (tab == SystemTab::WebSearch && is_network_tab(app.system_settings_tab))
        }
        SystemSettingsSearchTarget::General(tab) => {
            app.system_settings_tab == SystemTab::General && app.general_settings_tab == tab
        }
        SystemSettingsSearchTarget::Models(tab) => {
            app.system_settings_tab == SystemTab::Models && app.model_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Cron(tab) => {
            app.system_settings_tab == SystemTab::Cron && app.cron_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Agents(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab == tab
        }
        SystemSettingsSearchTarget::Channels(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab
                    == crate::app::state::AgentConfigTab::Channels
                && app.channels_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Memory(tab) => {
            app.system_settings_tab == SystemTab::Memory && app.memory_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Autonomy(tab) => {
            app.system_settings_tab == SystemTab::Autonomy
                && app.autonomy_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Security(tab) => {
            app.system_settings_tab == SystemTab::Security
                && app.security_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Gateway(tab) => {
            app.system_settings_tab == SystemTab::Gateway && app.gateway_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Network(tab) => app.system_settings_tab == tab,
        SystemSettingsSearchTarget::AgentMcp(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab == crate::app::state::AgentConfigTab::Mcp
                && app.mcp_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::AgentSkills(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab
                    == crate::app::state::AgentConfigTab::Skills
                && app.skills_settings.active_tab == tab
        }
    }
}
