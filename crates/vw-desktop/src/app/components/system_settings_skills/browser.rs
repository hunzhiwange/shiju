//! 系统设置中技能管理页面的浏览、目录或帮助视图。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::catalog::{
    catalog_group_section, catalog_matches_query, section_card_style, skill_badge,
};
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, icon_svg, primary_action_btn_style, rounded_action_btn_style,
    settings_close_button, settings_error_banner, settings_modal_card, settings_modal_overlay,
    settings_muted_text_style, settings_panel_style, settings_pick_list_menu_style,
    settings_pick_list_style, settings_segment_button_style, settings_value_badge,
};
use crate::app::state::{SkillsCatalogKind as CatalogSkillKind, SkillsDirectoryScope};
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Background, Border, Element, Length};
use vw_config_types::skills::SkillsDirectoryProvider;

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

pub(super) fn search_bar_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    style.background = Some(Background::Color(theme.palette().background));
    style.border.radius = 12.0.into();
    style.shadow = iced::Shadow::default();
    style
}

pub(super) fn search_text_input_style(
    theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let palette = theme.palette();
    let disabled = matches!(status, iced::widget::text_input::Status::Disabled);

    iced::widget::text_input::Style {
        background: Background::Color(iced::Color::TRANSPARENT),
        border: Border { width: 0.0, color: iced::Color::TRANSPARENT, radius: 0.0.into() },
        icon: palette.text.scale_alpha(0.65),
        placeholder: palette.text.scale_alpha(0.48),
        value: if disabled { palette.text.scale_alpha(0.50) } else { palette.text },
        selection: palette.primary.scale_alpha(0.22),
    }
}

pub(super) fn header_panel_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    let palette = theme.extended_palette();
    style.background = Some(Background::Color(palette.background.weak.color.scale_alpha(0.24)));
    style.border.color = palette.background.strong.color.scale_alpha(0.68);
    style.border.radius = 22.0.into();
    style
}

pub(super) fn catalog_panel_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    let palette = theme.extended_palette();
    style.background = Some(Background::Color(palette.background.weak.color.scale_alpha(0.14)));
    style.border.color = palette.background.strong.color.scale_alpha(0.52);
    style.border.radius = 20.0.into();
    style
}

pub(super) fn status_banner<'a>(message: &'a str, is_error: bool) -> Element<'a, Message> {
    if is_error {
        return settings_error_banner(message);
    }

    container(text(message).size(13))
        .padding([10, 12])
        .width(Length::Fill)
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                text_color: Some(palette.success.base.color),
                background: Some(Background::Color(palette.success.weak.color.scale_alpha(0.18))),
                border: Border {
                    width: 1.0,
                    color: palette.success.base.color.scale_alpha(0.4),
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
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
            settings_segment_button_style(theme, status, is_active)
        })
        .into()
}

pub(super) fn refresh_button(loading: bool) -> Element<'static, Message> {
    detail_action_button(
        if loading { "刷新中" } else { "刷新" },
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
        SkillsDirectoryProvider::Vibewindow => ".vibewindow/skills、skills 与 .agents/skills",
        SkillsDirectoryProvider::Codex => ".codex/skills、.agents/skills 与 skills",
        SkillsDirectoryProvider::Claude => ".claude/skills、skills 与 .agents/skills",
        SkillsDirectoryProvider::Cursor => ".cursor/skills、skills 与 .agents/skills",
    }
}

pub(super) fn provider_global_paths(provider: SkillsDirectoryProvider) -> &'static str {
    match provider {
        SkillsDirectoryProvider::Vibewindow => {
            #[cfg(debug_assertions)]
            {
                "~/.shiju-agent-dev/skills 与 ~/.skills"
            }
            #[cfg(not(debug_assertions))]
            {
                "~/.shiju-agent/skills 与 ~/.skills"
            }
        }
        SkillsDirectoryProvider::Codex => "~/.codex/skills 与 ~/.agents/skills",
        SkillsDirectoryProvider::Claude => "~/.claude/skills",
        SkillsDirectoryProvider::Cursor => "~/.cursor/skills",
    }
}

pub(super) fn scope_description(
    scope: SkillsDirectoryScope,
    provider: SkillsDirectoryProvider,
) -> String {
    match scope {
        SkillsDirectoryScope::Project => {
            format!("查看当前项目 {} 目录命中的技能。", provider_project_paths(provider))
        }
        SkillsDirectoryScope::Ancestor => {
            format!("查看从项目父级向上发现的 {} 技能。", provider_project_paths(provider))
        }
        SkillsDirectoryScope::Global => {
            format!("查看当前用户 {} 中的全局技能。", provider_global_paths(provider))
        }
        SkillsDirectoryScope::Bundled => "查看产品随包提供、可安装到项目的内置技能。".to_string(),
        SkillsDirectoryScope::All => {
            "全部结果包含当前项目、父级目录、全局目录以及内置技能，便于核对最终命中来源。"
                .to_string()
        }
    }
}

