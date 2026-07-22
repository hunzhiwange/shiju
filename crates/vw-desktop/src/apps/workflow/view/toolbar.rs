//! 工作流工具栏视图模块，负责中心工具栏、缩放控件、缩放菜单和应用切换入口。

use super::*;
use iced::widget::{column, row};

fn mix_color(left: Color, right: Color, factor: f32) -> Color {
    let factor = factor.clamp(0.0, 1.0);
    Color {
        r: left.r + (right.r - left.r) * factor,
        g: left.g + (right.g - left.g) * factor,
        b: left.b + (right.b - left.b) * factor,
        a: left.a + (right.a - left.a) * factor,
    }
}

fn toolbar_icon_style(theme: &Theme, enabled: bool, active: bool) -> iced::widget::svg::Style {
    let palette = theme.extended_palette();
    let color = if !enabled {
        theme.palette().text.scale_alpha(0.35)
    } else if active {
        palette.background.base.text
    } else {
        theme.palette().text
    };

    iced::widget::svg::Style { color: Some(color) }
}

fn toolbar_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
    active: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = if !enabled {
        None
    } else if active {
        match status {
            iced::widget::button::Status::Pressed => Some(Background::Color(mix_color(
                palette.background.strong.color,
                palette.background.base.color,
                0.30,
            ))),
            _ => Some(Background::Color(mix_color(
                palette.background.weak.color,
                palette.background.base.color,
                0.55,
            ))),
        }
    } else {
        match status {
            iced::widget::button::Status::Hovered => {
                Some(Background::Color(palette.background.weak.color))
            }
            iced::widget::button::Status::Pressed => {
                Some(Background::Color(palette.background.strong.color))
            }
            _ => None,
        }
    };

    iced::widget::button::Style {
        background,
        border: Border {
            width: 1.0,
            color: if active { palette.background.strong.color } else { Color::TRANSPARENT },
            radius: 6.0.into(),
        },
        text_color: if enabled {
            theme.palette().text
        } else {
            theme.palette().text.scale_alpha(0.35)
        },
        ..Default::default()
    }
}

fn zoom_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    radius: iced::border::Radius,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Pressed => {
            Some(Background::Color(palette.background.strong.color))
        }
        iced::widget::button::Status::Hovered => {
            Some(Background::Color(palette.background.weak.color))
        }
        _ => None,
    };

    iced::widget::button::Style {
        background,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn zoom_menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = if active {
        Some(Background::Color(theme.palette().primary.scale_alpha(0.14)))
    } else {
        match status {
            iced::widget::button::Status::Hovered => {
                Some(Background::Color(palette.background.weak.color.scale_alpha(0.55)))
            }
            iced::widget::button::Status::Pressed => {
                Some(Background::Color(palette.background.strong.color))
            }
            _ => None,
        }
    };

    iced::widget::button::Style {
        background,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        text_color: if active { theme.palette().primary } else { theme.palette().text },
        ..Default::default()
    }
}

/// 构建 center toolbar 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_center_toolbar(state: &WorkflowState) -> Element<'static, Message> {
    let node_editor_active = state
        .node_editor
        .as_ref()
        .is_some_and(|editor| matches!(editor.mode, WorkflowNodeEditorMode::Edit(_)));
    let app_editor_active = state
        .app_editor
        .as_ref()
        .is_some_and(|editor| matches!(editor.mode, WorkflowAppEditorMode::Edit(_)));

    let tool_button = |icon: Icon,
                       tooltip: &'static str,
                       active: bool,
                       message: Option<WorkflowMessage>|
     -> Element<'static, Message> {
        let enabled = message.is_some();
        let icon_el: Element<'static, Message> = svg(assets::get_icon(icon))
            .width(Length::Fixed(18.0))
            .height(Length::Fixed(18.0))
            .content_fit(iced::ContentFit::Contain)
            .style(move |theme: &Theme, _| toolbar_icon_style(theme, enabled, active))
            .into();

        let button = button(
            container(icon_el)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .style(move |theme: &Theme, status| toolbar_button_style(theme, status, enabled, active));

        let button: Element<'static, Message> = if let Some(message) = message {
            button.on_press(Message::WorkflowTool(message)).into()
        } else {
            button.into()
        };

        Tooltip::new(button, toolbar_tooltip_bubble(tooltip), TooltipPosition::Bottom)
            .gap(8.0)
            .into()
    };

    let divider = || -> Element<'static, Message> {
        container(Space::new().width(Length::Fixed(1.0)))
            .width(Length::Fixed(1.0))
            .height(Length::Fixed(18.0))
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(palette.background.weak.color)),
                    ..Default::default()
                }
            })
            .into()
    };

    container(
        row![
            tool_button(
                Icon::GitBranch,
                if state.quick_insert_panel_open { "收起插入菜单" } else { "插入节点" },
                state.quick_insert_panel_open,
                Some(WorkflowMessage::ToggleQuickInsertPanel),
            ),
            tool_button(
                Icon::Braces,
                "环境变量",
                variable_panel_is_open(state, WorkflowVariablePanelKind::Environment),
                Some(WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::Environment)),
            ),
            tool_button(
                Icon::Journals,
                "会话变量",
                variable_panel_is_open(state, WorkflowVariablePanelKind::Conversation),
                Some(WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::Conversation)),
            ),
            tool_button(
                Icon::GearWideConnected,
                "系统变量",
                variable_panel_is_open(state, WorkflowVariablePanelKind::System),
                Some(WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::System)),
            ),
            divider(),
            tool_button(
                Icon::Pencil,
                "编辑节点",
                node_editor_active,
                state.selected_node_id.as_ref().map(|_| WorkflowMessage::OpenEditNodeEditor(None)),
            ),
            tool_button(
                Icon::LayoutTextWindow,
                "编辑应用",
                app_editor_active,
                state.active_app_id.as_ref().map(|_| WorkflowMessage::OpenEditAppEditor(None)),
            ),
        ]
        .spacing(6)
        .height(Length::Fixed(32.0))
        .align_y(Alignment::Center),
    )
    .width(Length::Fixed(TOOLBAR_WIDTH))
    .height(Length::Fixed(TOOLBAR_HEIGHT))
    .padding(4)
    .style(floating_panel_style)
    .into()
}

