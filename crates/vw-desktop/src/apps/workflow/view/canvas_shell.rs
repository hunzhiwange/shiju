//! 工作流画布外壳视图模块，负责组合画布区域、悬浮工具栏、上下文菜单和快速插入面板。

use super::*;
use iced::widget::{column, mouse_area, row};

/// 构建 status chip 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_status_chip<'a>(status: &'a str) -> Element<'a, Message> {
    container(text(status).size(12)).padding([8, 12]).style(floating_panel_style).into()
}

/// 构建 canvas area 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_canvas_area<'a>(
    state: &'a WorkflowState,
    canvas_base: Element<'a, Message>,
) -> Element<'a, Message> {
    let zoom_presets = [10u32, 20, 40, 70, 80, 100, 125, 150, 175, 200, 250, 300, 400];
    let mut layers = vec![canvas_base];

    if state.has_apps() {
        layers.push(
            container(build_action_bar(state))
                .padding(iced::Padding {
                    top: FLOATING_MARGIN,
                    right: 0.0,
                    bottom: 0.0,
                    left: FLOATING_MARGIN,
                })
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Top)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );

        layers.push(
            container(build_center_toolbar(state))
                .padding(iced::Padding { top: FLOATING_MARGIN, right: 0.0, bottom: 0.0, left: 0.0 })
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Top)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );

        if state.quick_insert_panel_open {
            layers.push(
                container(build_quick_insert_panel(state))
                    .padding(iced::Padding {
                        top: FLOATING_MARGIN + TOOLBAR_HEIGHT + QUICK_INSERT_GAP,
                        right: 0.0,
                        bottom: 0.0,
                        left: 0.0,
                    })
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Top)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
        }

        layers.push(
            container(build_zoom_control(state))
                .padding(iced::Padding {
                    top: FLOATING_MARGIN,
                    right: FLOATING_MARGIN,
                    bottom: 0.0,
                    left: 0.0,
                })
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );

        layers.push(
            container(build_app_switcher_dock(state))
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: FLOATING_MARGIN,
                    left: FLOATING_MARGIN,
                })
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Bottom)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );
    }

    if let Some(status) = &state.status_message {
        layers.push(
            container(build_status_chip(status))
                .padding(iced::Padding { top: 0.0, right: 0.0, bottom: FLOATING_MARGIN, left: 0.0 })
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Bottom)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );
    }

    if state.context_menu.is_some() {
        layers.push(build_canvas_context_menu_overlay(state));
    }

    let canvas_with_ui: Element<'a, Message> =
        stack(layers).width(Length::Fill).height(Length::Fill).into();

    let mut canvas_with_overlays = if state.action_menu_open {
        PointBelowOverlay::new(canvas_with_ui, build_action_menu_overlay(state))
            .show(true)
            .anchor(Point::new(FLOATING_MARGIN, FLOATING_MARGIN + ACTION_BAR_HEIGHT))
            .gap(ACTION_MENU_GAP)
            .on_close(Message::WorkflowTool(WorkflowMessage::CloseFloatingPanels))
            .into()
    } else {
        canvas_with_ui
    };

    if state.zoom_menu_open {
        canvas_with_overlays = PointBelowOverlay::new(
            canvas_with_overlays,
            build_zoom_menu_overlay(state, &zoom_presets),
        )
        .show(true)
        .anchor(Point::new(100_000.0, FLOATING_MARGIN + ZOOM_CONTROL_HEIGHT))
        .gap(ZOOM_MENU_GAP)
        .on_close(Message::WorkflowTool(WorkflowMessage::CloseFloatingPanels))
        .into();
    }

    canvas_with_overlays
}