pub(super) fn active_scope_badge(scope: SkillsDirectoryScope) -> &'static str {
    match scope {
        SkillsDirectoryScope::Project => "当前筛选: 项目目录",
        SkillsDirectoryScope::Ancestor => "当前筛选: 父级目录",
        SkillsDirectoryScope::Global => "当前筛选: 全局目录",
        SkillsDirectoryScope::Bundled => "当前筛选: 内置技能",
        SkillsDirectoryScope::All => "当前筛选: 全部目录",
    }
}

pub(super) fn discovery_order_text(app: &App) -> String {
    if let Some(project_path) = &app.project_path {
        return format!(
            "{project_path}/{} -> 父级 {} -> {}",
            provider_project_paths(app.skills_settings.directory_provider),
            provider_project_paths(app.skills_settings.directory_provider),
            provider_global_paths(app.skills_settings.directory_provider)
        );
    }

    format!(
        "未打开项目时，仅显示 {} 与内置技能。",
        provider_global_paths(app.skills_settings.directory_provider)
    )
}

pub(super) fn discovery_order_view<'a>(app: &'a App) -> Element<'a, Message> {
    let project_open = app.project_path.is_some();
    let provider = app.skills_settings.directory_provider;
    let order = if project_open {
        vec![
            (
                "1",
                format!("当前项目 {}", provider_project_paths(provider)),
                "项目私有技能，优先级最高。".to_string(),
            ),
            (
                "2",
                format!("父级 {}", provider_project_paths(provider)),
                "从项目父级向上发现，越近优先级越高。".to_string(),
            ),
            (
                "3",
                provider_global_paths(provider).to_string(),
                "当前用户全局技能目录。".to_string(),
            ),
            ("4", "内置技能".to_string(), "产品随包提供，可安装到项目后覆盖。".to_string()),
        ]
    } else {
        vec![
            (
                "1",
                provider_global_paths(provider).to_string(),
                "未打开项目时使用当前用户全局技能目录。".to_string(),
            ),
            ("2", "内置技能".to_string(), "产品随包提供，可安装到项目后覆盖。".to_string()),
        ]
    };

    let rows = order.into_iter().fold(column![].spacing(10), |column, (index, title, desc)| {
        column.push(
            container(
                row![
                    settings_value_badge(index),
                    column![
                        text(title).size(14),
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

    column![
        container(
            column![
                row![
                    column![
                        text("发现顺序").size(22),
                        text("同名技能只展示最先命中的目录，保持项目优先，避免列表重复。")
                            .size(13)
                            .style(|theme: &iced::Theme| iced::widget::text::Style {
                                color: Some(theme.palette().text.scale_alpha(0.68)),
                            }),
                    ]
                    .spacing(6)
                    .width(Length::Fill),
                    settings_value_badge(if project_open {
                        "项目已打开"
                    } else {
                        "未打开项目"
                    }),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
                text(discovery_order_text(app))
                    .size(12)
                    .width(Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.68)),
                    }),
            ]
            .spacing(12),
        )
        .padding([18, 20])
        .width(Length::Fill)
        .style(header_panel_style),
        container(rows).padding([18, 20]).width(Length::Fill).style(catalog_panel_style),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn loading_banner<'a>() -> Element<'a, Message> {
    container(text("正在通过 gateway 同步技能目录...").size(12))
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
            text(title).size(14),
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

pub(super) fn detail_source_note(path: Option<&str>) -> Element<'static, Message> {
    if let Some(path) = path {
        return container(
            text(path.to_string())
                .size(11)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::Word)
                .style(|theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.6)),
                }),
        )
        .width(Length::Fill)
        .into();
    }

    container(text("当前为内置技能文档，未关联本地目录。").size(11).style(|theme: &iced::Theme| {
        iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.6)) }
    }))
    .width(Length::Fill)
    .into()
}

