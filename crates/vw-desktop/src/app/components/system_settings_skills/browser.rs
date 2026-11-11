//! 系统设置中技能管理页面的浏览、目录或帮助视图。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::catalog::{
    catalog_group_section, catalog_matches_query, icon_svg, section_card_style, skill_badge,
};
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style,
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_error_banner, settings_field_label, settings_group_card, settings_group_heading,
    settings_hover_help_label, settings_modal_card, settings_modal_header, settings_modal_overlay,
    settings_muted_text_style, settings_panel_style, settings_segment_button_style,
    settings_select, settings_success_banner, settings_text_input_style, settings_value_badge,
};
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::state::{SkillsCatalogKind as CatalogSkillKind, SkillsDirectoryScope};
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Element, Length};
use vw_config_types::skills::SkillsDirectoryProvider;
use vw_config_types::ui::DesktopLocale;

#[derive(Clone, Copy)]
pub(super) enum DetailActionStyle {
    Primary,
    Secondary,
    Danger,
}

pub(super) fn skill_project_options(app: &App) -> Vec<String> {
    let mut options = Vec::new();
    for path in app.project_path.iter().chain(app.recent_projects.iter()) {
        let path = path.trim();
        if !path.is_empty() && !options.iter().any(|option: &String| option.as_str() == path) {
            options.push(path.to_string());
        }
    }
    options
}

pub(super) fn header_panel_style(theme: &iced::Theme) -> iced::widget::container::Style {
    settings_panel_style(theme)
}

pub(super) fn catalog_panel_style(theme: &iced::Theme) -> iced::widget::container::Style {
    section_card_style(theme)
}

pub(super) fn status_banner<'a>(
    message: impl iced::widget::text::IntoFragment<'a>,
    is_error: bool,
) -> Element<'a, Message> {
    if is_error { settings_error_banner(message) } else { settings_success_banner(message) }
}

pub(super) fn scope_button(
    label: &'static str,
    scope: SkillsDirectoryScope,
    active_scope: SkillsDirectoryScope,
    enabled: bool,
) -> Element<'static, Message> {
    let is_active = scope == active_scope;
    let mut button = button(text(label).size(12)).padding([8, 12]);

    if enabled {
        button = button.on_press(Message::Settings(
            message::SettingsMessage::SkillsDirectoryScopeChanged(scope),
        ));
    }

    button
        .style(move |theme: &iced::Theme, status| {
            let mut style = settings_segment_button_style(theme, status, is_active);
            style.border.width = 0.0;
            style.text_color = theme.palette().text.scale_alpha(if enabled { 1.0 } else { 0.5 });
            style
        })
        .into()
}

pub(super) fn refresh_button(locale: DesktopLocale, loading: bool) -> Element<'static, Message> {
    detail_action_button(
        locale,
        if loading { tr(locale, "刷新中") } else { tr(locale, "刷新") },
        Icon::ArrowRepeat,
        DetailActionStyle::Secondary,
        (!loading).then_some(Message::Settings(message::SettingsMessage::SkillsRefresh)),
    )
}

pub(super) fn scope_source_matches(scope: SkillsDirectoryScope, source: &str) -> bool {
    match scope {
        SkillsDirectoryScope::Project => source == "workspace",
        SkillsDirectoryScope::Ancestor => source == "ancestor",
        SkillsDirectoryScope::Global => source == "global",
        SkillsDirectoryScope::Bundled => source == "bundled",
        SkillsDirectoryScope::All => true,
    }
}

pub(super) fn scope_title(scope: SkillsDirectoryScope) -> &'static str {
    match scope {
        SkillsDirectoryScope::Project => "项目目录技能",
        SkillsDirectoryScope::Ancestor => "父级目录技能",
        SkillsDirectoryScope::Global => "全局目录技能",
        SkillsDirectoryScope::Bundled => "内置技能",
        SkillsDirectoryScope::All => "全部技能目录",
    }
}

