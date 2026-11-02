//! 构建代理设置中的插件检索、详情与管理页面。

use super::browser::{
    DetailActionStyle, catalog_panel_style, empty_state, header_panel_style, loading_banner,
    search_bar_style, search_text_input_style, status_banner,
};
use super::catalog::{section_card_style, skill_badge};
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, icon_svg as base_icon_svg, primary_action_btn_style,
    rounded_action_btn_style, settings_panel_style, settings_value_badge,
};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::state::PluginCatalogItem;
use crate::app::{App, Message, message};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Element, Length};
use vw_config_types::ui::DesktopLocale;

/// 在插件标识、名称、版本、说明和来源路径中执行不区分大小写的匹配。
pub(super) fn plugin_matches_query(
    locale: DesktopLocale,
    plugin: &PluginCatalogItem,
    query: &str,
) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    let query = query.to_ascii_lowercase();
    plugin.id.to_ascii_lowercase().contains(&query)
        || plugin.display_name.to_ascii_lowercase().contains(&query)
        || plugin.version.to_ascii_lowercase().contains(&query)
        || plugin.description.to_ascii_lowercase().contains(&query)
        || source_label(locale, plugin).to_ascii_lowercase().contains(&query)
        || source_label(DesktopLocale::SimplifiedChinese, plugin)
            .to_ascii_lowercase()
            .contains(&query)
        || plugin
            .source_path
            .as_ref()
            .is_some_and(|path| path.to_ascii_lowercase().contains(&query))
}

/// 构造插件启停请求消息，不直接修改目录状态。
pub(super) fn plugin_set_enabled_message(plugin_id: &str, enabled: bool) -> Message {
    Message::Settings(message::SettingsMessage::PluginsSetEnabledRequested {
        plugin_id: plugin_id.to_string(),
        enabled,
    })
}

/// 构造插件删除请求消息，由设置更新层执行确认与持久化。
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

/// 构建插件目录卡片，并突出当前选中项和启用状态。
pub(super) fn plugin_catalog_item(
    locale: DesktopLocale,
    plugin: PluginCatalogItem,
    is_selected: bool,
) -> Element<'static, Message> {
    let initials = plugin_initials(&plugin);
    let plugin_id = plugin.id.clone();
    let display_name = plugin.display_name.clone();
    let version = plugin.version.clone();
    let description = plugin.description.clone();
    let component_text =
        crate::__!(crate::app::i18n::I18n::new(locale), "%u 个组件", plugin.component_count)
            .expect("valid skills display template");
    let compatibility = tr(locale, super::plugin_dependency_summary(&plugin.dependency_checks));
    let status = if plugin.enabled { tr(locale, "已启用") } else { tr(locale, "已禁用") };

    let details = column![
        row![
            text(display_name)
                .size(15)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::Word),
            skill_badge(format!("v{version}"), false),
            if is_selected {
                skill_badge(tr(locale, "当前"), true)
            } else {
                skill_badge(tr(locale, "查看"), false)
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(compatibility).size(12),
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
                        text_color: Some(theme.palette().text),
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
        .style(|theme: &iced::Theme, _status| iced::widget::button::Style {
            text_color: theme.palette().text,
            ..Default::default()
        })
        .into()
}

fn plugin_path_note(locale: DesktopLocale, path: Option<&str>) -> Element<'static, Message> {
    let text_value = path.unwrap_or(tr(locale, "未关联本地插件目录。")).to_string();
    container(
        text(text_value)
            .size(11)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::Glyph)
            .style(|theme: &iced::Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.6)),
            }),
    )
    .width(Length::Fill)
    .into()
}

