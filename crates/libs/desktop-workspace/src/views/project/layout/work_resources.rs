//! 工作模式项目资源区：概览、工作空间文件与浏览器停靠占位面。

use std::path::Path;

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Background, Border, Element, Length, Theme};

use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::tooltip::tooltip_dark_style;

use super::styles::work_surface_style;

const FILE_TREE_WIDTH: f32 = 286.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceBodyKind {
    Overview,
    WorkspaceFiles,
    Browser,
}

pub struct OverviewModel {
    pub project_name: String,
    pub project_path: String,
    pub session_title: String,
    pub session_count: usize,
    pub changed_files: usize,
    pub branch: String,
    pub is_git_repo: bool,
}

pub fn active_session_title<'a>(active: Option<&str>, mut sessions: impl Iterator<Item=(&'a str, &'a str)>, locale: config_ui::DesktopLocale) -> String {
    let i18n = desktop_i18n::I18n::new(locale);
    let Some(id) = active else { return i18n.__("暂无活动会话").into(); };
    sessions.find_map(|(key,title)| (key == id).then_some(title.trim())).filter(|title| !title.is_empty()).unwrap_or(i18n.__("未命名会话")).into()
}
pub fn overview_model(project_path: Option<&str>, session_title: String, cached_count: Option<usize>, session_directories: impl Iterator<Item=impl AsRef<str>>, changed_files: usize, branch: Option<&str>, is_git_repo: bool, locale: config_ui::DesktopLocale) -> OverviewModel {
    let i18n = desktop_i18n::I18n::new(locale);
    let path = project_path.unwrap_or_default();
    let project_name = Path::new(path).file_name().and_then(|name| name.to_str()).filter(|name| !name.is_empty()).unwrap_or(i18n.__("未打开项目")).into();
    let session_count = cached_count.unwrap_or_else(|| project_path.map_or(0, |path| session_directories.filter(|directory| directory.as_ref() == path).count()));
    OverviewModel { project_name, project_path:path.into(),session_title,session_count,changed_files,is_git_repo,branch:branch.unwrap_or(i18n.__("未获取")).into() }
}
#[derive(Clone, Copy)]
pub enum ResourceEvent { ModeSelected(ResourceBodyKind), SplitRatioChanged(f32), HalfFullscreenToggled, FullscreenToggled }
fn panel_style(theme: &Theme, mode: ResourceBodyKind) -> container::Style {
    let palette = theme.extended_palette();
    let browser_mode = mode == ResourceBodyKind::Browser;
    container::Style {
        border: Border {
            width: if browser_mode { 0.0 } else { 1.0 },
            color: palette.background.strong.color.scale_alpha(0.62),
            radius: if browser_mode { 0.0 } else { 12.0 }.into(),
        },
        ..work_surface_style(theme)
    }
}

fn card_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.44))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.48),
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

fn selector_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style + Copy {
    move |theme, status| {
        let palette = theme.extended_palette();
        let background = if active {
            Some(palette.primary.weak.color.scale_alpha(0.48))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(palette.background.strong.color.scale_alpha(0.58))
        } else {
            None
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: if active {
                palette.primary.strong.color
            } else {
                palette.background.base.text
            },
            border: Border { radius: 8.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

fn selector_button<Message: Clone + 'static>(
    label: &'static str,
    mode: ResourceBodyKind,
    active: ResourceBodyKind,
    event: fn(ResourceEvent) -> Message,
) -> Element<'static, Message> {
    button(text(label).size(12))
        .on_press(event(ResourceEvent::ModeSelected(mode)))
        .padding([6, 9])
        .style(selector_style(mode == active))
        .into()
}

fn resource_header_icons(fullscreen: bool) -> [Icon; 4] {
    [
        Icon::ArrowLeft,
        Icon::ArrowRight,
        Icon::LayoutTextWindow,
        if fullscreen { Icon::FullscreenExit } else { Icon::Fullscreen },
    ]
}

fn resource_header_icon(icon: Icon) -> svg::Svg<'static> {
    svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(|theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text.scale_alpha(0.86)),
        })
}

fn resource_header_icon_button<Message: Clone + 'static>(
    icon: Icon,
    label: &'static str,
    message: Message,
) -> Element<'static, Message> {
    let control = button(resource_header_icon(icon))
        .on_press(message)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .padding(7)
        .style(selector_style(false));

    Tooltip::new(
        control,
        container(text(label).size(12)).padding([6, 8]).style(tooltip_dark_style),
        TooltipPosition::Top,
    )
    .gap(6)
    .into()
}

