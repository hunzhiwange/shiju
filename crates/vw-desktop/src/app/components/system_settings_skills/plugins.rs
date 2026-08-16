//! Plugin management page for the agent settings surface.

use super::browser::{
    DetailActionStyle, catalog_panel_style, detail_action_button, empty_state, header_panel_style,
    loading_banner, search_bar_style, search_text_input_style, status_banner,
};
use super::catalog::{section_card_style, skill_badge};
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    icon_svg, settings_panel_style, settings_value_badge,
};
use crate::app::state::PluginCatalogItem;
use crate::app::{App, Message, message};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Element, Length};

pub(super) fn plugin_matches_query(plugin: &PluginCatalogItem, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    let query = query.to_ascii_lowercase();
    plugin.id.to_ascii_lowercase().contains(&query)
        || plugin.display_name.to_ascii_lowercase().contains(&query)
        || plugin.version.to_ascii_lowercase().contains(&query)
        || plugin.description.to_ascii_lowercase().contains(&query)
        || plugin
            .source_path
            .as_ref()
            .is_some_and(|path| path.to_ascii_lowercase().contains(&query))
}

pub(super) fn plugin_set_enabled_message(plugin_id: &str, enabled: bool) -> Message {
    Message::Settings(message::SettingsMessage::PluginsSetEnabledRequested {
        plugin_id: plugin_id.to_string(),
        enabled,
    })
}

pub(super) fn plugin_delete_message(plugin_id: &str) -> Message {
    Message::Settings(message::SettingsMessage::PluginsDeleteRequested(plugin_id.to_string()))
}

fn plugin_initials(plugin: &PluginCatalogItem) -> String {
    let initials = plugin
        .display_name
        .split(|ch: char| ch.is_whitespace() || ch == '-' || ch == '_')
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();

    if initials.is_empty() {
        plugin.id.chars().take(2).collect::<String>().to_ascii_uppercase()
    } else {
        initials.to_ascii_uppercase()
    }
}

pub(super) fn plugin_catalog_item(
    plugin: PluginCatalogItem,
    is_selected: bool,
) -> Element<'static, Message> {
    let initials = plugin_initials(&plugin);
    let plugin_id = plugin.id.clone();
    let display_name = plugin.display_name.clone();
    let version = plugin.version.clone();
    let description = plugin.description.clone();
    let component_text = format!("{} 个组件", plugin.component_count);
    let status = if plugin.enabled { "已启用" } else { "已禁用" };

    let details = column![
        row![
            text(display_name)
                .size(15)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::Word),
            skill_badge(format!("v{version}"), false),
            if is_selected { skill_badge("当前", true) } else { skill_badge("查看", false) },
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(description)
            .size(12)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::Word)
            .style(|theme: &iced::Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.68)),
            }),
        row![
            skill_badge(plugin_id.clone(), false),
            skill_badge(component_text, false),
            skill_badge(status, plugin.enabled),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(8)
    .width(Length::Fill);

    let content = container(
        row![
            container(text(initials).size(16))
                .width(44)
                .height(44)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .style(move |theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::container::Style {
                        text_color: Some(if plugin.enabled {
                            palette.success.base.color
                        } else {
                            theme.palette().text.scale_alpha(0.62)
                        }),
                        background: Some(Background::Color(if plugin.enabled {
                            palette.success.weak.color.scale_alpha(0.55)
                        } else {
                            palette.background.weak.color.scale_alpha(0.24)
                        })),
                        border: Border { radius: 12.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                }),
            details,
        ]
        .spacing(14)
        .align_y(Alignment::Start),
    )
    .padding([13, 15])
    .width(Length::Fill)
    .style(move |theme: &iced::Theme| {
        let mut style = settings_panel_style(theme);
        let palette = theme.extended_palette();
        style.background = Some(Background::Color(if is_selected {
            palette.primary.weak.color.scale_alpha(0.14)
        } else {
            palette.background.weak.color.scale_alpha(0.08)
        }));
        style.border.color = if is_selected {
            palette.primary.base.color.scale_alpha(0.54)
        } else {
            palette.background.strong.color.scale_alpha(0.48)
        };
        style.border.radius = 14.0.into();
        style.shadow = iced::Shadow::default();
        style
    });

    button(content)
        .width(Length::Fill)
        .padding(0)
        .on_press(Message::Settings(message::SettingsMessage::PluginsSelected(plugin.id)))
        .style(|_theme: &iced::Theme, _status| iced::widget::button::Style::default())
        .into()
}

fn plugin_path_note(path: Option<&str>) -> Element<'static, Message> {
    let text_value = path.unwrap_or("未关联本地插件目录。").to_string();
    container(
        text(text_value)
            .size(11)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::Word)
            .style(|theme: &iced::Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.6)),
            }),
    )
    .width(Length::Fill)
    .into()
}