/// 构建所选插件的详情及启停、删除操作区域。
pub(super) fn plugin_detail_panel(
    locale: DesktopLocale,
    selected: Option<&PluginCatalogItem>,
    busy: bool,
) -> Element<'static, Message> {
    let Some(plugin) = selected else {
        return empty_state(
            tr(locale, "未选择插件"),
            tr(locale, "从左侧列表选择一个插件查看本地管理动作。"),
        );
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
            locale,
            tr(locale, "更新"),
            Icon::ArrowRepeat,
            DetailActionStyle::Secondary,
            (!busy).then_some(Message::Settings(message::SettingsMessage::PluginsUpdateOpen(
                plugin_id.clone()
            )))
        ),
        detail_action_button(
            locale,
            if enabled { tr(locale, "禁用") } else { tr(locale, "启用") },
            if enabled { Icon::EyeSlash } else { Icon::Eye },
            DetailActionStyle::Secondary,
            (!busy).then_some(plugin_set_enabled_message(&plugin_id, !enabled)),
        ),
        detail_action_button(
            locale,
            tr(locale, "删除"),
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
                settings_value_badge(if enabled {
                    tr(locale, "已启用")
                } else {
                    tr(locale, "已禁用")
                }),
                settings_value_badge(
                    crate::__!(crate::app::i18n::I18n::new(locale), "%u 个组件", component_count)
                        .expect("valid skills display template")
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(source_label(locale, plugin)).size(12),
            text(tr(locale, super::plugin_dependency_summary(&plugin.dependency_checks))).size(13),
            text(tr(locale, hook_label(&plugin.hook_review))).size(12),
            plugin_path_note(locale, path.as_deref()),
            dependency_panel(locale, &plugin.dependency_checks),
            actions,
        ]
        .spacing(14),
    )
    .padding([16, 18])
    .width(Length::Fill)
    .style(section_card_style)
    .into()
}

/// 组合插件搜索、目录与详情面板的完整设置页面。
pub(crate) fn view<'a>(app: &'a App) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = &app.skills_settings;
    let plugins = s
        .plugin_catalog
        .iter()
        .filter(|plugin| plugin_matches_query(locale, plugin, s.query.trim()))
        .cloned()
        .collect::<Vec<_>>();
    let selected = s
        .selected_plugin_id
        .as_ref()
        .and_then(|selected| s.plugin_catalog.iter().find(|plugin| plugin.id == *selected))
        .or_else(|| plugins.first());

    let search_input = text_input(tr(locale, "搜索插件"), &s.query)
        .on_input(|value| Message::Settings(message::SettingsMessage::SkillsQueryChanged(value)))
        .padding([8, 0])
        .size(13)
        .style(search_text_input_style)
        .width(Length::Fill);

    let header = container(
        column![
            row![
                column![
                    text(tr(locale, "插件")).size(22),
                    text(tr(locale, "原包来源、依赖观察与信任状态；文件存在不代表运行兼容。"))
                        .size(13)
                        .style(|theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.68)),
                        }),
                ]
                .spacing(6)
                .width(Length::Fill),
                detail_action_button(
                    locale,
                    if s.plugin_loading { tr(locale, "刷新中") } else { tr(locale, "刷新") },
                    Icon::ArrowRepeat,
                    DetailActionStyle::Secondary,
                    (!s.plugin_loading)
                        .then_some(Message::Settings(message::SettingsMessage::PluginsRefresh)),
                ),
                detail_action_button(
                    locale,
                    tr(locale, "安装插件"),
                    Icon::Box,
                    DetailActionStyle::Primary,
                    (!s.plugin_loading)
                        .then_some(Message::Settings(message::SettingsMessage::PluginsInstallOpen))
                ),
                skill_badge(
                    crate::__!(crate::app::i18n::I18n::new(locale), "%u 个插件", plugins.len())
                        .expect("valid skills display template"),
                    true
                ),
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
        let rendered = s
            .plugin_status_message_translation
            .as_ref()
            .map(|value| {
                value
                    .render(crate::app::i18n::I18n::new(locale))
                    .expect("valid skills status template")
            })
            .unwrap_or_else(|| message.clone());
        content = content.push(status_banner(rendered, s.plugin_status_is_error));
    }

    if s.plugin_loading && s.plugin_catalog.is_empty() {
        content = content.push(loading_banner(locale));
        return content.into();
    }

    if plugins.is_empty() {
        content = content.push(empty_state(
            tr(locale, "暂无插件"),
            tr(locale, "本地插件目录中没有可展示的插件包。"),
        ));
        return content.into();
    }

    let list = plugins.iter().cloned().fold(column![].spacing(10), |column, plugin| {
        let is_selected = selected.is_some_and(|selected| selected.id == plugin.id);
        column.push(plugin_catalog_item(locale, plugin, is_selected))
    });

    let list_panel = container(
        scrollable(list.padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4)))
            .height(Length::Fixed(460.0)),
    )
    .padding([18, 20])
    .width(Length::FillPortion(3))
    .style(catalog_panel_style);

    let detail_panel =
        container(column![plugin_detail_panel(locale, selected, s.plugin_loading)].spacing(12))
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

