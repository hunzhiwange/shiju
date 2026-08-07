//! 发送控制组件模块
//!
//! 本模块提供输入面板底部的发送控制相关UI组件，包括：
//! - 任务池按钮：将输入内容添加到任务池
//! - 发送按钮：发送消息或任务
//! - 取消按钮：取消正在执行的任务
//! - 底部工具栏：整合所有控制按钮的容器
//!
//! 这些组件支持不同的交互状态（启用/禁用、排队模式等），
//! 并提供视觉反馈（悬停、按下状态）和工具提示。

use iced::widget::svg;
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::icon_svg;
use crate::app::components::input_panel::styles::{
    BOTTOM_BAR_CHEVRON_ICON_SIZE, BOTTOM_BAR_ICON_BUTTON_SIZE, BOTTOM_BAR_ICON_SIZE, popover_style,
    round_icon_button_style, selectable_list_button_style, selector_chevron_color,
    selector_label_font, selector_text_color, square_icon_button_style, tooltip_dark_style,
};
use crate::app::components::overlays::AboveOverlay;
use crate::app::state::ChatSendBehavior;
use crate::app::task::TaskSplitMode;
use crate::app::{Message, message};
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

const MAIN_SEND_BUTTON_SIZE: f32 = BOTTOM_BAR_ICON_BUTTON_SIZE;
const MAIN_SEND_ICON_SIZE: f32 = BOTTOM_BAR_ICON_SIZE;
const ATTACH_TO_ACTION_GAP: f32 = 3.0;
const SECONDARY_BUTTON_SIZE: f32 = BOTTOM_BAR_ICON_BUTTON_SIZE;
const SECONDARY_ICON_SIZE: f32 = BOTTOM_BAR_ICON_SIZE;
const SEND_MODE_ICON_SIZE: f32 = BOTTOM_BAR_CHEVRON_ICON_SIZE;

pub(super) fn permission_access_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
    active: bool,
) -> iced::widget::button::Style {
    if !active {
        return square_icon_button_style(theme, status, enabled);
    }

    let is_dark = is_dark_theme(theme);
    let background = match status {
        iced::widget::button::Status::Pressed => {
            if is_dark {
                Color::from_rgba8(231, 233, 237, 1.0)
            } else {
                Color::from_rgba8(10, 11, 16, 1.0)
            }
        }
        iced::widget::button::Status::Hovered => {
            if is_dark {
                Color::from_rgba8(249, 250, 251, 1.0)
            } else {
                Color::from_rgba8(13, 15, 20, 1.0)
            }
        }
        _ => prominent_action_background(theme),
    };
    let border_color = if is_dark {
        Color::from_rgba8(255, 255, 255, 0.18)
    } else {
        Color::from_rgba8(17, 19, 24, 0.12)
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 10.0.into(), width: 1.0, color: border_color },
        text_color: prominent_action_foreground(theme),
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.14 } else { 0.05 }),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 16.0,
        },
        ..Default::default()
    }
}

pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub(super) fn prominent_action_background(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        Color::from_rgba8(243, 244, 246, 1.0)
    } else {
        Color::from_rgba8(17, 19, 24, 1.0)
    }
}

pub(super) fn prominent_action_foreground(theme: &Theme) -> Color {
    if is_dark_theme(theme) { Color::from_rgba8(15, 16, 18, 1.0) } else { Color::WHITE }
}

pub(super) fn utility_cluster_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(crate::app::theme_catalog::input_surface_color(
            theme,
            if is_dark { 0.88 } else { 1.0 },
        ))),
        border: Border { radius: 999.0.into(), width: 0.0, color: Color::TRANSPARENT },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.12 } else { 0.035 }),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 16.0,
        },
        ..Default::default()
    }
}