pub(super) fn provider_project_paths(provider: SkillsDirectoryProvider) -> &'static str {
    match provider {
        SkillsDirectoryProvider::Vibewindow => ".shiju/skills, skills, .agents/skills",
        SkillsDirectoryProvider::Codex => ".codex/skills, .agents/skills, skills",
        SkillsDirectoryProvider::Claude => ".claude/skills, skills, .agents/skills",
        SkillsDirectoryProvider::Cursor => ".cursor/skills, skills, .agents/skills",
    }
}

pub(super) fn provider_global_paths(provider: SkillsDirectoryProvider) -> &'static str {
    match provider {
        SkillsDirectoryProvider::Vibewindow => {
            #[cfg(debug_assertions)]
            {
                "~/.shiju-harness-dev/skills, ~/.skills"
            }
            #[cfg(not(debug_assertions))]
            {
                "~/.shiju-harness/skills, ~/.skills"
            }
        }
        SkillsDirectoryProvider::Codex => "~/.codex/skills, ~/.agents/skills",
        SkillsDirectoryProvider::Claude => "~/.claude/skills",
        SkillsDirectoryProvider::Cursor => "~/.cursor/skills",
    }
}

pub(super) fn scope_description(
    locale: DesktopLocale,
    scope: SkillsDirectoryScope,
    provider: SkillsDirectoryProvider,
) -> String {
    match scope {
        SkillsDirectoryScope::Project => crate::__!(
            crate::app::i18n::I18n::new(locale),
            "查看当前项目 %s 目录命中的技能。",
            (provider_project_paths(provider)).to_string()
        )
        .expect("valid skills display template"),
        SkillsDirectoryScope::Ancestor => crate::__!(
            crate::app::i18n::I18n::new(locale),
            "查看从项目父级向上发现的 %s 技能。",
            (provider_project_paths(provider)).to_string()
        )
        .expect("valid skills display template"),
        SkillsDirectoryScope::Global => crate::__!(
            crate::app::i18n::I18n::new(locale),
            "查看当前用户 %s 中的全局技能。",
            (provider_global_paths(provider)).to_string()
        )
        .expect("valid skills display template"),
        SkillsDirectoryScope::Bundled => {
            tr(locale, "查看产品随包提供、可安装到项目的内置技能。").to_string()
        }
        SkillsDirectoryScope::All => tr(
            locale,
            "全部结果包含当前项目、父级目录、全局目录以及内置技能，便于核对最终命中来源。",
        )
        .to_string(),
    }
}

pub(super) fn discovery_order_text(app: &App) -> String {
    let locale = app.desktop_locale;
    if let Some(project_path) = &app.project_path {
        return crate::__!(
            crate::app::i18n::I18n::new(locale),
            "发现顺序：%s/%s -> 父级 %s -> %s",
            (project_path).to_string(),
            (provider_project_paths(app.skills_settings.directory_provider)).to_string(),
            (provider_project_paths(app.skills_settings.directory_provider)).to_string(),
            (provider_global_paths(app.skills_settings.directory_provider)).to_string()
        )
        .expect("valid skills display template");
    }

    crate::__!(
        crate::app::i18n::I18n::new(locale),
        "未打开项目时，仅显示 %s 与内置技能。",
        (provider_global_paths(app.skills_settings.directory_provider)).to_string()
    )
    .expect("valid skills display template")
}