fn source_label(locale: DesktopLocale, plugin: &PluginCatalogItem) -> String {
    let kind = match plugin.origin.kind.as_str() {
        "git" => "Git",
        "npm" => "npm",
        "local" => tr(locale, "本地安装"),
        "directory" => tr(locale, "目录发现"),
        _ => tr(locale, "来源待确认"),
    };
    let market = plugin.origin.marketplace.as_deref().unwrap_or(tr(locale, "无市场归属"));
    crate::__!(
        crate::app::i18n::I18n::new(locale),
        "来源：%s · %s · 解析版本 %s",
        (kind).to_string(),
        (market).to_string(),
        (plugin.origin.resolved_version.as_deref().unwrap_or(tr(locale, "未记录"))).to_string()
    )
    .expect("valid skills display template")
}
fn hook_label(status: &str) -> &'static str {
    match status {
        "not_declared" => "未声明钩子",
        "pending" => "钩子待信任；审核前不会执行",
        "trusted" => "已记录钩子信任；仍受宿主策略约束",
        "unsupported" => "钩子含不支持的声明",
        _ => "钩子信任状态待验证",
    }
}
fn dependency_panel(
    locale: DesktopLocale,
    checks: &[vw_gateway_client::DesktopPluginDependencyDto],
) -> Element<'static, Message> {
    let content = checks.iter().fold(column![].spacing(9), |column, check| {
        let label = match check.status.as_str() {
            "present" => tr(locale, "已检测存在"),
            "missing_host" => tr(locale, "缺少宿主程序"),
            "missing_package" => tr(locale, "包内资源缺失"),
            "unverified" => tr(locale, "待验证"),
            "scope_excluded" => tr(locale, "范围外托管能力"),
            "invalid" => tr(locale, "声明无效"),
            _ => tr(locale, "未知状态"),
        };
        column.push(
            column![
                text(format!("{} · {}：{label}", check.component, check.dependency))
                    .size(12)
                    .wrapping(iced::widget::text::Wrapping::Glyph),
                text(check.detail.clone()).size(11).wrapping(iced::widget::text::Wrapping::Glyph)
            ]
            .spacing(3),
        )
    });
    scrollable(content.padding([0, 7]))
        .direction(Direction::Vertical(Scrollbar::new().width(4)))
        .height(Length::Fixed(180.0))
        .into()
}

fn icon_svg(icon: Icon, size: f32) -> iced::widget::Svg<'static> {
    base_icon_svg(icon, size).style(|theme: &iced::Theme, _| iced::widget::svg::Style {
        color: Some(theme.palette().text),
    })
}
fn detail_action_button(
    locale: DesktopLocale,
    label: &'static str,
    icon: Icon,
    style: DetailActionStyle,
    on_press: Option<Message>,
) -> Element<'static, Message> {
    let contrast = matches!(&style, DetailActionStyle::Primary | DetailActionStyle::Danger);
    let icon =
        base_icon_svg(icon, 14.).style(move |theme: &iced::Theme, _| iced::widget::svg::Style {
            color: Some(if contrast { iced::Color::WHITE } else { theme.palette().text }),
        });
    let button =
        button(row![icon, text(tr(locale, label)).size(12)].spacing(8).align_y(Alignment::Center))
            .padding([10, 14])
            .on_press_maybe(on_press);
    match style {
        DetailActionStyle::Primary => button.style(primary_action_btn_style).into(),
        DetailActionStyle::Secondary => button.style(rounded_action_btn_style).into(),
        DetailActionStyle::Danger => button.style(danger_action_btn_style).into(),
    }
}
