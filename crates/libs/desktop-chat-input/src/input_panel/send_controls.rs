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

use desktop_assets::assets::Icon;
use super::composer_icons::ComposerIcon;
use super::icons::icon_svg;
use super::surface_styles::{
    BOTTOM_BAR_CHEVRON_ICON_SIZE, BOTTOM_BAR_ICON_BUTTON_SIZE, BOTTOM_BAR_ICON_SIZE, popover_style,
    round_icon_button_style, selectable_list_button_style, selector_chevron_color,
    selector_label_font, selector_pill_button_style, selector_text_color, square_icon_button_style,
    tooltip_dark_style,
};
use desktop_ui_controls::overlays::AboveOverlay;
use desktop_ui_controls::system_settings_common::styles::settings_segment_button_style;
use desktop_session::message::chat::request::ChatSendBehavior;
use super::send_message::Message;
use vw_api_chat_identity::ReasoningEffortDto;

const MAIN_SEND_BUTTON_SIZE: f32 = BOTTOM_BAR_ICON_BUTTON_SIZE;
const MAIN_SEND_ICON_SIZE: f32 = BOTTOM_BAR_ICON_SIZE;
const SECONDARY_BUTTON_SIZE: f32 = BOTTOM_BAR_ICON_BUTTON_SIZE;
const SECONDARY_ICON_SIZE: f32 = BOTTOM_BAR_ICON_SIZE;
const SEND_MODE_ICON_SIZE: f32 = BOTTOM_BAR_CHEVRON_ICON_SIZE;

pub fn permission_access_button_style(
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

pub fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub fn prominent_action_background(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        Color::from_rgba8(243, 244, 246, 1.0)
    } else {
        Color::from_rgba8(17, 19, 24, 1.0)
    }
}

pub fn prominent_action_foreground(theme: &Theme) -> Color {
    if is_dark_theme(theme) { Color::from_rgba8(15, 16, 18, 1.0) } else { Color::WHITE }
}

pub fn prominent_action_style(
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

pub fn send_behavior_icon(behavior: ChatSendBehavior) -> Icon {
    match behavior {
        ChatSendBehavior::Queue => Icon::ListUl,
        ChatSendBehavior::StopAndSend => Icon::Square,
        ChatSendBehavior::Guide => Icon::ChatTextFill,
    }
}

pub fn send_behavior_popover<'a>(
    i18n: desktop_i18n::I18n,
    selected: ChatSendBehavior,
) -> Element<'a, Message> {
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
                text(i18n.__(behavior.label())).size(13).font(selector_label_font()),
                text(i18n.__(behavior.description())).size(11).style(|theme: &Theme| {
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
            .on_press(Message::SelectChatSendBehavior(behavior));
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
    i18n: desktop_i18n::I18n,
    enabled: bool,
    task_mode_enabled: bool,
    input_text: String,
    task_mode_priority: String,
    task_mode_model: String,
    task_mode_reasoning_effort: Option<ReasoningEffortDto>,
) -> Element<'static, Message> {
    let pool_icon: Element<'_, Message> = ComposerIcon::Queue.svg(SECONDARY_ICON_SIZE)
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
    .style(move |theme: &Theme, status| round_icon_button_style(theme, status, enabled));

    // 根据启用状态和任务模式设置按钮的点击行为
    let pool_btn = if enabled {
        if task_mode_enabled {
            // 任务模式启用：发送带选项的添加任务消息
            pool_btn.on_press(Message::AddTaskFromInputWithOptions {
                content: input_text,
                priority: task_mode_priority,
                model: task_mode_model,
                reasoning_effort: task_mode_reasoning_effort,
            })
        } else {
            // 任务模式未启用：发送简单的添加任务消息
            pool_btn.on_press(Message::AddTaskFromInput(input_text))
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
        text(i18n.__(pool_tip_label))
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(tooltip_dark_style)
    .padding([6, 8]);

    // 返回带工具提示的按钮
    tooltip(pool_btn, pool_tip, Position::Top).into()
}

pub fn acp_permission_modes(
    i18n: desktop_i18n::I18n,
    full_access: bool,
    can_change: bool,
) -> Element<'static, Message> {
    let choice = |mode, label| {
        let selected = full_access == mode;
        button(
            text(if selected { format!("✓ {label}") } else { String::from(label) })
                .size(13)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(move |theme, status| {
            let mut style = settings_segment_button_style(theme, status, selected);
            style.border.width = 0.0;
            style.text_color =
                theme.palette().text.scale_alpha(if selected || can_change { 1.0 } else { 0.55 });
            style
        })
        .on_press_maybe(
            (can_change && !selected)
                .then_some(Message::ToggleFullAccessPermission),
        )
    };
    row![choice(false, i18n.__("审批模式")), choice(true, i18n.__("完全访问权限"))].spacing(8).into()
}

pub fn full_access_button(
    i18n: desktop_i18n::I18n,
    enabled: bool,
    active: bool,
) -> Element<'static, Message> {
    full_access_toggle(
        i18n,
        i18n.__(if active { "完全访问权限" } else { "审批模式" }),
        active,
        enabled.then_some(Message::ToggleFullAccessPermission),
    )
}

pub fn full_access_toggle<M: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    active: bool,
    action: Option<M>,
) -> Element<'static, M> {
    let enabled = action.is_some();
    let access_button = permission_trigger(label, enabled, active, None, action);
    let tooltip_label = if active {
        "关闭完全访问权限"
    } else if enabled {
        "获取完全访问权限"
    } else {
        "当前上下文不可获取完全访问权限"
    };

    tooltip(
        access_button,
        container(text(i18n.__(tooltip_label)).size(12)).style(tooltip_dark_style).padding([6, 8]),
        Position::Top,
    )
    .into()
}