/// 构建 left toolbar overlay 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
#[allow(dead_code)]
pub(super) fn build_left_toolbar_overlay(state: &WorkflowState) -> Element<'_, Message> {
    let toolbar = container(
        column![
            toolbar_icon_button(
                Icon::Plus,
                if state.quick_insert_panel_open { "收起插入菜单" } else { "插入节点" },
                WorkflowMessage::ToggleQuickInsertPanel,
                state.quick_insert_panel_open,
            ),
            toolbar_icon_button(
                Icon::Sliders,
                "环境变量",
                WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::Environment),
                variable_panel_is_open(state, WorkflowVariablePanelKind::Environment),
            ),
            toolbar_icon_button(
                Icon::ChatTextFill,
                "会话变量",
                WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::Conversation),
                variable_panel_is_open(state, WorkflowVariablePanelKind::Conversation),
            ),
            toolbar_icon_button(
                Icon::Gear,
                "系统变量",
                WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::System),
                variable_panel_is_open(state, WorkflowVariablePanelKind::System),
            ),
            toolbar_icon_button(
                Icon::ArrowsFullscreen,
                "适配视图",
                WorkflowMessage::ZoomFit,
                false
            ),
        ]
        .spacing(8),
    )
    .padding([12, 10])
    .style(modal_card_style);

    let mut overlay = row![toolbar].spacing(12).align_y(Alignment::Start);
    if state.quick_insert_panel_open {
        overlay = overlay.push(build_quick_insert_panel(state));
    }

    container(overlay)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding { top: 18.0, right: 0.0, bottom: 0.0, left: 18.0 })
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Top)
        .into()
}

/// 构建 canvas context menu overlay 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_canvas_context_menu_overlay(state: &WorkflowState) -> Element<'_, Message> {
    let Some(context_menu) = state.context_menu.as_ref() else {
        return container(Space::new().width(1).height(1)).into();
    };

    let menu_card: Element<'static, Message> = match &context_menu.target {
        WorkflowCanvasContextMenuTarget::Canvas => {
            build_context_node_picker_menu(state, "新增节点", "点击后直接放到右键位置", false)
        }
        WorkflowCanvasContextMenuTarget::Edge(_) => container(
            button(text("删除连线").size(12))
                .style(danger_action_btn_style)
                .padding([8, 10])
                .width(Length::Fill)
                .on_press(Message::WorkflowTool(WorkflowMessage::DeleteSelectedEdge)),
        )
        .width(Length::Fixed(208.0))
        .padding([10, 10])
        .style(modal_card_style)
        .into(),
        WorkflowCanvasContextMenuTarget::Node(node_id) => {
            let mut actions = column![context_menu_button(
                "编辑节点",
                WorkflowMessage::OpenEditNodeEditor(Some(node_id.clone())),
            )]
            .spacing(6);

            if state.document.node(node_id).is_some_and(|node| node.block_type != "start") {
                actions = actions
                    .push(context_menu_button("复制节点", WorkflowMessage::DuplicateSelectedNode));
            }

            actions = actions.push(context_menu_button(
                "新增下游节点",
                WorkflowMessage::OpenDownstreamNodePicker(node_id.clone()),
            ));
            actions =
                actions.push(context_menu_button("删除节点", WorkflowMessage::DeleteSelectedNode));

            container(actions)
                .width(Length::Fixed(208.0))
                .padding([10, 10])
                .style(modal_card_style)
                .into()
        }
        WorkflowCanvasContextMenuTarget::NodeInsert(_) => {
            build_context_node_picker_menu(state, "新增下游节点", "点击后自动关联到当前节点", true)
        }
    };

    let menu_card = mouse_area(menu_card)
        .on_scroll(|_| Message::WorkflowTool(WorkflowMessage::CanvasContextMenuScrollCaptured));

    container(
        column![
            Space::new().height(Length::Fixed(context_menu.anchor.y)),
            row![
                Space::new().width(Length::Fixed(context_menu.anchor.x)),
                menu_card,
                Space::new().width(Length::Fill),
            ]
            .width(Length::Fill),
            Space::new().height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// 提供 variable panel is open 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn variable_panel_is_open(
    state: &WorkflowState,
    kind: WorkflowVariablePanelKind,
) -> bool {
    state.variable_panel.as_ref() == Some(&kind)
}

/// 提供 toolbar icon button 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
#[allow(dead_code)]
pub(super) fn toolbar_icon_button(
    icon: Icon,
    tooltip: &'static str,
    message: WorkflowMessage,
    active: bool,
) -> Element<'static, Message> {
    let icon_color_active = active;
    let button_size = 40.0;
    let icon_element = svg(assets::get_icon(icon))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(move |theme: &Theme, _status| {
            let palette = theme.extended_palette();
            iced::widget::svg::Style {
                color: Some(if icon_color_active {
                    palette.primary.strong.text
                } else {
                    palette.background.base.text
                }),
            }
        });

    let button = button(
        container(icon_element)
            .width(Length::Fixed(button_size))
            .height(Length::Fixed(button_size))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .style(if active { primary_action_btn_style } else { rounded_action_btn_style })
    .padding(0)
    .width(Length::Fixed(button_size))
    .height(Length::Fixed(button_size))
    .on_press(Message::WorkflowTool(message));

    Tooltip::new(button, toolbar_tooltip_bubble(tooltip), TooltipPosition::Right).gap(8.0).into()
}

/// 提供 toolbar tooltip bubble 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn toolbar_tooltip_bubble<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(12).color(Color::WHITE))
        .padding([6, 10])
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Color::from_rgba8(24, 24, 24, 0.96).into()),
            text_color: Some(Color::WHITE),
            border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
            shadow: Shadow {
                color: Color::BLACK.scale_alpha(0.40),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            },
            ..Default::default()
        })
        .into()
}

