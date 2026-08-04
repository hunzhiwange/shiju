use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, markdown, pick_list, row, scrollable, text,
};
use iced::{Alignment, Element, Font, Length};
use vw_gateway_client::{
    SkillMarketSkillDetailDto, SkillMarketSkillSetDetailDto,
};

use super::install_guide::{
    InstallTargetPresentation, install_project_options,
    install_target_presentations, target_root_label,
};
use super::public_skills::SCROLLBAR_WIDTH;
use super::styles;
use crate::app::components::system_settings_common::{
    settings_pick_list_menu_style, settings_pick_list_style,
};
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage};
use crate::app::skill_hub::{
    LoadState, PublicDetailMode, PublicDetailState, PublicDetailTarget,
    PublicInstallState, PublicSkillMarketState, SkillInstallScope,
    SkillInstallTarget,
};
use crate::app::{App, Message};

fn public_message(message: PublicSkillMessage) -> Message {
    Message::SkillHub(SkillHubMessage::Public(message))
}

fn back_button() -> Element<'static, Message> {
    button(text("← 返回技能").size(13))
        .on_press(public_message(PublicSkillMessage::CloseDetail))
        .padding([7, 11])
        .style(styles::quiet_button_style)
        .into()
}

fn mode_button(
    label: &'static str,
    mode: PublicDetailMode,
    selected: bool,
) -> Element<'static, Message> {
    button(text(label).size(12))
        .on_press(public_message(PublicSkillMessage::SetDetailMode(mode)))
        .padding([7, 12])
        .style(move |theme, status| {
            styles::selected_tab_style(theme, status, selected)
        })
        .into()
}

fn installed_id(target: &PublicDetailTarget) -> String {
    match target {
        PublicDetailTarget::Skill(locator) => format!(
            "{}--{}--{}",
            locator.source, locator.namespace, locator.slug
        ),
        PublicDetailTarget::Suite { source, slug } => {
            format!("{source}--skillset--{slug}")
        }
    }
}

fn installed_in_catalog(
    market: &PublicSkillMarketState,
    target: &PublicDetailTarget,
) -> bool {
    let expected = installed_id(target);
    matches!(
        &market.installed,
        LoadState::Loaded(items)
            if items.iter().any(|item| item.installed && item.id == expected)
    )
}

fn selected_catalog_install(detail: &PublicDetailState) -> bool {
    detail.install_scope == SkillInstallScope::Project
        && detail.selected_install_targets.as_slice() == [SkillInstallTarget::VibeWindow]
}

pub(super) fn detail_scope_action_message(
    detail: &PublicDetailState,
    scope: SkillInstallScope,
    project_available: bool,
) -> Option<Message> {
    (!detail.install_is_busy()
        && detail.install_scope != scope
        && (scope == SkillInstallScope::Global || project_available))
        .then_some(public_message(
            PublicSkillMessage::SelectDetailInstallScope(scope),
        ))
}

pub(super) fn detail_target_action_message(
    detail: &PublicDetailState,
    target: SkillInstallTarget,
) -> Option<Message> {
    let removing_last = detail.selected_install_targets.len() == 1
        && detail
            .selected_install_targets
            .first()
            .is_some_and(|selected| *selected == target);
    (!detail.install_is_busy() && !removing_last).then_some(public_message(
        PublicSkillMessage::ToggleDetailInstallTarget(target),
    ))
}

pub(super) fn detail_project_action_message(
    selected_project_path: Option<&str>,
    install_busy: bool,
    project_path: String,
) -> Option<Message> {
    (!install_busy && selected_project_path != Some(project_path.as_str()))
        .then_some(public_message(
            PublicSkillMessage::SelectDetailInstallProject(project_path),
        ))
}

fn install_button(app: &App, detail: &PublicDetailState) -> Element<'static, Message> {
    let catalog_installed = selected_catalog_install(detail)
        && installed_in_catalog(&app.expert_hub.public_market, &detail.target);
    let project_required = matches!(detail.target, PublicDetailTarget::Suite { .. })
        || detail.install_scope == SkillInstallScope::Project;
    let (label, enabled) = match &detail.install {
        PublicInstallState::Installing { .. } => ("安装中…", false),
        PublicInstallState::Installed { paths } if paths.len() > 1 => ("多平台安装完成", false),
        PublicInstallState::Installed { .. } => ("安装完成", false),
        PublicInstallState::Idle if catalog_installed => ("已安装", false),
        PublicInstallState::Idle
            if project_required
                && detail
                    .selected_install_project_path
                    .as_deref()
                    .is_none_or(|path| path.trim().is_empty()) =>
        {
            ("请先选择项目", false)
        }
        PublicInstallState::Idle => ("安装", true),
    };
    let mut action = button(text(label).size(13))
        .padding([9, 18])
        .style(styles::pill_button_style);
    if enabled {
        action = action.on_press(public_message(PublicSkillMessage::InstallCurrent));
    }
    action.into()
}

