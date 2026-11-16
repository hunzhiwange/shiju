//! 构建代理设置中的插件检索、详情与管理页面。

use super::browser::{
    DetailActionStyle, catalog_panel_style, detail_action_button, empty_state, header_panel_style,
    loading_banner, status_banner,
};
use super::catalog::{catalog_item_button_style, icon_svg, section_card_style, skill_badge};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_group_heading_with_help,
    settings_muted_text_style, settings_text_input_style, settings_value_badge,
};
use desktop_i18n::{I18n, settings_extensions::text as tr};
use crate::types::PluginCatalogItem;
use crate::Event as Message;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, responsive, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Element, Length};
use config_ui::DesktopLocale;

/// 在插件标识、名称、版本、说明和来源路径中执行不区分大小写的匹配。
pub fn plugin_matches_query(
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
pub fn plugin_set_enabled_message(plugin_id: &str, enabled: bool) -> Message {
    Message::PluginsSetEnabledRequested {
        plugin_id: plugin_id.to_string(),
        enabled,
    }
}

/// 构造插件删除请求消息，由设置更新层执行确认与持久化。
pub fn plugin_delete_message(plugin_id: &str) -> Message {
    Message::PluginsDeleteRequested(plugin_id.to_string())
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
pub fn plugin_catalog_item(
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
        desktop_i18n::__!(desktop_i18n::I18n::new(locale), "%u 个组件", plugin.component_count)
            .expect("valid skills display template");
    let compatibility = tr(locale, super::plugin_dependency_summary(&plugin.dependency_checks));
    let status = if plugin.enabled { tr(locale, "已启用") } else { tr(locale, "已禁用") };

    let details = column![
        row![
            text(display_name)
                .size(15)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::Glyph),
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
            .wrapping(iced::widget::text::Wrapping::Glyph)
            .style(settings_muted_text_style),
        row![
            skill_badge(plugin_id.clone(), false),
            skill_badge(component_text, false),
            skill_badge(status, plugin.enabled),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .wrap(),
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
    .width(Length::Fill);

    button(content)
        .width(Length::Fill)
        .padding([12, 14])
        .on_press(Message::PluginsSelected(plugin.id))
        .style(move |theme, status| catalog_item_button_style(theme, status, is_selected))
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
pub fn plugin_detail_panel(
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
            (!busy).then_some(Message::PluginsUpdateOpen(
                plugin_id.clone()
            ))
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
    .align_y(Alignment::Center)
    .wrap();

    container(
        column![
            row![
                column![
                    text(display_name).size(17).wrapping(iced::widget::text::Wrapping::Glyph),
                    text(description)
                        .size(12)
                        .width(Length::Fill)
                        .wrapping(iced::widget::text::Wrapping::Glyph)
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
                    desktop_i18n::__!(desktop_i18n::I18n::new(locale), "%u 个组件", component_count)
                        .expect("valid skills display template")
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(source_label(locale, plugin))
                .size(12)
                .wrapping(iced::widget::text::Wrapping::Glyph),
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
pub fn view<'a>(app: ViewState<'a>) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = app;
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
        .or_else(|| plugins.first())
        .cloned();

    let search_input = text_input(tr(locale, "搜索插件"), &s.query)
        .on_input(|value| Message::SkillsQueryChanged(value))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .width(Length::Fill);

    let header = container(
        column![
            settings_group_heading_with_help(
                I18n::new(locale),
                app.app_theme,
                tr(locale, "插件"),
                tr(locale, "原包来源、依赖观察与信任状态；文件存在不代表运行兼容。"),
            ),
            row![
                detail_action_button(
                    locale,
                    if s.plugin_loading { tr(locale, "刷新中") } else { tr(locale, "刷新") },
                    Icon::ArrowRepeat,
                    DetailActionStyle::Secondary,
                    (!s.plugin_loading)
                        .then_some(Message::PluginsRefresh),
                ),
                detail_action_button(
                    locale,
                    tr(locale, "安装插件"),
                    Icon::Box,
                    DetailActionStyle::Primary,
                    (!s.plugin_loading)
                        .then_some(Message::PluginsInstallOpen)
                ),
                skill_badge(
                    desktop_i18n::__!(desktop_i18n::I18n::new(locale), "%u 个插件", plugins.len())
                        .expect("valid skills display template"),
                    true
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap(),
            search_input,
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
                    .render(desktop_i18n::I18n::new(locale))
                    .expect("valid skills status template")
            })
            .unwrap_or_else(|| message.clone());
        content = content.push(status_banner(rendered, s.plugin_status_is_error));
    }

    if s.plugin_loading && s.plugin_catalog.is_empty() {
        content = content.push(loading_banner(tr(locale, "刷新中")));
        return content.into();
    }

    if plugins.is_empty() {
        content = content.push(empty_state(
            tr(locale, "暂无插件"),
            tr(locale, "本地插件目录中没有可展示的插件包。"),
        ));
        return content.into();
    }

    content = content.push(
        responsive(move |size| {
            let stacked = size.width < 760.0;
            let list = plugins.iter().cloned().fold(column![].spacing(8), |list, plugin| {
                let is_selected =
                    selected.as_ref().is_some_and(|selected| selected.id == plugin.id);
                list.push(plugin_catalog_item(locale, plugin, is_selected))
            });
            let list_panel = container(
                scrollable(list.padding([0, 7]))
                    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                    .height(Length::Shrink),
            )
            .padding([12, 8])
            .max_height(if stacked { 244 } else { 484 })
            .width(Length::FillPortion(3))
            .style(catalog_panel_style);
            let detail_panel =
                container(plugin_detail_panel(locale, selected.as_ref(), s.plugin_loading))
                    .width(Length::FillPortion(2));

            if stacked {
                column![list_panel, detail_panel].spacing(14).into()
            } else {
                row![list_panel, detail_panel].spacing(14).align_y(Alignment::Start).into()
            }
        })
        .height(Length::Shrink),
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
    desktop_i18n::__!(
        desktop_i18n::I18n::new(locale),
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
    checks: &[gateway_desktop_settings::DesktopPluginDependencyDto],
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
    container(
        scrollable(content.padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Shrink),
    )
    .max_height(180)
    .into()
}

#[derive(Clone, Copy)]
pub struct ViewState<'a> {
 pub desktop_locale: DesktopLocale, pub app_theme: &'a iced::Theme,
 pub query: &'a String, pub plugin_catalog: &'a Vec<PluginCatalogItem>,
 pub selected_plugin_id: &'a Option<String>, pub plugin_loading: bool,
 pub plugin_status_message: &'a Option<String>, pub plugin_status_message_translation: &'a Option<desktop_i18n::format::Message>, pub plugin_status_is_error: bool,
}
