//! 工作流顶部栏视图模块，负责渲染标题、运行状态、操作按钮和更多操作浮层。

use super::*;
use iced::widget::{column, row};

/// 构建 header 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
#[allow(dead_code)]
pub(super) fn build_header(state: &WorkflowState) -> Element<'_, Message> {
    let reload_label = "重新载入";
    let description = if !state.has_apps() {
        String::new()
    } else if let Some(path) = state.source_path.as_deref() {
        format!("Dify DSL 源文件：{}{}", path, if state.active_is_dirty { " · 未保存" } else { "" })
    } else if state.active_is_dirty {
        "当前为未保存草稿，保存后会落到本地 yml。".to_string()
    } else {
        "当前应用来自本地数据库，可继续编辑并保存。".to_string()
    };
    let app_switcher = build_app_switcher(state);

    let overview_badges = row![
        settings_value_badge(format!("{} 个应用", state.apps.len())),
        settings_value_badge(format!("{} 节点", state.document.nodes.len())),
        settings_value_badge(format!("{} 连线", state.document.edges.len())),
        settings_value_badge(if state.active_is_dirty { "草稿待保存" } else { "已同步" }),
    ]
    .spacing(8)
    .wrap();

    let primary_actions = row![
        button(text("新增应用").size(12))
            .style(primary_action_btn_style)
            .padding([9, 14])
            .on_press(Message::WorkflowTool(WorkflowMessage::OpenCreateAppEditor)),
        button(text("导入 DSL").size(12))
            .style(rounded_action_btn_style)
            .padding([9, 14])
            .on_press(Message::WorkflowTool(WorkflowMessage::OpenFile)),
        button(text("保存 yml").size(12))
            .style(rounded_action_btn_style)
            .padding([9, 14])
            .on_press_maybe(
                state
                    .active_app_id
                    .as_ref()
                    .map(|_| Message::WorkflowTool(WorkflowMessage::SaveActiveApp)),
            ),
        button(text("另存为").size(12))
            .style(rounded_action_btn_style)
            .padding([9, 14])
            .on_press_maybe(
                state
                    .active_app_id
                    .as_ref()
                    .map(|_| Message::WorkflowTool(WorkflowMessage::SaveActiveAppAs)),
            ),
    ]
    .spacing(8)
    .wrap();

    let maintenance_actions = row![
        button(text("编辑应用").size(12))
            .style(rounded_action_btn_style)
            .padding([8, 12])
            .on_press_maybe(
                state
                    .active_app_id
                    .as_ref()
                    .map(|_| Message::WorkflowTool(WorkflowMessage::OpenEditAppEditor(None))),
            ),
        button(text("编辑节点").size(12))
            .style(rounded_action_btn_style)
            .padding([8, 12])
            .on_press_maybe(
                state
                    .selected_node_id
                    .as_ref()
                    .map(|_| Message::WorkflowTool(WorkflowMessage::OpenEditNodeEditor(None))),
            ),
        button(text(reload_label).size(12))
            .style(rounded_action_btn_style)
            .padding([8, 12])
            .on_press_maybe(
                state
                    .active_app_id
                    .as_ref()
                    .map(|_| Message::WorkflowTool(WorkflowMessage::Reload)),
            ),
        button(text("适配视图").size(12))
            .style(rounded_action_btn_style)
            .padding([8, 12])
            .on_press(Message::WorkflowTool(WorkflowMessage::ZoomFit)),
        if state.connection_draft.is_some() {
            button(text("取消连线").size(12))
                .style(rounded_action_btn_style)
                .padding([8, 12])
                .on_press(Message::WorkflowTool(WorkflowMessage::CancelConnection))
        } else if state.selected_node_id.is_some() {
            button(text("删除节点").size(12))
                .style(danger_action_btn_style)
                .padding([8, 12])
                .on_press(Message::WorkflowTool(WorkflowMessage::DeleteSelectedNode))
        } else {
            button(text("删除连线").size(12))
                .style(danger_action_btn_style)
                .padding([8, 12])
                .on_press_maybe(
                    state
                        .selected_edge_id
                        .as_ref()
                        .map(|_| Message::WorkflowTool(WorkflowMessage::DeleteSelectedEdge)),
                )
        },
        button(text("−").size(13))
            .style(rounded_action_btn_style)
            .padding([8, 10])
            .on_press(Message::WorkflowTool(WorkflowMessage::Zoom(1.0 / 1.1, None))),
        settings_value_badge(format!("{:.0}%", state.zoom.max(0.1) * 100.0)),
        button(text("+").size(13))
            .style(rounded_action_btn_style)
            .padding([8, 10])
            .on_press(Message::WorkflowTool(WorkflowMessage::Zoom(1.1, None))),
    ]
    .spacing(8)
    .wrap();

    container(
        row![
            column![
                {
                    let mut summary = column![app_switcher].spacing(8);

                    if !description.is_empty() {
                        summary = summary
                            .push(text(description).size(12).style(settings_muted_text_style));
                    }

                    summary
                },
                overview_badges,
            ]
            .spacing(10)
            .width(Length::FillPortion(2)),
            column![primary_actions, maintenance_actions].spacing(10).width(Length::FillPortion(3)),
        ]
        .spacing(18)
        .align_y(Alignment::Start),
    )
    .padding([18, 20])
    .style(settings_panel_style)
    .into()
}