fn project_selector(
    detail: &PublicDetailState,
    options: Vec<String>,
) -> Element<'static, Message> {
    let install_busy = detail.install_is_busy();
    let selected_project_path = detail.selected_install_project_path.clone();
    row![
        text("目标项目").size(12),
        pick_list(
            options,
            detail.selected_install_project_path.clone(),
            move |project_path| {
                detail_project_action_message(
                    selected_project_path.as_deref(),
                    install_busy,
                    project_path,
                )
                .unwrap_or(Message::None)
            }
        )
        .placeholder("选择项目")
        .padding([8, 12])
        .text_size(12)
        .width(Length::Fill)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn scope_button(
    detail: &PublicDetailState,
    label: &'static str,
    scope: SkillInstallScope,
    project_available: bool,
) -> Element<'static, Message> {
    let selected = detail.install_scope == scope;
    let mut action = button(text(label).size(12))
        .padding([7, 13])
        .style(move |theme, status| {
            styles::selected_tab_style(theme, status, selected)
        });
    if let Some(message) =
        detail_scope_action_message(detail, scope, project_available)
    {
        action = action.on_press(message);
    }
    action.into()
}

fn install_target_card(
    detail: &PublicDetailState,
    presentation: InstallTargetPresentation,
    directory_name: &str,
) -> Element<'static, Message> {
    let selected = detail
        .selected_install_targets
        .contains(&presentation.target);
    let path = format!(
        "{}/{}",
        target_root_label(presentation.target, detail.install_scope),
        directory_name
    );
    let mut path_text = text(path.clone()).size(11);
    if path.is_ascii() {
        path_text = path_text.font(Font::MONOSPACE);
    }
    let content = column![
        row![
            text(if selected { "✓" } else { "○" }).size(14),
            text(presentation.name).size(13),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
        text(presentation.compatibility)
            .size(11)
            .style(styles::muted_text_style),
        path_text.style(styles::muted_text_style),
    ]
    .spacing(6);
    let mut action = button(content)
        .width(Length::Fill)
        .padding(12)
        .style(move |theme, status| {
            styles::selected_tab_style(theme, status, selected)
        });
    if let Some(message) =
        detail_target_action_message(detail, presentation.target)
    {
        action = action.on_press(message);
    }
    container(action)
        .width(Length::Fixed(340.0))
        .padding(2)
        .style(styles::inset_panel_style)
        .into()
}

fn installed_paths(paths: &[String]) -> Element<'static, Message> {
    let mut items = column![text("安装位置").size(12)].spacing(6);
    for path in paths {
        let mut value = text(path.clone()).size(11);
        if path.is_ascii() {
            value = value.font(Font::MONOSPACE);
        }
        items = items.push(value.style(styles::muted_text_style));
    }
    container(items)
        .width(Length::Fill)
        .padding(12)
        .style(styles::inset_panel_style)
        .into()
}