pub(super) fn plugin_detail_panel(
    selected: Option<&PluginCatalogItem>,
    busy: bool,
) -> Element<'static, Message> {
    let Some(plugin) = selected else {
        return empty_state("未选择插件", "从左侧列表选择一个插件查看本地管理动作。");
    };

    let plugin_id = plugin.id.clone();
    let display_name = plugin.display_name.clone();
    let description = plugin.description.clone();
    let path = plugin.source_path.clone();
    let enabled = plugin.enabled;
    let component_count = plugin.component_count;
    let version = plugin.version.clone();

    let actions = row![
        detail_action_button(
            if enabled { "禁用" } else { "启用" },
            if enabled { Icon::EyeSlash } else { Icon::Eye },
            DetailActionStyle::Secondary,
            (!busy).then_some(plugin_set_enabled_message(&plugin_id, !enabled)),
        ),
        detail_action_button(
            "删除",
            Icon::Trash,
            DetailActionStyle::Danger,
            (!busy).then_some(plugin_delete_message(&plugin_id)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(
        column![
            row![
                column![
                    text(display_name).size(20),
                    text(description)
                        .size(12)
                        .width(Length::Fill)
                        .wrapping(iced::widget::text::Wrapping::Word)
                        .style(|theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.7)),
                        }),
                ]
                .spacing(4)
                .width(Length::Fill),
                icon_svg(Icon::Box, 18.0),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                settings_value_badge(format!("v{version}")),
                settings_value_badge(if enabled { "已启用" } else { "已禁用" }),
                settings_value_badge(format!("{component_count} 个组件")),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            plugin_path_note(path.as_deref()),
            actions,
        ]
        .spacing(14),
    )
    .padding([16, 18])
    .width(Length::Fill)
    .style(section_card_style)
    .into()
}

pub(in crate::app::components) fn view<'a>(app: &'a App) -> Element<'a, Message> {
    let s = &app.skills_settings;
    let plugins = s
        .plugin_catalog
        .iter()
        .filter(|plugin| plugin_matches_query(plugin, s.query.trim()))
        .cloned()
        .collect::<Vec<_>>();
    let selected = s
        .selected_plugin_id
        .as_ref()
        .and_then(|selected| s.plugin_catalog.iter().find(|plugin| plugin.id == *selected))
        .or_else(|| plugins.first());

    let search_input = text_input("搜索插件", &s.query)
        .on_input(|value| Message::Settings(message::SettingsMessage::SkillsQueryChanged(value)))
        .padding([8, 0])
        .size(13)
        .style(search_text_input_style)
        .width(Length::Fill);

    let header = container(
        column![
            row![
                column![
                    text("插件").size(22),
                    text("本地插件包、组件数量与启用状态。").size(13).style(
                        |theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.68)),
                        }
                    ),
                ]
                .spacing(6)
                .width(Length::Fill),
                detail_action_button(
                    if s.plugin_loading { "刷新中" } else { "刷新" },
                    Icon::ArrowRepeat,
                    DetailActionStyle::Secondary,
                    (!s.plugin_loading)
                        .then_some(Message::Settings(message::SettingsMessage::PluginsRefresh)),
                ),
                skill_badge(format!("{} 个插件", plugins.len()), true),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            container(
                row![icon_svg(Icon::Search, 14.0), search_input]
                    .spacing(8)
                    .align_y(Alignment::Center)
            )
            .padding([0, 4])
            .width(Length::Fill)
            .style(search_bar_style),
        ]
        .spacing(12),
    )
    .padding([18, 20])
    .width(Length::Fill)
    .style(header_panel_style);

    let mut content = column![header].spacing(16).width(Length::Fill);

    if let Some(message) = &s.plugin_status_message {
        content = content.push(status_banner(message, s.plugin_status_is_error));
    }

    if s.plugin_loading && s.plugin_catalog.is_empty() {
        content = content.push(loading_banner());
        return content.into();
    }

    if plugins.is_empty() {
        content = content.push(empty_state("暂无插件", "本地插件目录中没有可展示的插件包。"));
        return content.into();
    }

    let list = plugins.iter().cloned().fold(column![].spacing(10), |column, plugin| {
        let is_selected = selected.is_some_and(|selected| selected.id == plugin.id);
        column.push(plugin_catalog_item(plugin, is_selected))
    });

    let list_panel = container(
        scrollable(list)
            .direction(Direction::Vertical(Scrollbar::new().width(4)))
            .height(Length::Fixed(460.0)),
    )
    .padding([18, 20])
    .width(Length::FillPortion(3))
    .style(catalog_panel_style);

    let detail_panel =
        container(column![plugin_detail_panel(selected, s.plugin_loading)].spacing(12))
            .width(Length::FillPortion(2));

    content = content.push(
        row![list_panel, Space::new().width(Length::Fixed(2.0)), detail_panel]
            .spacing(14)
            .align_y(Alignment::Start),
    );

    content.into()
}

#[cfg(test)]
#[path = "plugins_tests.rs"]
mod plugins_tests;