pub(super) fn discovery_order_view<'a>(app: &'a App) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let project_open = app.project_path.is_some();
    let provider = app.skills_settings.directory_provider;
    let order = if project_open {
        vec![
            (
                "1",
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
                    "当前项目 %s",
                    (provider_project_paths(provider)).to_string()
                )
                .expect("valid skills display template"),
                tr(locale, "项目私有技能，优先级最高。").to_string(),
            ),
            (
                "2",
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
                    "父级 %s",
                    (provider_project_paths(provider)).to_string()
                )
                .expect("valid skills display template"),
                tr(locale, "从项目父级向上发现，越近优先级越高。").to_string(),
            ),
            (
                "3",
                provider_global_paths(provider).to_string(),
                tr(locale, "当前用户全局技能目录。").to_string(),
            ),
            (
                "4",
                tr(locale, "内置技能").to_string(),
                tr(locale, "产品随包提供，可安装到项目后覆盖。").to_string(),
            ),
        ]
    } else {
        vec![
            (
                "1",
                provider_global_paths(provider).to_string(),
                tr(locale, "未打开项目时使用当前用户全局技能目录。").to_string(),
            ),
            (
                "2",
                tr(locale, "内置技能").to_string(),
                tr(locale, "产品随包提供，可安装到项目后覆盖。").to_string(),
            ),
        ]
    };

    let rows = order.into_iter().fold(column![].spacing(10), |column, (index, title, desc)| {
        column.push(
            container(
                row![
                    settings_value_badge(index),
                    column![
                        text(title).size(14).wrapping(iced::widget::text::Wrapping::Glyph),
                        text(desc).size(12).style(|theme: &iced::Theme| {
                            iced::widget::text::Style {
                                color: Some(theme.palette().text.scale_alpha(0.64)),
                            }
                        }),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding([12, 14])
            .width(Length::Fill)
            .style(section_card_style),
        )
    });

    let title = tr(locale, "发现顺序");
    column![
        container(
            column![
                row![
                    settings_hover_help_label(
                        I18n::new(locale),
                        &app.app_theme,
                        title,
                        text(title).size(22),
                        tr(locale, "同名技能只展示最先命中的目录，保持项目优先，避免列表重复。",),
                    )
                    .width(Length::Fill)
                    .wrap(),
                    settings_value_badge(if project_open {
                        tr(locale, "项目已打开")
                    } else {
                        tr(locale, "未打开项目")
                    }),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
                text(discovery_order_text(app))
                    .size(12)
                    .width(Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::Glyph)
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.68)),
                    }),
            ]
            .spacing(12),
        )
        .padding([18, 20])
        .width(Length::Fill)
        .style(header_panel_style),
        rows,
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn loading_banner<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(12))
        .padding([10, 12])
        .width(Length::Fill)
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                text_color: Some(theme.palette().text.scale_alpha(0.72)),
                background: Some(Background::Color(
                    palette.background.weak.color.scale_alpha(0.18),
                )),
                border: Border {
                    width: 1.0,
                    color: palette.background.strong.color.scale_alpha(0.45),
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn empty_state<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    container(
        column![
            container(icon_svg(Icon::Search, 18.0)).padding([12, 12]).style(
                |theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::container::Style {
                        text_color: Some(theme.palette().text.scale_alpha(0.7)),
                        background: Some(Background::Color(
                            palette.background.weak.color.scale_alpha(0.22),
                        )),
                        border: Border { radius: 14.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                }
            ),
            text(title).size(14).wrapping(iced::widget::text::Wrapping::Glyph),
            text(description).size(12).style(|theme: &iced::Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.68)),
            }),
        ]
        .spacing(6)
        .align_x(Alignment::Center),
    )
    .padding([18, 14])
    .width(Length::Fill)
    .center_x(Length::Fill)
    .style(section_card_style)
    .into()
}

pub(super) fn detail_source_note(
    locale: DesktopLocale,
    path: Option<&str>,
) -> Element<'static, Message> {
    if let Some(path) = path {
        return container(
            text(path.to_string())
                .size(11)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::Glyph)
                .style(|theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.6)),
                }),
        )
        .width(Length::Fill)
        .into();
    }

    container(text(tr(locale, "当前为内置技能文档，未关联本地目录。")).size(11).style(
        |theme: &iced::Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(0.6)),
        },
    ))
    .width(Length::Fill)
    .into()
}