/// 构建 action bar 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_action_bar(state: &WorkflowState) -> Element<'static, Message> {
    let icon_button = |icon: Icon,
                       tooltip: &'static str,
                       message: Option<WorkflowMessage>,
                       active: bool|
     -> Element<'static, Message> {
        let enabled = message.is_some();
        let icon_element: Element<'static, Message> = svg(assets::get_icon(icon))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .content_fit(iced::ContentFit::Contain)
            .style(move |theme: &Theme, _| action_bar_icon_style(theme, enabled, active))
            .into();

        let button = button(
            container(icon_element)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .width(Length::Fixed(ACTION_BAR_BUTTON_SIZE))
        .height(Length::Fixed(ACTION_BAR_BUTTON_SIZE))
        .style(move |theme: &Theme, status| {
            action_bar_button_style(theme, status, enabled, active)
        });

        let button: Element<'static, Message> = if let Some(message) = message {
            button.on_press(Message::WorkflowTool(message)).into()
        } else {
            button.into()
        };

        Tooltip::new(button, toolbar_tooltip_bubble(tooltip), TooltipPosition::Bottom)
            .gap(8.0)
            .into()
    };

    container(
        row![
            icon_button(
                Icon::LayoutSidebar,
                if state.action_menu_open { "收起菜单" } else { "打开菜单" },
                Some(WorkflowMessage::ToggleActionMenu),
                state.action_menu_open,
            ),
            icon_button(
                Icon::ArrowCounterClockwise,
                "撤销",
                (!state.undo_stack.is_empty()).then_some(WorkflowMessage::Undo),
                false,
            ),
            icon_button(
                Icon::ArrowClockwise,
                "重做",
                (!state.redo_stack.is_empty()).then_some(WorkflowMessage::Redo),
                false,
            ),
            icon_button(
                Icon::ArrowRepeat,
                "重新载入",
                state.active_app_id.as_ref().map(|_| WorkflowMessage::Reload),
                false,
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding(ACTION_BAR_PADDING)
    .style(floating_panel_style)
    .into()
}

fn action_bar_icon_style(theme: &Theme, enabled: bool, active: bool) -> iced::widget::svg::Style {
    let color = if !enabled {
        Color::from_rgba8(160, 160, 160, 1.0)
    } else if active {
        theme.palette().primary
    } else {
        theme.palette().text
    };
    iced::widget::svg::Style { color: Some(color) }
}

fn action_bar_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
    active: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = if !enabled {
        None
    } else if active {
        Some(Background::Color(theme.palette().primary.scale_alpha(0.14)))
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
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        text_color: if enabled {
            theme.palette().text
        } else {
            Color::from_rgba8(160, 160, 160, 1.0)
        },
        ..Default::default()
    }
}

/// 构建 action menu overlay 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_action_menu_overlay(state: &WorkflowState) -> Element<'static, Message> {
    let menu_item = |icon: Icon,
                     label: &'static str,
                     message: Option<WorkflowMessage>|
     -> Element<'static, Message> {
        let enabled = message.is_some();

        let icon_el = svg(assets::get_icon(icon))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .content_fit(iced::ContentFit::Contain)
            .style(move |theme: &Theme, _| action_menu_icon_style(theme, enabled));

        let button = button(
            container(
                row![
                    container(icon_el)
                        .width(Length::Fixed(22.0))
                        .align_x(iced::alignment::Horizontal::Center),
                    text(label).size(13),
                    Space::new().width(Length::Fill),
                ]
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([6, 10]),
        )
        .style(move |theme: &Theme, status| action_menu_button_style(theme, status, enabled))
        .width(Length::Fill);

        if let Some(message) = message {
            button.on_press(Message::WorkflowTool(message)).into()
        } else {
            button.into()
        }
    };

    let has_active_app = state.active_app_id.is_some();

    container(
        column![
            menu_item(Icon::Grid1x2, "应用列表", Some(WorkflowMessage::ShowSavedApps)),
            menu_item(
                Icon::FileEarmarkPlus,
                "新增应用",
                Some(WorkflowMessage::OpenCreateAppEditor)
            ),
            menu_item(Icon::FolderOpen, "导入 DSL", Some(WorkflowMessage::OpenFile)),
            menu_item(
                Icon::Save,
                "保存 YAML",
                has_active_app.then_some(WorkflowMessage::SaveActiveApp)
            ),
            menu_item(
                Icon::Save,
                "另存为",
                has_active_app.then_some(WorkflowMessage::SaveActiveAppAs)
            ),
            menu_item(
                Icon::ArrowRepeat,
                "重新载入",
                has_active_app.then_some(WorkflowMessage::Reload)
            ),
            menu_item(
                Icon::CloudDownload,
                "导出 PNG",
                has_active_app.then_some(WorkflowMessage::ExportPng)
            ),
            menu_item(
                Icon::CloudDownload,
                "导出 JPEG",
                has_active_app.then_some(WorkflowMessage::ExportJpeg)
            ),
            menu_item(
                Icon::CloudDownload,
                "导出 SVG",
                has_active_app.then_some(WorkflowMessage::ExportSvg)
            ),
        ]
        .spacing(2),
    )
    .padding(12)
    .width(Length::Fixed(240.0))
    .style(floating_panel_style)
    .into()
}

fn action_menu_icon_style(theme: &Theme, enabled: bool) -> iced::widget::svg::Style {
    let color = if enabled { theme.palette().text } else { theme.palette().text.scale_alpha(0.35) };
    iced::widget::svg::Style { color: Some(color) }
}

fn action_menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = if !enabled {
        None
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
        text_color: if enabled {
            theme.palette().text
        } else {
            theme.palette().text.scale_alpha(0.35)
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;