/// 构建 zoom control 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_zoom_control(state: &WorkflowState) -> Element<'static, Message> {
    let zoom_label = format!("{:.0}%", (state.zoom * 100.0).round());

    let zoom_minus: Element<'static, Message> = button(
        container(text("-").size(14).line_height(1.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::WorkflowTool(WorkflowMessage::Zoom(1.0 / 1.1, None)))
    .style(move |theme: &Theme, status| {
        zoom_button_style(
            theme,
            status,
            iced::border::Radius {
                top_left: 12.0,
                top_right: 0.0,
                bottom_left: 12.0,
                bottom_right: 0.0,
            },
        )
    })
    .width(Length::Fixed(34.0))
    .height(Length::Fixed(ZOOM_CONTROL_HEIGHT))
    .padding(0)
    .into();

    let zoom_plus: Element<'static, Message> = button(
        container(text("+").size(14).line_height(1.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::WorkflowTool(WorkflowMessage::Zoom(1.1, None)))
    .style(move |theme: &Theme, status| {
        zoom_button_style(
            theme,
            status,
            iced::border::Radius {
                top_left: 0.0,
                top_right: 12.0,
                bottom_left: 0.0,
                bottom_right: 12.0,
            },
        )
    })
    .width(Length::Fixed(34.0))
    .height(Length::Fixed(ZOOM_CONTROL_HEIGHT))
    .padding(0)
    .into();

    let zoom_label_btn: Element<'static, Message> = button(
        container(text(zoom_label).size(12).line_height(1.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::WorkflowTool(WorkflowMessage::ToggleZoomMenu))
    .style(move |theme: &Theme, status| zoom_button_style(theme, status, 0.0.into()))
    .width(Length::Fill)
    .height(Length::Fixed(ZOOM_CONTROL_HEIGHT))
    .padding(0)
    .into();

    let divider = || -> Element<'static, Message> {
        container(Space::new().width(Length::Fixed(1.0)))
            .height(Length::Fixed(ZOOM_CONTROL_HEIGHT))
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(palette.background.strong.color)),
                    ..Default::default()
                }
            })
            .into()
    };

    container(
        row![zoom_minus, divider(), zoom_label_btn, divider(), zoom_plus]
            .spacing(0)
            .align_y(Alignment::Center),
    )
    .width(Length::Fixed(ZOOM_CONTROL_WIDTH))
    .height(Length::Fixed(ZOOM_CONTROL_HEIGHT))
    .style(floating_panel_style)
    .into()
}

/// 构建 zoom menu overlay 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_zoom_menu_overlay(
    state: &WorkflowState,
    zoom_presets: &[u32],
) -> Element<'static, Message> {
    let current_percent = (state.zoom * 100.0).round().clamp(0.0, 10_000.0) as u32;

    let menu_button =
        |label: String, active: bool, message: WorkflowMessage| -> Element<'static, Message> {
            button(container(text(label).size(12)).width(Length::Fill).padding([6, 10]))
                .on_press(Message::WorkflowTool(message))
                .style(move |theme: &Theme, status| zoom_menu_button_style(theme, status, active))
                .width(Length::Fill)
                .into()
        };

    let mut items =
        column![menu_button("适配窗口".to_string(), false, WorkflowMessage::ZoomFit,)].spacing(2);

    for percent in zoom_presets {
        items = items.push(menu_button(
            format!("{}%", percent),
            current_percent == *percent,
            WorkflowMessage::ZoomSet(*percent as f32 / 100.0),
        ));
    }

    container(items)
        .padding(6)
        .width(Length::Fixed(ZOOM_CONTROL_WIDTH))
        .style(floating_panel_style)
        .into()
}

/// 构建 app switcher dock 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_app_switcher_dock(state: &WorkflowState) -> Element<'static, Message> {
    container(build_app_switcher(state)).width(Length::Fixed(300.0)).into()
}

#[cfg(test)]
#[path = "toolbar_tests.rs"]
mod toolbar_tests;