pub(super) fn detail_action_button(
    label: &'static str,
    icon: Icon,
    style: DetailActionStyle,
    on_press: Option<Message>,
) -> Element<'static, Message> {
    let content =
        row![icon_svg(icon, 14.0), text(label).size(12)].spacing(8).align_y(Alignment::Center);

    let mut btn = button(content).padding([10, 14]);
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
    let s = &app.skills_settings;

    if s.detail_loading {
        return loading_banner();
    }

    if let Some(err) = &s.detail_error {
        return settings_error_banner(err);
    }

    if let Some(detail) = &s.selected_skill_detail {
        let source_badge = match detail.source.as_str() {
            "workspace" => "项目目录",
            "ancestor" => "父级目录",
            "global" => "全局目录",
            "bundled" => "内置",
            _ => "来源",
        };
        let busy = s.loading || s.detail_loading;

        let mut actions = row![].spacing(8).align_y(Alignment::Center);

        if detail.can_install {
            actions = actions.push(detail_action_button(
                "安装到项目",
                Icon::Plus,
                DetailActionStyle::Primary,
                (!busy && project_open).then_some(Message::Settings(
                    message::SettingsMessage::SkillsInstallBuiltInRequested(detail.id.clone()),
                )),
            ));
        }

        if detail.can_toggle {
            actions = actions.push(detail_action_button(
                if detail.enabled { "禁用" } else { "启用" },
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
                "删除",
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
                        text(&detail.title).size(20),
                        text(&detail.description).size(12).style(|theme: &iced::Theme| {
                            iced::widget::text::Style {
                                color: Some(theme.palette().text.scale_alpha(0.7)),
                            }
                        }),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    skill_badge(
                        match detail.kind {
                            CatalogSkillKind::Recommended => "Featured",
                            CatalogSkillKind::System => "Built-in",
                            CatalogSkillKind::Personal => "Local",
                        },
                        matches!(detail.kind, CatalogSkillKind::Recommended),
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![
                    settings_value_badge(source_badge),
                    settings_value_badge(if detail.installed { "已安装" } else { "未安装" }),
                    settings_value_badge(if detail.enabled { "已启用" } else { "已禁用" }),
                    skill_badge(detail.document_name.clone(), true),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                detail_source_note(detail.source_path.as_deref()),
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
                            .wrapping(iced::widget::text::Wrapping::Word),
                    )
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
            .push(container(actions).width(Length::Fill))
            .push(document)
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        return container(content).width(Length::Fill).height(Length::Fill).into();
    }

    empty_state(
        "加载技能详情中",
        "点击左侧技能卡片后，这里会显示 SKILL.md 或 SKILL.toml 内容，并提供启用、禁用、删除操作。",
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
    let s = &app.skills_settings;
    if s.selected_skill_id.is_none() {
        return base;
    }

    let close_message = Message::Settings(message::SettingsMessage::SkillsDetailClosed);
    let title =
        s.selected_skill_detail.as_ref().map(|detail| detail.title.as_str()).unwrap_or("技能详情");
    let subtitle = s
        .selected_skill_detail
        .as_ref()
        .map(|detail| match detail.source.as_str() {
            "workspace" => "项目目录技能",
            "ancestor" => "父级目录技能",
            "global" => "全局目录技能",
            "bundled" => "内置技能",
            _ => "技能",
        })
        .unwrap_or("读取技能文档中");

    let card = settings_modal_card(
        column![
            row![
                column![
                    text(title).size(16),
                    text(subtitle).size(12).style(|theme: &iced::Theme| {
                        iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.62)),
                        }
                    }),
                ]
                .spacing(4)
                .width(Length::Fill),
                Space::new().width(Length::Shrink),
                settings_close_button(close_message.clone()),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            detail_modal_body(app, app.project_path.is_some()),
        ]
        .spacing(14)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fixed(860.0))
    .height(Length::Fixed(640.0));

    settings_modal_overlay(Some(base), close_message, card)
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
    let s = &app.skills_settings;
    let project_open = app.project_path.is_some();
    let all_skills = &s.catalog;
    let skills = s
        .catalog
        .iter()
        .filter(|skill| catalog_matches_query(skill, s.query.trim()))
        .filter(|skill| scope_source_matches(s.directory_scope, &skill.source))
        .cloned()
        .collect::<Vec<_>>();

    let search_input = text_input("搜索技能", &s.query)
        .on_input(|value| Message::Settings(message::SettingsMessage::SkillsQueryChanged(value)))
        .padding([8, 0])
        .size(13)
        .style(search_text_input_style)
        .width(Length::Fill);
    let project_options = skill_project_options(app);
    let project_switcher: Element<'_, Message> = if project_options.is_empty() {
        text("尚未打开项目；当前仅显示全局与内置技能。")
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        row![
            text("项目目录").size(13).width(Length::Fixed(84.0)),
            pick_list(project_options, app.project_path.clone(), |path| {
                Message::Settings(message::SettingsMessage::SkillsProjectSelected(path))
            })
            .placeholder("选择项目目录")
            .padding([9, 12])
            .text_size(12)
            .width(Length::Fill)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style),
            settings_value_badge(if s.loading { "切换中" } else { "技能作用域" }),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    };

    let header = container(
        column![
            row![
                column![
                    text("技能").size(22),
                    text(scope_description(s.directory_scope, s.directory_provider))
                        .size(13)
                        .style(|theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.68)),
                        }),
                ]
                .spacing(6)
                .width(Length::Fill),
                refresh_button(s.loading),
                skill_badge(format!("{} 项技能", skills.len()), true),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            container(
                row![icon_svg(Icon::Search, 14.0), search_input]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .padding([0, 4])
            .width(Length::Fill)
            .style(search_bar_style),
            project_switcher,
            row![
                scope_button(
                    "项目目录",
                    SkillsDirectoryScope::Project,
                    s.directory_scope,
                    project_open,
                ),
                scope_button(
                    "父级目录",
                    SkillsDirectoryScope::Ancestor,
                    s.directory_scope,
                    project_open,
                ),
                scope_button("全局目录", SkillsDirectoryScope::Global, s.directory_scope, true),
                scope_button("内置技能", SkillsDirectoryScope::Bundled, s.directory_scope, true),
                scope_button("全部目录", SkillsDirectoryScope::All, s.directory_scope, true),
                Space::new().width(Length::Fill),
                settings_value_badge(active_scope_badge(s.directory_scope)),
                settings_value_badge(if project_open {
                    "项目已打开"
                } else {
                    "未打开项目"
                }),
                settings_value_badge(format!("总目录项 {}", all_skills.len())),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(12),
    )
    .padding([18, 20])
    .width(Length::Fill)
    .style(header_panel_style);

    let mut content = column![header].spacing(16).width(Length::Fill);

    if let Some(message) = &s.status_message {
        content = content.push(status_banner(message, s.status_is_error));
    }

    let mut catalog_panel = column![
        row![
            column![
                text(scope_title(s.directory_scope)).size(18),
                text(scope_description(s.directory_scope, s.directory_provider)).size(12).style(
                    |theme: &iced::Theme| {
                        iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.64)),
                        }
                    }
                ),
            ]
            .spacing(4)
            .width(Length::Fill),
            settings_value_badge(format!("{} results", skills.len())),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(16)
    .width(Length::Fill);

    if s.loading {
        catalog_panel = catalog_panel.push(loading_banner());
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
            "workspace",
            workspace,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !ancestor.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            "ancestor",
            ancestor,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !global.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            "global",
            global,
            s.selected_skill_id.as_deref(),
        ));
    }
    if !bundled.is_empty() {
        catalog_panel = catalog_panel.push(catalog_group_section(
            "bundled",
            bundled,
            s.selected_skill_id.as_deref(),
        ));
    }

    if skills.is_empty() && !s.loading {
        let (title, description) = match (s.directory_scope, project_open) {
            (SkillsDirectoryScope::Project, false) => {
                ("未打开项目目录", "打开项目后可查看当前工程的 skills 目录，或先切换到全部目录。")
            }
            (SkillsDirectoryScope::Project, true) => (
                "项目目录下没有技能",
                "当前项目目录没有命中技能，可以切换到全部目录查看父级、全局和内置技能。",
            ),
            (SkillsDirectoryScope::Ancestor, false) => {
                ("未打开项目目录", "打开项目后可查看父级技能目录。")
            }
            (SkillsDirectoryScope::Ancestor, true) => {
                ("父级目录下没有技能", "项目父级目录没有命中技能，可以切换到全局目录或内置技能。")
            }
            (SkillsDirectoryScope::Global, _) => {
                ("全局目录下没有技能", "当前目录提供方的全局技能目录没有命中技能。")
            }
            (SkillsDirectoryScope::Bundled, _) => {
                ("没有内置技能", "内置技能目录为空，或当前搜索关键字没有匹配结果。")
            }
            (SkillsDirectoryScope::All, _) => {
                ("没有匹配的技能", "试试清空搜索关键字，或稍后刷新技能目录。")
            }
        };
        catalog_panel = catalog_panel.push(empty_state(title, description));
    }

    let catalog_container =
        container(catalog_panel).padding([20, 20]).width(Length::Fill).style(catalog_panel_style);

    content = content.push(catalog_container);

    content.into()
}