fn permission_trigger<'a, M: Clone + 'a>(
    label: &'static str,
    enabled: bool,
    active: bool,
    expanded: Option<bool>,
    action: Option<M>,
) -> Element<'a, M> {
    let shield = ComposerIcon::Permission.svg(BOTTOM_BAR_ICON_SIZE).style(move |theme: &Theme, _| {
        svg::Style { color: Some(permission_label_color(theme, enabled, active)) }
    });
    let mut label = row![shield, text(label).size(12)].spacing(6).align_y(Alignment::Center);
    if let Some(expanded) = expanded {
        let icon = if expanded { Icon::ChevronUp } else { Icon::ChevronDown };
        label = label.push(
            icon_svg(icon, BOTTOM_BAR_CHEVRON_ICON_SIZE).style(move |theme: &Theme, _| {
                svg::Style {
                    color: Some(permission_label_color(theme, enabled, active).scale_alpha(0.82)),
                }
            }),
        );
    }

    button(container(label).center_y(BOTTOM_BAR_ICON_BUTTON_SIZE))
        .padding([0, 9])
        .style(move |theme, status| {
            let mut style = selector_pill_button_style(theme, status, expanded == Some(true));
            style.text_color = permission_label_color(theme, enabled, active);
            style
        })
        .on_press_maybe(action)
        .into()
}

/// 打开权限选择时只展示状态。切换由明确选择另一项触发。
pub fn permission_mode<'a, M: Clone + 'a>(
    i18n: desktop_i18n::I18n,
    enabled: bool,
    active: bool,
    expanded: bool,
    toggle: M,
    close: M,
    change: M,
) -> Element<'a, M> {
    use iced::widget::column;
    let label = i18n.__(if active { "完全访问权限" } else { "审批模式" });
    let trigger = permission_trigger(label, enabled, active, Some(expanded), Some(toggle));
    if !expanded {
        return trigger;
    }

    let mut choices = column![].spacing(3);
    for (full_access, title, description) in [
        (false, "审批模式", "需要审批的工具会先请求确认。"),
        (true, "完全访问权限", "允许工具免审批运行，仅用于你信任的任务。"),
    ] {
        let selected = full_access == active;
        let mut title = row![text(i18n.__(title)).size(13)].spacing(6).align_y(Alignment::Center);
        if full_access {
            title = title.push(
                container(text(i18n.__("推荐")).size(10))
                    .padding([2, 5])
                    .style(|theme| {
                        let green = full_access_color(theme);
                        container::Style {
                            text_color: Some(green),
                            background: Some(green.scale_alpha(0.12).into()),
                            border: Border::default().rounded(999.0),
                            ..Default::default()
                        }
                    }),
            );
        }
        let mut label = row![
            column![
                title,
                text(i18n.__(description))
                    .size(11)
                    .wrapping(text::Wrapping::None)
                    .style(desktop_ui_controls::system_settings_common::settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if selected {
            label = label.push(icon_svg(Icon::Check, 13.0));
        }
        let action = if selected { Some(close.clone()) } else { enabled.then(|| change.clone()) };
        choices = choices.push(
            button(label)
                .on_press_maybe(action)
                .width(Length::Fill)
                .padding([8, 10])
                .style(move |theme, status| selectable_list_button_style(theme, status, selected)),
        );
    }
    if !enabled {
        choices = choices.push(
            container(
                text(i18n.__("当前无法更改会话权限。"))
                    .size(11)
                    .style(desktop_ui_controls::system_settings_common::settings_muted_text_style),
            )
            .padding([5, 10]),
        );
    }

    let menu = container(choices).width(Length::Fixed(336.0)).padding(8).style(popover_style);
    AboveOverlay::new(trigger, menu)
        .show(true)
        .block_background_events(true)
        .flip_if_clipped()
        .gap(7.0)
        .on_close(close)
        .into()
}

fn permission_label_color(theme: &Theme, enabled: bool, active: bool) -> Color {
    if active {
        full_access_color(theme)
    } else {
        theme.palette().text.scale_alpha(if enabled { 0.82 } else { 0.55 })
    }
}

fn full_access_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        Color::from_rgb8(134, 239, 172)
    } else {
        Color::from_rgb8(22, 101, 52)
    }
}