fn skill_install_options(
    app: &App,
    detail: &PublicDetailState,
) -> Option<Element<'static, Message>> {
    let PublicDetailTarget::Skill(locator) = &detail.target else {
        return None;
    };
    let project_options = install_project_options(app);
    let directory_name = installed_id(&detail.target);
    let target_cards = install_target_presentations(detail.install_scope)
        .into_iter()
        .map(|presentation| {
            install_target_card(detail, presentation, &directory_name)
        })
        .collect::<Vec<_>>();
    let mut content = column![
        row![
            column![
                text("安装到 Agent")
                    .size(18)
                    .font(Font {
                        weight: iced::font::Weight::Semibold,
                        ..Font::default()
                    }),
                text(format!(
                    "为 {} 选择安装范围和一个或多个 Agent 目标。",
                    locator.slug
                ))
                .size(12)
                .style(styles::muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            install_button(app, detail),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        row![
            text("安装范围").size(12),
            scope_button(detail, "全局", SkillInstallScope::Global, true),
            scope_button(
                detail,
                "项目",
                SkillInstallScope::Project,
                !project_options.is_empty(),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(14);
    if detail.install_scope == SkillInstallScope::Project {
        content = content.push(project_selector(detail, project_options));
    }
    content = content.push(row(target_cards).spacing(10).wrap());
    if let PublicInstallState::Installed { paths } = &detail.install {
        content = content.push(installed_paths(paths));
    }
    Some(
        container(content)
            .padding(20)
            .width(Length::Fill)
            .style(styles::detail_panel_style)
            .into(),
    )
}

fn field(label: &'static str, value: String) -> Element<'static, Message> {
    column![
        text(label).size(11).style(styles::muted_text_style),
        text(if value.trim().is_empty() {
            "暂无".to_string()
        } else {
            value
        })
        .size(13),
    ]
    .spacing(4)
    .width(Length::Fixed(190.0))
    .into()
}

fn skill_information(detail: &SkillMarketSkillDetailDto) -> Element<'static, Message> {
    let skill = &detail.skill;
    let security = detail
        .security
        .as_ref()
        .map(|security| {
            security
                .summary
                .clone()
                .unwrap_or_else(|| security.status.clone())
        })
        .unwrap_or_else(|| "未提供".to_string());
    container(
        column![
            text("技能信息")
                .size(18)
                .font(Font {
                    weight: iced::font::Weight::Semibold,
                    ..Font::default()
                }),
            row![
                field("作者", skill.author.clone()),
                field("分类", skill.category.clone()),
                field("版本", skill.coordinate.version.clone()),
                field("来源", skill.coordinate.locator.source.clone()),
            ]
            .spacing(16)
            .wrap(),
            row![
                field("评分", format!("{:.1}", skill.rating)),
                field("下载", skill.downloads.to_string()),
                field("安装", skill.installs.to_string()),
                field("安全", security),
            ]
            .spacing(16)
            .wrap(),
            row![
                field("许可证", detail.license.clone().unwrap_or_default()),
                field(
                    "API Key",
                    if detail.requires_api_key {
                        "需要".to_string()
                    } else {
                        "不需要".to_string()
                    },
                ),
                field("命名空间", skill.coordinate.locator.namespace.clone()),
                field("标识", skill.coordinate.locator.slug.clone()),
            ]
            .spacing(16)
            .wrap(),
        ]
        .spacing(16),
    )
    .padding(20)
    .width(Length::Fill)
    .style(styles::detail_panel_style)
    .into()
}

fn suite_information(detail: &SkillMarketSkillSetDetailDto) -> Element<'static, Message> {
    let mut members = column![].spacing(7);
    for coordinate in &detail.members {
        members = members.push(
            container(
                row![
                    text(format!(
                        "{} / {}",
                        coordinate.locator.namespace, coordinate.locator.slug
                    ))
                    .size(13)
                    .width(Length::Fill),
                    text(if coordinate.version.is_empty() {
                        "最新".to_string()
                    } else {
                        coordinate.version.clone()
                    })
                    .size(11)
                    .style(styles::muted_text_style),
                ]
                .align_y(Alignment::Center),
            )
            .padding([9, 12])
            .width(Length::Fill)
            .style(styles::inset_panel_style),
        );
    }
    if detail.members.is_empty() {
        members =
            members.push(text("套件未声明成员").size(13).style(styles::muted_text_style));
    }
    container(
        column![
            row![
                text("套件成员")
                    .size(18)
                    .font(Font {
                        weight: iced::font::Weight::Semibold,
                        ..Font::default()
                    }),
                container(text(detail.members.len().to_string()).size(11))
                    .padding([4, 8])
                    .style(styles::meta_badge_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            members,
        ]
        .spacing(12),
    )
    .padding(20)
    .width(Length::Fill)
    .style(styles::detail_panel_style)
    .into()
}

fn raw_markdown(content: &str) -> Element<'_, Message> {
    container(
        text(content)
            .size(13)
            .font(Font::default())
            .width(Length::Fill),
    )
    .padding(18)
    .width(Length::Fill)
    .style(styles::inset_panel_style)
    .into()
}

fn rendered_markdown<'a>(
    app: &App,
    detail: &'a PublicDetailState,
) -> Element<'a, Message> {
    let mut markdown_style = markdown::Style::from(&app.theme());
    markdown_style.inline_code_font = Font::default();
    markdown_style.code_block_font = Font::default();
    let settings = markdown::Settings::with_text_size(14, markdown_style);
    markdown::view(detail.markdown_items.iter(), settings)
        .map(|_| Message::None)
        .into()
}

fn content_text(detail: &PublicDetailState) -> Option<&str> {
    match (&detail.target, &detail.skill_file, &detail.suite_detail) {
        (PublicDetailTarget::Skill(_), LoadState::Loaded(file), _) => {
            Some(file.content.as_str())
        }
        (
            PublicDetailTarget::Suite { .. },
            _,
            LoadState::Loaded(suite),
        ) => Some(suite.content.as_str()),
        _ => None,
    }
}

fn content_panel<'a>(
    app: &'a App,
    detail: &'a PublicDetailState,
) -> Element<'a, Message> {
    let body = match content_text(detail) {
        Some(content) if detail.mode == PublicDetailMode::Markdown => {
            raw_markdown(content)
        }
        Some(_) => rendered_markdown(app, detail),
        None => status_panel("内容正在加载…"),
    };
    column![
        row![
            text("技能内容")
                .size(18)
                .font(Font {
                    weight: iced::font::Weight::Semibold,
                    ..Font::default()
                })
                .width(Length::Fill),
            mode_button(
                "可视化",
                PublicDetailMode::Visual,
                detail.mode == PublicDetailMode::Visual,
            ),
            mode_button(
                "Markdown",
                PublicDetailMode::Markdown,
                detail.mode == PublicDetailMode::Markdown,
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        body,
    ]
    .spacing(12)
    .into()
}

fn status_panel(message: &str) -> Element<'_, Message> {
    container(text(message).size(13).style(styles::muted_text_style))
        .padding(20)
        .width(Length::Fill)
        .style(styles::detail_panel_style)
        .into()
}

fn loaded_header(
    app: &App,
    detail: &PublicDetailState,
) -> Option<Element<'static, Message>> {
    let (name, description, source) = match (&detail.skill_detail, &detail.suite_detail) {
        (LoadState::Loaded(skill), _) => (
            skill.skill.name.clone(),
            skill.skill.description.clone(),
            skill.skill.coordinate.locator.source.clone(),
        ),
        (_, LoadState::Loaded(suite)) => (
            suite.skill_set.name.clone(),
            suite.skill_set.description.clone(),
            suite.skill_set.source.clone(),
        ),
        _ => return None,
    };
    let mut header = row![
            column![
                text(name)
                    .size(25)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..Font::default()
                    }),
                text(description).size(13).style(styles::muted_text_style),
                text(format!("来源：{source}"))
                    .size(11)
                    .style(styles::muted_text_style),
            ]
            .spacing(6)
            .width(Length::Fill),
        ]
        .spacing(16)
        .align_y(Alignment::Center);
    if matches!(detail.target, PublicDetailTarget::Suite { .. }) {
        header = header.push(install_button(app, detail));
    }
    Some(header.into())
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let Some(detail) = &app.expert_hub.public_market.detail else {
        return status_panel("请从公开技能列表选择技能或套件");
    };
    let header = loaded_header(app, detail)
        .unwrap_or_else(|| status_panel("详情正在加载…"));
    let information = match (&detail.skill_detail, &detail.suite_detail) {
        (LoadState::Loaded(skill), _) => skill_information(skill),
        (_, LoadState::Loaded(suite)) => suite_information(suite),
        (LoadState::Error(error), _) | (_, LoadState::Error(error)) => {
            status_panel(error)
        }
        _ => status_panel("正在同步详情信息…"),
    };
    let mut page = column![
        back_button(),
        header,
        container(Space::new())
            .height(Length::Fixed(1.0))
            .width(Length::Fill)
            .style(styles::divider_style),
    ];
    if let Some(options) = skill_install_options(app, detail) {
        page = page.push(options);
    }
    page = page
        .push(information)
        .push(content_panel(app, detail))
        .spacing(18)
        .width(Length::Fill);
    scrollable(
        container(page)
            .width(Length::Fill)
            .max_width(1180)
            .center_x(Length::Fill),
    )
    .direction(Direction::Vertical(
        Scrollbar::new()
            .width(SCROLLBAR_WIDTH)
            .scroller_width(SCROLLBAR_WIDTH),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