pub(super) fn prominent_action_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
) -> iced::widget::button::Style {
    let is_dark = is_dark_theme(theme);
    let disabled_bg = if is_dark {
        Color::from_rgba8(24, 25, 29, 0.92)
    } else {
        Color::from_rgba8(247, 248, 250, 1.0)
    };
    let disabled_border = if is_dark {
        Color::from_rgba8(44, 47, 53, 0.95)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };

    let (base_bg, base_border, base_fg) = if enabled {
        (
            prominent_action_background(theme),
            if is_dark {
                Color::from_rgba8(255, 255, 255, 0.18)
            } else {
                Color::from_rgba8(17, 19, 24, 0.12)
            },
            prominent_action_foreground(theme),
        )
    } else {
        (
            disabled_bg,
            disabled_border,
            theme.palette().text.scale_alpha(if is_dark { 0.36 } else { 0.42 }),
        )
    };

    let background = match status {
        iced::widget::button::Status::Pressed => {
            if enabled {
                if is_dark {
                    Color::from_rgba8(231, 233, 237, 1.0)
                } else {
                    Color::from_rgba8(10, 11, 16, 1.0)
                }
            } else if is_dark {
                Color::from_rgba8(30, 32, 37, 0.96)
            } else {
                Color::from_rgba8(238, 241, 245, 1.0)
            }
        }
        iced::widget::button::Status::Hovered => {
            if enabled {
                if is_dark {
                    Color::from_rgba8(249, 250, 251, 1.0)
                } else {
                    Color::from_rgba8(13, 15, 20, 1.0)
                }
            } else if is_dark {
                Color::from_rgba8(28, 29, 34, 0.94)
            } else {
                Color::from_rgba8(242, 244, 247, 1.0)
            }
        }
        _ => base_bg,
    };
    let border_color = match status {
        iced::widget::button::Status::Pressed | iced::widget::button::Status::Hovered => {
            if enabled {
                if is_dark {
                    Color::from_rgba8(255, 255, 255, 0.24)
                } else {
                    Color::from_rgba8(17, 19, 24, 0.18)
                }
            } else {
                base_border
            }
        }
        _ => base_border,
    };
    let shadow = if enabled {
        iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.07 }),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        }
    } else {
        iced::Shadow::default()
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 999.0.into(), width: 1.0, color: border_color },
        text_color: base_fg,
        shadow,
        ..Default::default()
    }
}

pub(super) fn send_behavior_icon(behavior: ChatSendBehavior) -> Icon {
    match behavior {
        ChatSendBehavior::Queue => Icon::ListUl,
        ChatSendBehavior::StopAndSend => Icon::Square,
        ChatSendBehavior::Guide => Icon::ChatTextFill,
    }
}

pub(super) fn send_behavior_popover<'a>(selected: ChatSendBehavior) -> Element<'a, Message> {
    let mut list = iced::widget::column![].spacing(4);
    for behavior in
        [ChatSendBehavior::Queue, ChatSendBehavior::StopAndSend, ChatSendBehavior::Guide]
    {
        let is_selected = behavior == selected;
        let check_icon: Element<'_, Message> = if is_selected {
            icon_svg(Icon::Check, 14.0).into()
        } else {
            Space::new().width(Length::Fixed(14.0)).into()
        };
        let button_content = iced::widget::row![
            icon_svg(send_behavior_icon(behavior), 14.0),
            iced::widget::column![
                text(behavior.label()).size(13).font(selector_label_font()),
                text(behavior.description()).size(11).style(|theme: &Theme| {
                    iced::widget::text::Style { color: Some(selector_text_color(theme, false)) }
                }),
            ]
            .spacing(2)
            .width(Length::Fill),
            check_icon,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let button = button(button_content)
            .padding([10, 12])
            .style(move |theme: &Theme, status| {
                selectable_list_button_style(theme, status, is_selected)
            })
            .on_press(Message::View(message::ViewMessage::SelectChatSendBehavior(behavior)));
        list = list.push(button);
    }

    container(list).style(popover_style).padding(8).width(Length::Fixed(320.0)).into()
}