pub fn workflow_mode_button(
    i18n: desktop_i18n::I18n,
    active: bool,
) -> Element<'static, Message> {
    let workflow_icon: Element<'_, Message> = ComposerIcon::Workflow.svg(SECONDARY_ICON_SIZE)
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
            if active {
                prominent_action_style(theme, status, true)
            } else {
                round_icon_button_style(theme, status, true)
            }
        })
        .on_press(Message::WorkflowModeToggled(!active));

    let tooltip_label = if active { "关闭工作流执行" } else { "启用工作流执行" };
    tooltip(
        button,
        container(text(i18n.__(tooltip_label)).size(12)).style(tooltip_dark_style).padding([6, 8]),
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
/// - 空闲无内容时显示语音聊天入口，有内容时显示发送图标
/// - 请求态在右侧提供模式切换入口，并允许通过弹出菜单切换发送行为
/// - 仅在 `can_send` 为 true 时才可点击主按钮
pub fn send_button<'a>(
    i18n: desktop_i18n::I18n,
    behavior: ChatSendBehavior,
    show_send_mode_popover: bool,
    voice_available: bool,
    enabled: bool,
    can_send: bool,
    is_requesting: bool,
) -> Element<'a, Message> {
    #[cfg(not(target_arch = "wasm32"))]
    let voice_primary = !enabled && !is_requesting;
    #[cfg(target_arch = "wasm32")]
    let voice_primary = false;
    let action_enabled = if voice_primary { voice_available } else { can_send };
    let main_tooltip = if voice_primary {
        if voice_available {
            "开始新的语音聊天"
        } else {
            "当前状态不可开始语音聊天"
        }
    } else if !can_send {
        "请先输入内容"
    } else if is_requesting {
        behavior.description()
    } else {
        "发送消息"
    };

    let icon = if voice_primary {
        svg(svg::Handle::from_memory(br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" d="M4 10v4m4-8v12m4-15v18m4-15v12m4-8v4"/></svg>"#))
            .width(MAIN_SEND_ICON_SIZE).height(MAIN_SEND_ICON_SIZE)
    } else if is_requesting {
        icon_svg(send_behavior_icon(behavior), MAIN_SEND_ICON_SIZE)
    } else {
        ComposerIcon::Send.svg(MAIN_SEND_ICON_SIZE)
    };
    let main_send_icon: Element<'_, Message> = icon
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if action_enabled {
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

    let main_send = button(main_button_content)
        .padding(0)
        .style(move |theme: &Theme, status| prominent_action_style(theme, status, action_enabled));
    #[cfg(not(target_arch = "wasm32"))]
    let main_send = if voice_primary {
        main_send.on_press_maybe(
            voice_available.then_some(Message::VoiceChatStart),
        )
    } else {
        main_send
            .on_press_maybe(can_send.then_some(Message::SendPressed))
    };
    #[cfg(target_arch = "wasm32")]
    let main_send = main_send
        .on_press_maybe(can_send.then_some(Message::SendPressed));

    let main_send = tooltip(
        main_send,
        container(text(i18n.__(main_tooltip)).size(12))
            .style(tooltip_dark_style)
            .padding([6, 8]),
        Position::Top,
    );

    if !is_requesting {
        return main_send.into();
    }

    let toggle_btn = button(
        container(
            icon_svg(
                if show_send_mode_popover { Icon::ChevronUp } else { Icon::ChevronDown },
                SEND_MODE_ICON_SIZE,
            )
            .style(move |theme: &Theme, _| svg::Style {
                color: Some(selector_chevron_color(theme, show_send_mode_popover)),
            }),
        )
        .width(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .height(Length::Fixed(SECONDARY_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .style(move |theme: &Theme, status| round_icon_button_style(theme, status, true))
    .on_press(Message::ToggleSendModePopover);

    let toggle_btn = tooltip(
        toggle_btn,
        container(
            text(if show_send_mode_popover {
                i18n.__("收起发送方式")
            } else {
                i18n.__("展开发送方式")
            })
            .size(12),
        )
        .style(tooltip_dark_style)
        .padding([6, 8]),
        Position::Top,
    );

    let split = row![main_send, toggle_btn].spacing(4).align_y(Alignment::Center);
    let popover = send_behavior_popover(i18n, behavior);

    AboveOverlay::new(split, popover)
        .show(show_send_mode_popover)
        .gap(6.0)
        .on_close(Message::CloseSendModePopover)
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
pub fn cancel_button(i18n: desktop_i18n::I18n, submit_anim: u8) -> Element<'static, Message> {
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
    .on_press(Message::CancelPressed);

    // 创建工具提示容器
    let cancel_tip = container(
        text(i18n.__("停止任务"))
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(tooltip_dark_style)
    .padding([6, 8]);

    // 返回带工具提示的按钮
    tooltip(cancel_btn, cancel_tip, Position::Top).into()
}

/// 右侧操作保持单行，左侧工具在剩余宽度内排布。
pub(crate) fn bottom_bar<'a, Message: Clone + 'a>(
    agent_tab_btn: Option<Element<'a, Message>>,
    tools_tab_btn: Option<Element<'a, Message>>,
    skills_tab_btn: Option<Element<'a, Message>>,
    acp_btn: Option<Element<'a, Message>>,
    workflow_btn: Element<'a, Message>,
    model_btn: Element<'a, Message>,
    usage_btn: Element<'a, Message>,
    attach_btn: Element<'a, Message>,
    voice_btn: Option<Element<'a, Message>>,
    pool_btn: Option<Element<'a, Message>>,
    cancel_btn: Option<Element<'a, Message>>,
    send_btn: Option<Element<'a, Message>>,
    task_mode_enabled: bool,
) -> Element<'a, Message> {
    let mut options = row![]
        .spacing(5)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    options = options.push(workflow_btn);
    for option in [agent_tab_btn, tools_tab_btn, skills_tab_btn, acp_btn]
        .into_iter()
        .flatten()
    {
        options = options.push(option);
    }

    let mut actions = row![usage_btn, model_btn].spacing(6).align_y(Alignment::Center);
    if let Some(voice_btn) = voice_btn {
        actions = actions.push(voice_btn);
    }
    if let Some(pool_btn) = pool_btn {
        actions = actions.push(pool_btn);
    }
    if !task_mode_enabled {
        for action in [cancel_btn, send_btn].into_iter().flatten() {
            actions = actions.push(action);
        }
    }

    row![
        attach_btn,
        options.wrap().vertical_spacing(5),
        actions.width(Length::Shrink),
    ]
    .width(Length::Fill)
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([7, 13])
    .into()
}

/// 只接受当前会话目录与项目目录，不借用宿主状态。
pub fn has_permission_context(session_directory: Option<&str>, project_directory: Option<&str>) -> bool {
    session_directory.is_some_and(|directory| !directory.trim().is_empty())
        || project_directory.is_some_and(|directory| !directory.trim().is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn voice_available(voice: &desktop_voice_ui::message::voice::State) -> bool {
    use desktop_voice_ui::message::voice_draft::Phase;
    !voice.chat_active() && !matches!(voice.draft.phase, Phase::Recording | Phase::Transcribing)
}
pub fn voice_session_available(task_mode: bool, workflow_mode: bool, acp: bool) -> bool {
    !task_mode && !workflow_mode && !acp
}

pub fn has_sendable_input(text: &str, has_attachments: bool) -> bool {
    !text.trim().is_empty() || has_attachments
}
pub fn permission_change_allowed(has_context: bool, saving: bool, draft: bool, requesting: bool) -> bool {
    has_context && !saving && (!draft || !requesting)
}
