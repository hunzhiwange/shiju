//! 工作模式项目资源区：概览、工作空间文件与浏览器停靠占位面。

use std::path::Path;

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Background, Border, Element, Length, Theme};

use crate::app::assets::{self, Icon};
use crate::app::components::{file_tree, preview_panel};
use crate::app::components::input_panel::styles::tooltip_dark_style;
use crate::app::message::ViewMessage;
use crate::app::state::WorkResourceMode;
use crate::app::{App, Message};

const FILE_TREE_WIDTH: f32 = 286.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResourceBodyKind {
    Overview,
    WorkspaceFiles,
    Browser,
}

pub(super) struct OverviewModel {
    pub(super) project_name: String,
    pub(super) project_path: String,
    pub(super) session_title: String,
    pub(super) session_count: usize,
    pub(super) changed_files: usize,
    pub(super) branch: String,
}

pub(super) fn body_kind(app: &App) -> ResourceBodyKind {
    match app.work_resource_mode {
        WorkResourceMode::Overview => ResourceBodyKind::Overview,
        WorkResourceMode::WorkspaceFiles => ResourceBodyKind::WorkspaceFiles,
        WorkResourceMode::Browser => ResourceBodyKind::Browser,
    }
}

fn active_session_title(app: &App) -> String {
    let Some(active_id) = app.active_session_id.as_deref() else {
        return "暂无活动会话".to_string();
    };
    app.sessions
        .iter()
        .chain(app.project_sessions.values().flat_map(|sessions| sessions.iter()))
        .find(|session| session.id == active_id)
        .map(|session| session.title.trim())
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "未命名会话".to_string())
}

pub(super) fn overview_model(app: &App) -> OverviewModel {
    let project_path = app.project_path.clone().unwrap_or_default();
    let project_name = Path::new(&project_path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "未打开项目".to_string());
    let session_count =
        app.project_path.as_ref().and_then(|path| app.project_sessions.get(path)).map_or_else(
            || {
                app.project_path.as_deref().map_or(0, |path| {
                    app.sessions.iter().filter(|session| session.directory == path).count()
                })
            },
            Vec::len,
        );

    OverviewModel {
        project_name,
        project_path,
        session_title: active_session_title(app),
        session_count,
        changed_files: app.git_changed_files.len(),
        branch: app.selected_branch.clone().unwrap_or_else(|| "未获取".to_string()),
    }
}

pub(super) fn panel_style(theme: &Theme, mode: WorkResourceMode) -> container::Style {
    let palette = theme.extended_palette();
    let browser_mode = mode == WorkResourceMode::Browser;
    container::Style {
        background: Some(Background::Color(crate::app::theme_catalog::panel_surface_color(
            theme, 0.985,
        ))),
        border: Border {
            width: if browser_mode { 0.0 } else { 1.0 },
            color: palette.background.strong.color.scale_alpha(0.62),
            radius: if browser_mode { 0.0 } else { 12.0 }.into(),
        },
        ..Default::default()
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

fn selector_button(
    label: &'static str,
    mode: WorkResourceMode,
    active: WorkResourceMode,
) -> Element<'static, Message> {
    button(text(label).size(12))
        .on_press(Message::View(ViewMessage::WorkResourceModeSelected(mode)))
        .padding([6, 9])
        .style(selector_style(mode == active))
        .into()
}

pub(super) fn resource_header_icons(fullscreen: bool) -> [Icon; 4] {
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

fn resource_header_icon_button(
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

fn overview_card<'a>(label: &'static str, value: impl Into<String>) -> Element<'a, Message> {
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

fn overview(app: &App) -> Element<'_, Message> {
    let model = overview_model(app);
    column![
        text(model.project_name).size(20),
        text(if model.project_path.is_empty() {
            "请选择项目以查看资源".to_string()
        } else {
            model.project_path
        })
        .size(12)
        .style(|theme: &Theme| text::Style { color: Some(theme.palette().text.scale_alpha(0.58)) }),
        row![
            overview_card("当前会话", model.session_title),
            overview_card("项目会话", model.session_count.to_string()),
        ]
        .spacing(10),
        row![
            overview_card("Git 分支", model.branch),
            overview_card("工作区更改", model.changed_files.to_string()),
        ]
        .spacing(10),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

fn workspace_files(app: &App) -> Element<'_, Message> {
    let file_manager = container(file_tree::view_file_manager(app))
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
        container(preview_panel::view(app)).width(Length::Fill).height(Length::Fill).padding(4);

    row![file_manager, divider, preview].spacing(4).width(Length::Fill).height(Length::Fill).into()
}

pub(super) fn view(app: &App, corner_radius: f32) -> Element<'_, Message> {
    let resource_mode = app.work_resource_mode;
    let [expand_left_icon, contract_right_icon, half_fullscreen_icon, fullscreen_icon] =
        resource_header_icons(app.work_resource_fullscreen);
    let selector = row![
        selector_button("概览", WorkResourceMode::Overview, app.work_resource_mode),
        selector_button("工作空间文件", WorkResourceMode::WorkspaceFiles, app.work_resource_mode,),
        selector_button("浏览器", WorkResourceMode::Browser, app.work_resource_mode),
        Space::new().width(Length::Fill),
        resource_header_icon_button(
            expand_left_icon,
            "向左扩展",
            Message::View(ViewMessage::WorkSplitRatioChanged(
                app.work_split_ratio - 0.05,
            )),
        ),
        resource_header_icon_button(
            contract_right_icon,
            "向右收起",
            Message::View(ViewMessage::WorkSplitRatioChanged(
                app.work_split_ratio + 0.05,
            )),
        ),
        resource_header_icon_button(
            half_fullscreen_icon,
            if app.work_resource_half_fullscreen { "退出半屏" } else { "半屏" },
            Message::View(ViewMessage::WorkResourceHalfFullscreenToggled),
        ),
        resource_header_icon_button(
            fullscreen_icon,
            if app.work_resource_fullscreen { "退出全屏" } else { "全屏" },
            Message::View(ViewMessage::WorkResourceFullscreenToggled),
        ),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    let body = match body_kind(app) {
        ResourceBodyKind::Overview => overview(app),
        ResourceBodyKind::WorkspaceFiles => workspace_files(app),
        ResourceBodyKind::Browser => {
            crate::app::message::view::panel::work_browser::view(app)
        }
    };
    let _ = corner_radius;

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