fn overview_card<'a, Message: 'a>(label: &'static str, value: impl Into<String>) -> Element<'a, Message> {
    container(
        column![
            text(label).size(11).style(|theme: &Theme| text::Style {
                color: Some(theme.palette().text.scale_alpha(0.56)),
            }),
            text(value.into()).size(14),
        ]
        .spacing(5),
    )
    .width(Length::Fill)
    .padding([11, 12])
    .style(card_style)
    .into()
}

pub fn overview<Message: 'static>(model: OverviewModel, locale: config_ui::DesktopLocale) -> Element<'static, Message> {
    let mut content = column![
        text(model.project_name).size(20),
        text(if model.project_path.is_empty() {
            desktop_i18n::I18n::new(locale).__("请选择项目以查看资源").to_string()
        } else {
            model.project_path
        })
        .size(12)
        .style(|theme: &Theme| text::Style { color: Some(theme.palette().text.scale_alpha(0.58)) }),
        row![
            overview_card(
                desktop_i18n::I18n::new(locale).__("当前会话"),
                model.session_title
            ),
            overview_card(
                desktop_i18n::I18n::new(locale).__("项目会话"),
                model.session_count.to_string()
            ),
        ]
        .spacing(10),
    ]
    .spacing(12)
    .width(Length::Fill);
    if model.is_git_repo {
        content = content.push(
            row![
                overview_card(
                    desktop_i18n::I18n::new(locale).__("Git 分支"),
                    model.branch
                ),
                overview_card(
                    desktop_i18n::I18n::new(locale).__("工作区更改"),
                    model.changed_files.to_string()
                ),
            ]
            .spacing(10),
        );
    }
    content.into()
}

pub fn workspace_files<'a, Message: 'a>(files: Element<'a, Message>, preview: Element<'a, Message>) -> Element<'a, Message> {
    let file_manager = container(files)
        .width(Length::Fixed(FILE_TREE_WIDTH))
        .height(Length::Fill)
        .padding([4, 6]);
    let divider = container(Space::new()).width(Length::Fixed(1.0)).height(Length::Fill).style(
        |theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color.scale_alpha(0.58),
            )),
            ..Default::default()
        },
    );
    let preview =
        container(preview).width(Length::Fill).height(Length::Fill).padding(4);

    row![file_manager, divider, preview].spacing(4).width(Length::Fill).height(Length::Fill).into()
}

pub fn view<'a, Message: Clone + 'static>(resource_mode: ResourceBodyKind, fullscreen: bool, half_fullscreen: bool, split_ratio: f32, locale: config_ui::DesktopLocale, body: Element<'a, Message>, event: fn(ResourceEvent)->Message) -> Element<'a, Message> {
    let [expand_left_icon, contract_right_icon, half_fullscreen_icon, fullscreen_icon] =
        resource_header_icons(fullscreen);
    let selector = row![
        selector_button(
            desktop_i18n::I18n::new(locale).__("概览"),
            ResourceBodyKind::Overview,
            resource_mode, event
        ),
        selector_button(
            desktop_i18n::I18n::new(locale).__("工作空间文件"),
            ResourceBodyKind::WorkspaceFiles,
            resource_mode, event
        ),
        selector_button(
            desktop_i18n::I18n::new(locale).__("浏览器"),
            ResourceBodyKind::Browser,
            resource_mode, event
        ),
        Space::new().width(Length::Fill),
        resource_header_icon_button(
            expand_left_icon,
            desktop_i18n::I18n::new(locale).__("向左扩展"),
            event(ResourceEvent::SplitRatioChanged(split_ratio - 0.05)),
        ),
        resource_header_icon_button(
            contract_right_icon,
            desktop_i18n::I18n::new(locale).__("向右收起"),
            event(ResourceEvent::SplitRatioChanged(split_ratio + 0.05)),
        ),
        resource_header_icon_button(
            half_fullscreen_icon,
            if half_fullscreen {
                desktop_i18n::I18n::new(locale).__("退出半屏")
            } else {
                desktop_i18n::I18n::new(locale).__("半屏")
            },
            event(ResourceEvent::HalfFullscreenToggled),
        ),
        resource_header_icon_button(
            fullscreen_icon,
            if fullscreen {
                desktop_i18n::I18n::new(locale).__("退出全屏")
            } else {
                desktop_i18n::I18n::new(locale).__("全屏")
            },
            event(ResourceEvent::FullscreenToggled),
        ),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    container(
        column![
            container(selector).width(Length::Fill).padding([7, 8]),
            container(body).width(Length::Fill).height(Length::Fill).padding([8, 10]),
        ]
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |theme| panel_style(theme, resource_mode))
    .into()
}

#[cfg(test)]
#[path="work_resources_tests.rs"] mod work_resources_tests;