/// 创建任务池按钮
///
/// 该按钮用于将用户输入的内容添加到任务池中。按钮的可用性和行为
/// 取决于当前是否启用了任务模式以及是否有输入内容。
///
/// # 参数
///
/// * `enabled` - 按钮是否启用（基于是否有输入内容）
/// * `task_mode_enabled` - 是否启用了任务模式
/// * `input_text` - 用户输入的文本内容
/// * `task_mode_priority` - 任务模式的优先级设置
/// * `task_mode_model` - 任务模式的大模型标识符
/// * `task_mode_subtasks` - 任务模式的子任务列表
///
/// # 返回值
///
/// 返回一个包含工具提示的任务池按钮元素
///
/// # 行为说明
///
/// - 当任务模式禁用时，按钮不可点击，提示用户"请开启任务模式"
/// - 当任务模式启用但无输入时，按钮不可点击，提示用户"请先输入任务"
/// - 当任务模式启用且有输入时，按钮可点击，显示"加入任务池"
/// - 点击后会根据任务模式是否启用，发送不同的消息（带选项或不带选项）
pub fn pool_button(
    enabled: bool,
    task_mode_enabled: bool,
    input_text: String,
    task_mode_priority: String,
    task_mode_model: String,
    task_mode_reasoning_effort: Option<ReasoningEffortDto>,
    task_mode_task_split_mode: TaskSplitMode,
    task_mode_subtasks: Vec<String>,
) -> Element<'static, Message> {
    let pool_icon: Element<'_, Message> = icon_svg(Icon::Grid1x2, SECONDARY_ICON_SIZE)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if enabled {
                theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.88 })
            } else {
                theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.36 } else { 0.42 })
            }),
        })
        .into();

    // 创建按钮，设置容器尺寸和对齐方式
    let pool_btn = button(
        container(pool_icon)
            .width(Length::Fixed(SECONDARY_BUTTON_SIZE))
            .height(Length::Fixed(SECONDARY_BUTTON_SIZE))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .style(move |theme: &Theme, status| square_icon_button_style(theme, status, enabled));

    // 根据启用状态和任务模式设置按钮的点击行为
    let pool_btn = if enabled {
        if task_mode_enabled {
            // 任务模式启用：发送带选项的添加任务消息
            pool_btn.on_press(Message::TaskBoard(
                message::TaskBoardMessage::AddTaskFromInputWithOptions {
                    content: input_text,
                    priority: task_mode_priority,
                    model: task_mode_model,
                    reasoning_effort: task_mode_reasoning_effort,
                    task_split_mode: task_mode_task_split_mode,
                    subtasks: task_mode_subtasks,
                },
            ))
        } else {
            // 任务模式未启用：发送简单的添加任务消息
            pool_btn.on_press(Message::TaskBoard(message::TaskBoardMessage::AddTaskFromInput(
                input_text,
            )))
        }
    } else {
        // 未启用时不添加点击事件
        pool_btn
    };

    // 根据当前状态设置工具提示文本
    let pool_tip_label = if !task_mode_enabled {
        "请开启任务模式"
    } else if enabled {
        "加入任务池"
    } else {
        "请先输入任务"
    };

    // 创建工具提示容器
    let pool_tip = container(
        text(pool_tip_label)
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(tooltip_dark_style)
    .padding([6, 8]);

    // 返回带工具提示的按钮
    tooltip(pool_btn, pool_tip, Position::Top).into()
}

pub fn full_access_button(enabled: bool, active: bool) -> Element<'static, Message> {
    let shield_icon: Element<'_, Message> = icon_svg(Icon::ShieldLock, SECONDARY_ICON_SIZE)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if active {
                prominent_action_foreground(theme)
            } else if enabled {
                theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.88 })
            } else {
                theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.36 } else { 0.42 })
            }),
        })
        .into();

    let button_content = container(shield_icon)
        .width(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .height(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let access_button = button(button_content).padding(0).style(move |theme: &Theme, status| {
        permission_access_button_style(theme, status, enabled, active)
    });

    let access_button = if enabled {
        access_button.on_press(Message::Chat(message::ChatMessage::ToggleFullAccessPermission))
    } else {
        access_button
    };

    let tooltip_label = if active {
        "关闭完全访问权限"
    } else if enabled {
        "获取完全访问权限"
    } else {
        "当前上下文不可获取完全访问权限"
    };

    tooltip(
        access_button,
        container(text(tooltip_label).size(12)).style(tooltip_dark_style).padding([6, 8]),
        Position::Top,
    )
    .into()
}

pub fn workflow_mode_button(active: bool) -> Element<'static, Message> {
    let workflow_icon: Element<'_, Message> = icon_svg(Icon::Grid1x2, SECONDARY_ICON_SIZE)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if active {
                prominent_action_foreground(theme)
            } else {
                theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.88 })
            }),
        })
        .into();

    let button_content = container(workflow_icon)
        .width(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .height(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let button = button(button_content)
        .padding(0)
        .style(move |theme: &Theme, status| {
            permission_access_button_style(theme, status, true, active)
        })
        .on_press(Message::Chat(message::ChatMessage::WorkflowModeToggled(!active)));

    let tooltip_label = if active { "关闭工作流执行" } else { "启用工作流执行" };
    tooltip(
        button,
        container(text(tooltip_label).size(12)).style(tooltip_dark_style).padding([6, 8]),
        Position::Top,
    )
    .into()
}

pub fn goal_mode_button(active: bool) -> Element<'static, Message> {
    let goal_icon: Element<'_, Message> = icon_svg(Icon::Star, SECONDARY_ICON_SIZE)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if active {
                prominent_action_foreground(theme)
            } else {
                theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.88 })
            }),
        })
        .into();

    let button_content = container(goal_icon)
        .width(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .height(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let button = button(button_content)
        .padding(0)
        .style(move |theme: &Theme, status| {
            permission_access_button_style(theme, status, true, active)
        })
        .on_press(Message::Chat(message::ChatMessage::GoalModeToggled(!active)));

    let tooltip_label = if active { "关闭目标模式" } else { "启用目标模式" };
    tooltip(
        button,
        container(text(tooltip_label).size(12)).style(tooltip_dark_style).padding([6, 8]),
        Position::Top,
    )
    .into()
}

pub fn subagent_mode_button(active: bool) -> Element<'static, Message> {
    let subagent_icon: Element<'_, Message> = icon_svg(Icon::Robot, SECONDARY_ICON_SIZE)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if active {
                prominent_action_foreground(theme)
            } else {
                theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.88 })
            }),
        })
        .into();

    let button_content = container(subagent_icon)
        .width(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .height(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let button = button(button_content)
        .padding(0)
        .style(move |theme: &Theme, status| {
            permission_access_button_style(theme, status, true, active)
        })
        .on_press(Message::Chat(message::ChatMessage::SubagentModeToggled(!active)));

    let tooltip_label = if active { "关闭子智能体模式" } else { "启用子智能体模式" };
    tooltip(
        button,
        container(text(tooltip_label).size(12)).style(tooltip_dark_style).padding([6, 8]),
        Position::Top,
    )
    .into()
}

/// 创建发送按钮
///
/// 该按钮用于发送消息或队列任务。按钮的外观和行为会根据
/// 当前状态动态调整，支持普通发送和队列发送两种模式。
///
/// # 参数
///
/// * `enabled` - 主按钮是否启用（基于是否有输入内容）
/// * `can_send` - 是否可以发送（基于当前应用状态）
/// * `is_requesting` - 当前会话是否正在请求中
///
/// # 返回值
///
/// 返回一个包含工具提示的发送按钮元素
///
/// # 行为说明
///
/// - 空闲态仅显示旧版发送图标，不暴露模式切换入口
/// - 请求态在右侧提供模式切换入口，并允许通过弹出菜单切换发送行为
/// - 仅在 `can_send` 为 true 时才可点击主按钮
pub fn send_button<'a>(
    app: &'a crate::app::App,
    _enabled: bool,
    can_send: bool,
    is_requesting: bool,
) -> Element<'a, Message> {
    let behavior = app.chat_send_behavior;
    let main_icon = if is_requesting { send_behavior_icon(behavior) } else { Icon::ArrowUp };
    let main_tooltip = if !can_send {
        "请先输入内容"
    } else if is_requesting {
        behavior.description()
    } else {
        "发送消息"
    };

    let main_send_icon: Element<'_, Message> = icon_svg(main_icon, MAIN_SEND_ICON_SIZE)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if can_send {
                prominent_action_foreground(theme)
            } else {
                theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.78 } else { 0.72 })
            }),
        })
        .into();

    let main_button_content = container(main_send_icon)
        .width(Length::Fixed(MAIN_SEND_BUTTON_SIZE))
        .height(Length::Fixed(MAIN_SEND_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let main_send = if can_send {
        button(main_button_content)
            .padding(0)
            .style(move |theme: &Theme, status| prominent_action_style(theme, status, can_send))
            .on_press(Message::Chat(message::ChatMessage::SendPressed))
    } else {
        button(main_button_content)
            .padding(0)
            .style(move |theme: &Theme, status| prominent_action_style(theme, status, can_send))
    };

    let main_send = tooltip(
        main_send,
        container(text(main_tooltip).size(12)).style(tooltip_dark_style).padding([6, 8]),
        Position::Top,
    );

    if !is_requesting {
        return main_send.into();
    }

    let toggle_btn = button(
        container(
            icon_svg(
                if app.show_send_mode_popover { Icon::ChevronUp } else { Icon::ChevronDown },
                SEND_MODE_ICON_SIZE,
            )
            .style(move |theme: &Theme, _| svg::Style {
                color: Some(selector_chevron_color(theme, app.show_send_mode_popover)),
            }),
        )
        .width(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .height(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .style(move |theme: &Theme, status| round_icon_button_style(theme, status, true))
    .on_press(Message::View(message::ViewMessage::ToggleSendModePopover));

    let toggle_btn = tooltip(
        toggle_btn,
        container(
            text(if app.show_send_mode_popover {
                "收起发送方式"
            } else {
                "展开发送方式"
            })
            .size(12),
        )
        .style(tooltip_dark_style)
        .padding([6, 8]),
        Position::Top,
    );

    let split = row![main_send, toggle_btn].spacing(4).align_y(Alignment::Center);
    let popover = send_behavior_popover(behavior);

    AboveOverlay::new(split, popover)
        .show(app.show_send_mode_popover)
        .gap(6.0)
        .on_close(Message::View(message::ViewMessage::CloseSendModePopover))
        .into()
}

/// 创建取消按钮
///
/// 该按钮用于取消正在执行的任务。按钮包含一个白色方形图标，
/// 并带有呼吸动画效果，通过参数控制动画帧。
///
/// # 参数
///
/// * `submit_anim` - 提交动画帧计数器，用于控制内部方形的大小动画
///
/// # 返回值
///
/// 返回一个包含工具提示的取消按钮元素
///
/// # 动画说明
///
/// 按钮内部的方形图标会根据 `submit_anim` 参数产生呼吸效果：
/// - 动画帧除以2后取模3，循环显示三种不同尺寸（10.0、11.0、12.0）
/// - 这种设计提供了视觉反馈，表明任务正在执行中
pub fn cancel_button(submit_anim: u8) -> Element<'static, Message> {
    // 计算动画帧：减慢动画速度
    let stop_anim = submit_anim / 2;

    // 根据动画帧计算方形尺寸，产生呼吸效果
    let stop_size = match stop_anim % 3 {
        0 => 8.0,
        1 => 9.0,
        _ => 10.0,
    };

    // 创建白色方形图标
    let stop_square =
        container(Space::new().width(Length::Fixed(stop_size)).height(Length::Fixed(stop_size)))
            .style(|theme: &Theme| iced::widget::container::Style {
                background: Some(Background::Color(prominent_action_foreground(theme))),
                border: Border { radius: 2.0.into(), width: 0.0, color: Color::TRANSPARENT },
                ..Default::default()
            });

    // 创建按钮，设置容器尺寸和对齐
    let cancel_btn = button(
        container(stop_square)
            .width(Length::Fixed(MAIN_SEND_BUTTON_SIZE))
            .height(Length::Fixed(MAIN_SEND_BUTTON_SIZE))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .style(|theme: &Theme, status| prominent_action_style(theme, status, true))
    .on_press(Message::Chat(message::ChatMessage::CancelPressed));

    // 创建工具提示容器
    let cancel_tip = container(
        text("停止任务")
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(tooltip_dark_style)
    .padding([6, 8]);

    // 返回带工具提示的按钮
    tooltip(cancel_btn, cancel_tip, Position::Top).into()
}

/// 创建底部工具栏
///
/// 该函数组装输入面板底部的所有控制按钮，形成一个完整的工具栏。
/// 根据任务模式的启用状态，工具栏会显示不同的按钮组合。
///
/// # 参数
///
/// * `model_btn` - 模型选择按钮元素
/// * `usage_btn` - 使用情况按钮元素
/// * `attach_btn` - 附件按钮元素
/// * `pool_btn` - 可选的任务池按钮元素
/// * `cancel_btn` - 可选的取消按钮元素
/// * `send_btn` - 可选的发送按钮元素
/// * `task_mode_enabled` - 是否启用了任务模式
///
/// # 返回值
///
/// 返回一个包含所有按钮的底部工具栏元素
///
/// # 布局说明
///
/// 工具栏采用水平布局，从左到右依次为：
/// 1. 完全访问权限按钮（如果提供）
/// 2. 目标模式按钮
/// 3. 工作流按钮
/// 4. 子智能体按钮
/// 5. 代理 / 工具 / 技能 TAB 入口（如果提供）
/// 6. ACP 按钮（如果提供）
/// 7. 推理强度按钮（如果提供）
/// 8. 模型按钮（左侧固定）
/// 9. 弹性空白（占据中间空间）
/// 10. 使用情况按钮
/// 11. 附件按钮
/// 12. 任务池按钮（如果提供）
/// 13. 取消和发送按钮（仅在非任务模式下显示）
pub fn bottom_bar<'a>(
    agent_tab_btn: Option<Element<'a, Message>>,
    tools_tab_btn: Option<Element<'a, Message>>,
    skills_tab_btn: Option<Element<'a, Message>>,
    acp_btn: Option<Element<'a, Message>>,
    reasoning_btn: Option<Element<'a, Message>>,
    permission_btn: Option<Element<'a, Message>>,
    goal_btn: Element<'a, Message>,
    workflow_btn: Element<'a, Message>,
    subagent_btn: Element<'a, Message>,
    model_btn: Element<'a, Message>,
    usage_btn: Element<'a, Message>,
    attach_btn: Element<'a, Message>,
    pool_btn: Option<Element<'a, Message>>,
    cancel_btn: Option<Element<'a, Message>>,
    send_btn: Option<Element<'a, Message>>,
    task_mode_enabled: bool,
) -> Element<'a, Message> {
    // 创建基础布局：完全访问 + 目标 + 工作流 + 子智能体 + 会话 TAB + ACP + 模型 + 弹性空白 + 使用情况
    let mut bar = row![].spacing(4);

    if let Some(permission_btn) = permission_btn {
        bar = bar.push(permission_btn);
    }

    bar = bar.push(goal_btn);
    bar = bar.push(workflow_btn);
    bar = bar.push(subagent_btn);

    if let Some(agent_tab_btn) = agent_tab_btn {
        bar = bar.push(agent_tab_btn);
    }
    if let Some(tools_tab_btn) = tools_tab_btn {
        bar = bar.push(tools_tab_btn);
    }
    if let Some(skills_tab_btn) = skills_tab_btn {
        bar = bar.push(skills_tab_btn);
    }

    if let Some(acp_btn) = acp_btn {
        bar = bar.push(acp_btn);
    }

    if let Some(reasoning_btn) = reasoning_btn {
        bar = bar.push(reasoning_btn);
    }

    bar = bar.push(model_btn);
    bar = bar.push(Space::new().width(Length::Fill));

    let mut attach_btn = Some(attach_btn);
    let mut utility_controls = row![usage_btn].spacing(4).align_y(Alignment::Center);

    if task_mode_enabled {
        if let Some(attach_btn) = attach_btn.take() {
            utility_controls = utility_controls.push(attach_btn);
        }
    }

    if let Some(pool_btn) = pool_btn {
        utility_controls = utility_controls.push(pool_btn);
    }

    bar = bar.push(
        container(utility_controls)
            .padding(iced::Padding { top: 1.0, right: 5.0, bottom: 1.0, left: 5.0 })
            .style(utility_cluster_style),
    );

    // 在非任务模式下，添加取消和发送按钮
    if !task_mode_enabled {
        let mut action_controls = row![].spacing(4).align_y(Alignment::Center);
        let mut has_action_control = false;
        let has_following_action = cancel_btn.is_some() || send_btn.is_some();

        if let Some(attach_btn) = attach_btn.take() {
            let attach_btn = if has_following_action {
                container(attach_btn).padding(iced::Padding {
                    top: 0.0,
                    right: ATTACH_TO_ACTION_GAP,
                    bottom: 0.0,
                    left: 0.0,
                })
            } else {
                container(attach_btn)
            };

            action_controls = action_controls.push(attach_btn);
            has_action_control = true;
        }

        if let Some(cancel_btn) = cancel_btn {
            action_controls = action_controls.push(cancel_btn);
            has_action_control = true;
        }
        if let Some(send_btn) = send_btn {
            action_controls = action_controls.push(send_btn);
            has_action_control = true;
        }
        if has_action_control {
            bar = bar.push(action_controls);
        }
    }

    // 设置垂直对齐和内边距，返回最终布局
    bar.spacing(4)
        .align_y(Alignment::Center)
        .padding(iced::Padding { top: 3.0, right: 12.0, bottom: 6.0, left: 12.0 })
        .into()
}