/// 提供 available node types 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn available_node_types(
    state: &WorkflowState,
    exclude_start: bool,
) -> Vec<WorkflowNodeTypeDescriptor> {
    let has_start_node = state.has_start_node();

    supported_node_types()
        .iter()
        .copied()
        .filter(|node_type| {
            if node_type.block_type == "start" { !exclude_start && !has_start_node } else { true }
        })
        .collect()
}

/// 构建 quick insert panel 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_quick_insert_panel(state: &WorkflowState) -> Element<'static, Message> {
    let available_node_types = available_node_types(state, false);
    let mut grid = row![].spacing(10);

    for node_type in available_node_types.iter().copied() {
        grid = grid.push(quick_insert_node_button(node_type));
    }

    let grid = grid.wrap();

    container(
        column![
            row![
                column![
                    text("插入节点").size(16),
                    text("点击后直接放到当前画布中心附近，细节配置后续再编辑节点。")
                        .size(11)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                Space::new().width(Length::Fill),
                settings_value_badge(format!("{} 种", available_node_types.len())),
            ]
            .align_y(Alignment::Center),
            container(
                scrollable(container(grid).width(Length::Fill))
                    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                    .width(Length::Fill)
                    .height(Length::Fixed(304.0)),
            ),
        ]
        .spacing(14),
    )
    .width(Length::Fixed(438.0))
    .padding([14, 16])
    .style(floating_panel_style)
    .into()
}

/// 提供 quick insert node button 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn quick_insert_node_button(
    node_type: WorkflowNodeTypeDescriptor,
) -> Element<'static, Message> {
    let accent = workflow_node_accent_color(node_type.block_type);
    button(
        row![
            workflow_node_icon_badge(node_type.icon, accent, 16.0),
            column![
                text(node_type.label).size(13),
                text(node_type.summary).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .style(rounded_action_btn_style)
    .padding([12, 12])
    .width(Length::Fixed(136.0))
    .on_press(Message::WorkflowTool(WorkflowMessage::InsertSuggestedNode(
        node_type.block_type.to_string(),
    )))
    .into()
}

#[cfg(test)]
#[path = "canvas_shell_tests.rs"]
mod canvas_shell_tests;