pub(super) fn detail_action_button(
    locale: DesktopLocale,
    label: &'static str,
    icon: Icon,
    style: DetailActionStyle,
    on_press: Option<Message>,
) -> Element<'static, Message> {
    let on_press_enabled = on_press.is_some();
    let icon = icon_svg(icon, 14.0).style(move |theme: &iced::Theme, _| {
        let color = match (on_press_enabled, style) {
            (false, _) => theme.palette().text.scale_alpha(0.45),
            (true, DetailActionStyle::Primary | DetailActionStyle::Danger) => iced::Color::WHITE,
            (true, DetailActionStyle::Secondary) => theme.palette().text,
        };
        iced::widget::svg::Style { color: Some(color) }
    });
    let content =
        row![icon, text(tr(locale, label)).size(13)].spacing(8).align_y(Alignment::Center);

    let mut btn = button(content).padding([8, 14]);
    if let Some(message) = on_press {
        btn = btn.on_press(message);
    }

    match style {
        DetailActionStyle::Primary => btn.style(primary_action_btn_style).into(),
        DetailActionStyle::Secondary => btn.style(rounded_action_btn_style).into(),
        DetailActionStyle::Danger => btn.style(danger_action_btn_style).into(),
    }
}

pub(super) fn detail_modal_body<'a>(app: &'a App, project_open: bool) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = &app.skills_settings;

    if s.detail_loading {
        return loading_banner(tr(locale, "正在通过 gateway 同步技能目录..."));
    }

    if let Some(err) = &s.detail_error {
        return settings_error_banner(err);
    }

    if let Some(detail) = &s.selected_skill_detail {
        let source_badge = match detail.source.as_str() {
            "workspace" => tr(locale, "项目目录"),
            "ancestor" => tr(locale, "父级目录"),
            "global" => tr(locale, "全局目录"),
            "bundled" => tr(locale, "内置"),
            _ => tr(locale, "来源"),
        };
        let busy = s.loading || s.detail_loading;

        let mut actions = row![].spacing(8).align_y(Alignment::Center);

        if detail.can_install {
            actions = actions.push(detail_action_button(
                locale,
                tr(locale, "安装到项目"),
                Icon::Plus,
                DetailActionStyle::Primary,
                (!busy && project_open).then_some(Message::Settings(
                    message::SettingsMessage::SkillsInstallBuiltInRequested(detail.id.clone()),
                )),
            ));
        }

        if detail.can_toggle {
            actions = actions.push(detail_action_button(
                locale,
                if detail.enabled { tr(locale, "禁用") } else { tr(locale, "启用") },
                if detail.enabled { Icon::EyeSlash } else { Icon::Eye },
                DetailActionStyle::Secondary,
                (!busy).then_some(Message::Settings(
                    message::SettingsMessage::SkillsSetEnabledRequested {
                        skill_id: detail.id.clone(),
                        enabled: !detail.enabled,
                    },
                )),
            ));
        }

        if detail.can_delete {
            actions = actions.push(detail_action_button(
                locale,
                tr(locale, "删除"),
                Icon::Trash,
                DetailActionStyle::Danger,
                (!busy).then_some(Message::Settings(
                    message::SettingsMessage::SkillsDeleteRequested(detail.id.clone()),
                )),
            ));
        }

        let summary = container(
            column![
                row![
                    column![
                        text(&detail.title).size(17).wrapping(iced::widget::text::Wrapping::Glyph),
                        text(&detail.description)
                            .size(12)
                            .wrapping(iced::widget::text::Wrapping::Glyph)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    skill_badge(
                        match detail.kind {
                            CatalogSkillKind::Recommended => tr(locale, "推荐"),
                            CatalogSkillKind::System => tr(locale, "内置"),
                            CatalogSkillKind::Personal => tr(locale, "本地"),
                        },
                        matches!(detail.kind, CatalogSkillKind::Recommended),
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![
                    settings_value_badge(source_badge),
                    settings_value_badge(if detail.installed {
                        tr(locale, "已安装")
                    } else {
                        tr(locale, "未安装")
                    }),
                    settings_value_badge(if detail.enabled {
                        tr(locale, "已启用")
                    } else {
                        tr(locale, "已禁用")
                    }),
                    skill_badge(detail.document_name.clone(), true),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap(),
                detail_source_note(locale, detail.source_path.as_deref()),
            ]
            .spacing(10),
        )
        .padding([16, 18])
        .width(Length::Fill)
        .style(section_card_style);

        let document = container(
            column![
                row![icon_svg(Icon::FileText, 14.0), text(&detail.document_name).size(13),]
                    .spacing(8)
                    .align_y(Alignment::Center),
                scrollable(
                    container(
                        text(&detail.document_content)
                            .size(12)
                            .width(Length::Fill)
                            .wrapping(iced::widget::text::Wrapping::Glyph),
                    )
                    .padding([0, 7])
                    .width(Length::Fill),
                )
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .width(Length::Fill)
                .height(Length::Fill),
            ]
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .padding([16, 18])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(section_card_style);

        let content = column![summary]
            .push(container(actions.wrap()).width(Length::Fill))
            .push(document)
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        return container(content).width(Length::Fill).height(Length::Fill).into();
    }

    empty_state(
        tr(locale, "加载技能详情中"),
        tr(
            locale,
            "点击左侧技能卡片后，这里会显示 SKILL.md 或 SKILL.toml 内容，并提供启用、禁用、删除操作。",
        ),
    )
}

/// 构建或处理 `view_overlays` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = &app.skills_settings;
    if s.selected_skill_id.is_none() {
        return base;
    }

    let close_message = Message::Settings(message::SettingsMessage::SkillsDetailClosed);
    let title = s
        .selected_skill_detail
        .as_ref()
        .map(|detail| detail.title.as_str())
        .unwrap_or(tr(locale, "技能详情"));
    let subtitle = s
        .selected_skill_detail
        .as_ref()
        .map(|detail| match detail.source.as_str() {
            "workspace" => tr(locale, "项目目录技能"),
            "ancestor" => tr(locale, "父级目录技能"),
            "global" => tr(locale, "全局目录技能"),
            "bundled" => tr(locale, "内置技能"),
            _ => tr(locale, "技能"),
        })
        .unwrap_or(tr(locale, "读取技能文档中"));

    let card = settings_modal_card(
        column![
            settings_modal_header(
                settings_group_heading(title, subtitle),
                settings_close_button(close_message.clone()),
            ),
            detail_modal_body(app, app.project_path.is_some()),
        ]
        .spacing(14)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .style(super::modal_card_style)
    .width(Length::Fixed(860.0))
    .height(Length::Fixed(640.0));

    settings_modal_overlay(Some(base), close_message, container(card).padding(12))
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn view<'a>(app: &'a App) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = &app.skills_settings;
    let project_open = app.project_path.is_some();
    let skills = s
        .catalog
        .iter()
        .filter(|skill| catalog_matches_query(locale, skill, s.query.trim()))
        .filter(|skill| scope_source_matches(s.directory_scope, &skill.source))
        .cloned()
        .collect::<Vec<_>>();

    let search_input = text_input(tr(locale, "搜索技能"), &s.query)
        .on_input(|value| Message::Settings(message::SettingsMessage::SkillsQueryChanged(value)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .width(Length::Fill);
    let project_options = skill_project_options(app);
    let project_switcher: Element<'_, Message> = if project_options.is_empty() {
        text(tr(locale, "尚未打开项目；当前仅显示全局与内置技能。"))
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        row![
            settings_field_label(tr(locale, "项目目录")),
            settings_select(project_options, app.project_path.clone(), |path| {
                Message::Settings(message::SettingsMessage::SkillsProjectSelected(path))
            })
            .placeholder(tr(locale, "选择项目目录"))
            .width(Length::Fill),
            text(if s.loading { tr(locale, "切换中") } else { tr(locale, "技能作用域") })
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    };

    let filters = settings_group_card(
        container(
            column![
                row![icon_svg(Icon::Search, 16.0), search_input, refresh_button(locale, s.loading)]
                    .spacing(10)
                    .align_y(Alignment::Center),
                project_switcher,
                row![
                    scope_button(
                        tr(locale, "项目目录"),
                        SkillsDirectoryScope::Project,
                        s.directory_scope,
                        project_open,
                    ),
                    scope_button(
                        tr(locale, "父级目录"),
                        SkillsDirectoryScope::Ancestor,
                        s.directory_scope,
                        project_open,
                    ),
                    scope_button(
                        tr(locale, "全局目录"),
                        SkillsDirectoryScope::Global,
                        s.directory_scope,
                        true
                    ),
                    scope_button(
                        tr(locale, "内置技能"),
                        SkillsDirectoryScope::Bundled,
                        s.directory_scope,
                        true
                    ),
                    scope_button(
                        tr(locale, "全部目录"),
                        SkillsDirectoryScope::All,
                        s.directory_scope,
                        true
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap(),
            ]
            .spacing(12),
        )
        .padding([14, 0]),
    );

    let mut content = column![filters].spacing(20).width(Length::Fill);

    if let Some(message) = &s.status_message {
        let rendered = s
            .status_message_translation
            .as_ref()
            .map(|value| {
                value
                    .render(crate::app::i18n::I18n::new(locale))
                    .expect("valid skills status template")
            })
            .unwrap_or_else(|| message.clone());
        content = content.push(status_banner(rendered, s.status_is_error));
    }

    let mut catalog_panel = column![
        row![
            column![
                text(tr(locale, scope_title(s.directory_scope))).size(18),
                text(scope_description(locale, s.directory_scope, s.directory_provider))
                    .size(13)
                    .style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            text(
                crate::__!(crate::app::i18n::I18n::new(locale), "%u 项技能", skills.len())
                    .expect("valid skill results template")
            )
            .size(12)
            .style(settings_muted_text_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(20)
    .width(Length::Fill);

    if s.loading {
        catalog_panel =
            catalog_panel.push(loading_banner(tr(locale, "正在通过 gateway 同步技能目录...")));
    }

    let workspace =
        skills.iter().filter(|skill| skill.source == "workspace").cloned().collect::<Vec<_>>();
    let ancestor =
        skills.iter().filter(|skill| skill.source == "ancestor").cloned().collect::<Vec<_>>();
    let global =
        skills.iter().filter(|skill| skill.source == "global").cloned().collect::<Vec<_>>();
    let bundled =
        skills.iter().filter(|skill| skill.source == "bundled").cloned().collect::<Vec<_>>();

    if !workspace.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            locale,
            "workspace",
            workspace,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !ancestor.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            locale,
            "ancestor",
            ancestor,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !global.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            locale,
            "global",
            global,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !bundled.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            locale,
            "bundled",
            bundled,
            s.selected_skill_id.as_deref(),
        ));
    }

    if skills.is_empty() && !s.loading {
        let (title, description) = match (s.directory_scope, project_open) {
            (SkillsDirectoryScope::Project, false) => (
                tr(locale, "未打开项目目录"),
                tr(locale, "打开项目后可查看当前工程的 skills 目录，或先切换到全部目录。"),
            ),
            (SkillsDirectoryScope::Project, true) => (
                tr(locale, "项目目录下没有技能"),
                tr(
                    locale,
                    "当前项目目录没有命中技能，可以切换到全部目录查看父级、全局和内置技能。",
                ),
            ),
            (SkillsDirectoryScope::Ancestor, false) => {
                (tr(locale, "未打开项目目录"), tr(locale, "打开项目后可查看父级技能目录。"))
            }
            (SkillsDirectoryScope::Ancestor, true) => (
                tr(locale, "父级目录下没有技能"),
                tr(locale, "项目父级目录没有命中技能，可以切换到全局目录或内置技能。"),
            ),
            (SkillsDirectoryScope::Global, _) => (
                tr(locale, "全局目录下没有技能"),
                tr(locale, "当前目录提供方的全局技能目录没有命中技能。"),
            ),
            (SkillsDirectoryScope::Bundled, _) => (
                tr(locale, "没有内置技能"),
                tr(locale, "内置技能目录为空，或当前搜索关键字没有匹配结果。"),
            ),
            (SkillsDirectoryScope::All, _) => (
                tr(locale, "没有匹配的技能"),
                tr(locale, "试试清空搜索关键字，或稍后刷新技能目录。"),
            ),
        };
        catalog_panel = catalog_panel.push(empty_state(title, description));
    }

    content.push(catalog_panel).into()
}
