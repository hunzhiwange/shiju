//! # 聊天面板头部组件
//!
//! 该模块实现了聊天界面的顶部标题栏，包含会话标题、状态指示器、使用率统计和操作菜单。
//!
//! ## 主要功能
//!
//! - **会话标题显示**：显示当前活跃会话的标题，点击可重命名
//! - **状态指示**：显示会话的运行状态（运行中、排队中、成功等）
//! - **工作区徽章**：标识当前会话所属的工作区
//! - **Token 使用率**：以环形进度条形式显示上下文窗口的使用情况
//! - **操作菜单**：提供重命名、归档、删除等会话操作
//!
//! ## 组件结构
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ ● 会话标题 [工作区] [运行中]      [使用率] [菜单]           │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::svg::{self};
use iced::widget::{
    Space, button, canvas, column, container, mouse_area, row, text,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use super::utils::{get_session_title, icon_svg, mix_color};
use crate::app::assets::Icon;
use crate::app::components::input_panel::styles::{round_icon_button_style, tooltip_dark_style};
use crate::app::components::input_panel::usage::{
    UsageRing, get_usage_details, get_usage_rate_percent,
};
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::status_animation::spinner_frame;
use crate::app::state::WorkspaceMode;
use crate::app::theme_catalog::with_panel_background;
use crate::app::{App, Message, message};

fn header_surface_style(
    _theme: &Theme,
    chrome: Option<&crate::app::theme_catalog::ChromeStyleOverrides>,
) -> iced::widget::container::Style {
    // 默认透明；显式/合成的 chat_header_bg 提供分区透明度色层。
    iced::widget::container::Style {
        background: chrome
            .and_then(|c| c.chat_header_bg)
            .map(Background::Color),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        ..Default::default()
    }
}

pub(super) fn show_half_fullscreen_control(_workspace_mode: WorkspaceMode) -> bool {
    true
}

fn theme_is_dark(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

/// 在已加载会话与项目会话缓存中查找会话 Info。
fn known_session_info<'a>(
    app: &'a App,
    session_id: &str,
) -> Option<&'a vw_shared::session::info::Info> {
    app.sessions.iter().find(|session| session.id == session_id).or_else(|| {
        app.project_sessions
            .values()
            .flat_map(|sessions| sessions.iter())
            .find(|session| session.id == session_id)
    })
}

/// 获取当前活跃会话的工作目录
///
/// # 参数
///
/// - `app`: 应用状态引用
///
/// # 返回值
///
/// 返回当前活跃会话的工作目录路径，如果没有活跃会话则返回 `None`
fn active_session_directory(app: &App) -> Option<String> {
    let session_id = app.active_session_id.as_ref()?;
    known_session_info(app, session_id).map(|session| session.directory.clone())
}

/// 当前活动会话若存在 `parent_id`，返回打开父会话的消息。
pub(super) fn return_to_parent_session_pressed(app: &App) -> Option<Message> {
    let session_id = app.active_session_id.as_deref()?;
    let session = known_session_info(app, session_id)?;
    let parent_id = session.parent_id.as_deref()?.trim();
    if parent_id.is_empty() {
        return None;
    }
    let path = app
        .project_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .or_else(|| {
            known_session_info(app, parent_id)
                .map(|parent| parent.directory.clone())
                .filter(|directory| !directory.trim().is_empty())
        })
        .or_else(|| {
            let directory = session.directory.trim();
            (!directory.is_empty()).then(|| directory.to_string())
        })?;
    Some(Message::Project(message::ProjectMessage::OpenProjectSessionPressed(
        path,
        parent_id.to_string(),
    )))
}

fn return_to_parent_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let is_dark = theme_is_dark(theme);
    let p = theme.extended_palette();
    let (bg, border, fg) = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
            if is_dark {
                (
                    p.primary.base.color.scale_alpha(0.22),
                    p.primary.base.color.scale_alpha(0.40),
                    theme.palette().text.scale_alpha(0.96),
                )
            } else {
                (
                    p.primary.base.color.scale_alpha(0.14),
                    p.primary.base.color.scale_alpha(0.28),
                    p.primary.base.color.scale_alpha(0.90),
                )
            }
        }
        _ => {
            if is_dark {
                (
                    p.primary.base.color.scale_alpha(0.14),
                    p.primary.base.color.scale_alpha(0.28),
                    theme.palette().text.scale_alpha(0.92),
                )
            } else {
                (
                    p.primary.base.color.scale_alpha(0.08),
                    p.primary.base.color.scale_alpha(0.18),
                    p.primary.base.color.scale_alpha(0.82),
                )
            }
        }
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { width: 1.0, color: border, radius: 999.0.into() },
        text_color: fg,
        ..Default::default()
    }
}

fn return_to_parent_button<'a>(msg: Message) -> Element<'a, Message> {
    button(
        row![
            icon_svg(Icon::Back)
                .width(Length::Fixed(11.0))
                .height(Length::Fixed(11.0))
                .style(|theme: &Theme, _status| svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.88)),
                }),
            text("返回主会话").size(10),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([3, 8])
    .style(return_to_parent_button_style)
    .on_press(msg)
    .into()
}

fn session_workspace_badge_state(app: &App) -> Option<(String, bool)> {
    let directory = active_session_directory(app)?;
    let project_path = app.project_path.as_deref().unwrap_or_default();
    let is_primary = !project_path.is_empty() && directory == project_path;
    let label = if is_primary {
        "主工作区".to_string()
    } else {
        std::path::Path::new(&directory)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "工作区".to_string())
    };

    Some((label, is_primary))
}

fn session_workspace_badge_style(
    theme: &Theme,
    is_primary: bool,
) -> iced::widget::container::Style {
    let is_dark = theme_is_dark(theme);
    let p = theme.extended_palette();
    let (bg, border, fg) = if is_primary {
        if is_dark {
            (
                Color::from_rgba8(24, 25, 29, 0.96),
                Color::from_rgba8(45, 48, 54, 0.94),
                theme.palette().text.scale_alpha(0.92),
            )
        } else {
            (
                Color::from_rgb8(247, 248, 250),
                Color::from_rgb8(226, 231, 237),
                Color::from_rgb8(75, 85, 99),
            )
        }
    } else if is_dark {
        (
            p.primary.base.color.scale_alpha(0.14),
            p.primary.base.color.scale_alpha(0.28),
            p.primary.base.text.scale_alpha(0.92),
        )
    } else {
        (
            p.primary.base.color.scale_alpha(0.08),
            p.primary.base.color.scale_alpha(0.18),
            p.primary.base.color.scale_alpha(0.82),
        )
    };
    iced::widget::container::Style {
        background: Some(Background::Color(bg)),
        border: Border { width: 1.0, color: border, radius: 999.0.into() },
        text_color: Some(fg),
        ..Default::default()
    }
}

/// 创建会话的工作区徽章组件
///
/// 根据会话的工作目录，显示对应的工作区标签。主工作区与普通工作区使用不同的颜色样式。
///
/// # 参数
///
/// - `app`: 应用状态引用
///
/// # 返回值
///
/// 返回工作区徽章的 UI 元素，如果没有活跃会话则返回 `None`
///
/// # 样式
///
/// - **主工作区**：灰色背景（RGB: 229, 231, 235），深灰文字（RGB: 75, 85, 99）
/// - **普通工作区**：浅蓝背景（RGB: 219, 234, 254），蓝色文字（RGB: 30, 64, 175）
fn session_workspace_badge<'a>(app: &App) -> Option<Element<'a, Message>> {
    let (label, is_primary) = session_workspace_badge_state(app)?;

    Some(
        container(text(label).size(10))
            .padding([3, 8])
            .style(move |theme: &Theme| session_workspace_badge_style(theme, is_primary))
            .into(),
    )
}

/// 获取当前会话的运行状态信息
///
/// 返回会话的详细状态，包括是否正在运行、排队数量、是否有未查看的成功结果，
/// 以及用于状态指示器动画的呼吸效果参数和颜色。
///
/// # 参数
///
/// - `app`: 应用状态引用
///
/// # 返回值
///
/// 返回一个元组，包含：
/// - `bool`: 会话是否正在请求中
/// - `usize`: 排队等待的消息数量
/// - `bool`: 是否有未查看的成功结果
/// - `f32`: 呼吸动画值（0.0 到 1.0），用于颜色渐变
/// - `Color`: 当前状态指示器的颜色（基于呼吸动画在橙色和红色之间混合）
///
/// # 动画算法
///
/// 呼吸效果使用正弦波实现，周期约为 1.8 秒。会话 ID 用于生成相位偏移，
/// 使不同会话的状态指示器动画错开，避免同步闪烁。
fn get_session_status(app: &App) -> (bool, usize, bool, f32, Color) {
    let now_s = crate::app::time::now_ms() as f64 / 1000.0;

    if let Some(session_id) = &app.active_session_id {
        let runtime = app.session_runtime_states.get(session_id);
        let is_running = runtime.map(|r| r.is_requesting).unwrap_or(false);
        let queued = runtime.map(|r| r.queue.len()).unwrap_or(0);
        let has_unseen_success = runtime.map(|r| r.has_unseen_success).unwrap_or(false);

        // 使用 FNV-1a 哈希算法为会话 ID 生成相位偏移
        let session_phase = session_id
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_mul(16777619).wrapping_add(b as u32));
        let phase =
            ((now_s / 1.8) + (session_phase as f64 / u32::MAX as f64)) * std::f64::consts::TAU;
        let breathe = ((phase.sin() + 1.0) * 0.5) as f32;
        let running_color =
            mix_color(Color::from_rgb8(245, 158, 11), Color::from_rgb8(239, 68, 68), breathe);

        (is_running, queued, has_unseen_success, breathe, running_color)
    } else {
        (false, 0, false, 0.0, Color::TRANSPARENT)
    }
}

fn colored_text_style(color: Color) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(color) }
}

fn running_status_badge_style(color: Color) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(color.scale_alpha(0.16))),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        text_color: Some(color),
        ..Default::default()
    }
}

fn queued_status_badge_style() -> iced::widget::container::Style {
    let color = Color::from_rgb8(245, 158, 11);
    iced::widget::container::Style {
        background: Some(Background::Color(color.scale_alpha(0.16))),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        text_color: Some(color),
        ..Default::default()
    }
}

pub(super) fn header_icon_button<'a>(
    icon: Icon,
    tooltip_label: &'static str,
    msg: Message,
) -> Element<'a, Message> {
    let icon = icon_svg(icon).width(Length::Fixed(13.0)).height(Length::Fixed(13.0)).style(
        |theme: &Theme, _status| svg::Style { color: Some(theme.palette().text.scale_alpha(0.88)) },
    );

    let content = container(icon)
        .width(Length::Fixed(26.0))
        .height(Length::Fixed(26.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let button = button(content)
        .padding(0)
        .style(|theme: &Theme, status| round_icon_button_style(theme, status, true))
        .on_press(msg);

    Tooltip::new(
        button,
        container(text(tooltip_label).size(12)).padding([6, 8]).style(tooltip_dark_style),
        TooltipPosition::Bottom,
    )
    .gap(6)
    .into()
}

/// 构建聊天面板的头部视图
///
/// 创建包含会话标题、状态指示器、工作区徽章、使用率统计和操作菜单的头部栏。
///
/// # 参数
///
/// - `app`: 应用状态引用
///
/// # 返回值
///
/// 返回头部栏的 UI 元素
///
/// # UI 布局
///
/// ```text
/// [状态图标] [会话标题] [工作区徽章] [状态标签] --- [使用率按钮] [菜单按钮]
/// ```
///
/// # 组件说明
///
/// - **状态图标**：根据会话状态显示不同颜色的圆点（运行中/排队中/成功）
/// - **会话标题**：可点击，触发重命名操作
/// - **工作区徽章**：显示会话所属的工作区名称
/// - **状态标签**：显示"运行中"或"排队 N"等状态信息
/// - **使用率按钮**：显示环形进度条，悬停显示详细的 token 使用信息
/// - **菜单按钮**：打开会话操作菜单（重命名、归档、删除）
pub fn chat_header_view(app: &App) -> Element<'_, Message> {
    let session_title = get_session_title(app);
    let usage_percent = get_usage_rate_percent(app);
    let (input_tokens, context_limit, estimated_cost, total_tokens) = get_usage_details(app);
    let input_limit = app.usage_model_info.as_ref().map(|info| info.input_limit).unwrap_or(0);
    let (is_running, queued, has_unseen_success, breathe, running_color) = get_session_status(app);

    let status_icon: Element<'_, Message> = {
        if is_running {
            text("●").size(12.0).style(move |_: &Theme| colored_text_style(running_color)).into()
        } else if queued > 0 {
            let queued_color = mix_color(
                Color::from_rgb8(245, 158, 11),
                Color::from_rgb8(239, 68, 68),
                breathe * 0.75,
            );
            text("●").size(8.5).style(move |_: &Theme| colored_text_style(queued_color)).into()
        } else if has_unseen_success {
            text("●")
                .size(11)
                .style(|_: &Theme| colored_text_style(Color::from_rgb8(46, 184, 114)))
                .into()
        } else {
            Space::new().width(Length::Fixed(0.0)).height(Length::Fixed(0.0)).into()
        }
    };

    let mut status_badges: Element<'_, Message> = Space::new().width(Length::Fixed(0.0)).into();
    if is_running || queued > 0 {
        let mut badges_row = row![].spacing(4);
        if is_running {
            let running_badge_color = Color::from_rgb8(239, 68, 68);
            badges_row = badges_row.push(
                container(
                    row![
                        text(spinner_frame(app.status_animation_frame))
                            .size(9)
                            .style(move |_: &Theme| colored_text_style(running_badge_color)),
                        text("运行中").size(9)
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .padding([1, 6])
                .style(move |_: &Theme| running_status_badge_style(running_badge_color)),
            );
        }
        if queued > 0 {
            badges_row = badges_row.push(
                container(text(format!("排队 {}", queued)).size(9))
                    .padding([1, 6])
                    .style(|_: &Theme| queued_status_badge_style()),
            );
        }
        status_badges = badges_row.into();
    }

    let title_element: Element<'_, Message> =
        mouse_area(text(session_title.clone()).size(14).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.96)) }
        }))
        .on_press(if let Some(session_id) = app.active_session_id.clone() {
            Message::Project(message::ProjectMessage::SessionTitleClicked(session_id))
        } else {
            Message::None
        })
        .into();

    let fullscreen_icon =
        if app.is_chat_fullscreen() { Icon::FullscreenExit } else { Icon::Fullscreen };
    let fullscreen_label = if app.is_chat_fullscreen() { "退出全屏" } else { "全屏" };
    let fullscreen_button = header_icon_button(
        fullscreen_icon,
        fullscreen_label,
        Message::Chat(message::ChatMessage::ToggleFullscreen),
    );
    let mut fullscreen_controls = row![].spacing(6).align_y(Alignment::Center);
    if show_half_fullscreen_control(app.workspace_mode) {
        fullscreen_controls = fullscreen_controls.push(header_icon_button(
            Icon::LayoutTextWindow,
            if app.is_chat_half_fullscreen() { "退出半屏" } else { "半屏" },
            Message::Chat(message::ChatMessage::ToggleHalfFullscreen),
        ));
    }
    let fullscreen_controls = fullscreen_controls.push(fullscreen_button);

    let usage_ring = canvas(UsageRing { percent: usage_percent })
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0));

    let usage_btn_inner = container(usage_ring)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let usage_tooltip_content = container(
        column![
            text(format!("输入窗口: {} / {}", input_tokens, input_limit))
                .size(12)
                .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
            text(format!("上下文上限: {}", context_limit))
                .size(12)
                .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
            text(format!("使用率: {:.1}%", usage_percent)).size(12).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::from_rgb8(255, 225, 80)) }
            }),
            text(format!("累计 Token: {}", total_tokens)).size(12).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.9)) }
            }),
            text(format!("预估成本: ${:.4}", estimated_cost)).size(12).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.9)) }
            }),
            if app.usage_model_info.as_ref().is_some_and(|info| info.limit_is_default) {
                text("限制：使用保守默认值").size(12).style(|_theme: &Theme| {
                    iced::widget::text::Style { color: Some(Color::from_rgb8(255, 225, 80)) }
                })
            } else {
                text("").size(12)
            },
        ]
        .spacing(4),
    )
    .style(tooltip_dark_style)
    .padding([8, 10]);

    let usage_btn = button(usage_btn_inner)
        .padding(0)
        .style(|theme: &Theme, status| round_icon_button_style(theme, status, true))
        .on_press(Message::View(message::ViewMessage::OpenUsage));

    let usage_rate_btn: Element<'_, Message> =
        Tooltip::new(usage_btn, usage_tooltip_content, TooltipPosition::Bottom).into();

    let session_menu_icon = icon_svg(Icon::Gear)
        .width(Length::Fixed(13.0))
        .height(Length::Fixed(13.0))
        .style(|theme: &Theme, _status| svg::Style { color: Some(theme.palette().text) });

    let session_menu_content = container(session_menu_icon)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let mut session_menu_btn: Element<'_, Message> = button(session_menu_content)
        .padding(0)
        .style(move |theme: &Theme, status| round_icon_button_style(theme, status, true))
        .on_press(Message::View(message::ViewMessage::ToggleSessionActionsPopover))
        .into();

    if app.show_session_actions_popover && app.active_session_id.is_some() {
        session_menu_btn = BelowOverlay::new(session_menu_btn, session_actions_popover(app))
            .show(true)
            .on_close(Message::View(message::ViewMessage::ClosePopovers))
            .into();
    }

    let mut left = row![status_icon, title_element].spacing(6).align_y(Alignment::Center);
    if let Some(tag) = session_workspace_badge(app) {
        left = left.push(tag);
    }
    if let Some(return_msg) = return_to_parent_session_pressed(app) {
        left = left.push(return_to_parent_button(return_msg));
    }
    left = left.push(status_badges);

    let chrome = crate::app::theme_catalog::resolve_chrome_style_from_skin(
        app.active_theme_skin.as_ref(),
        &app.app_theme,
    );
    let header = container(
        row![
            left,
            Space::new().width(Length::Fill),
            usage_rate_btn,
            session_menu_btn,
            fullscreen_controls
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(iced::Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 })
    .style(move |theme: &Theme| header_surface_style(theme, Some(&chrome)));

    let header_bg = app
        .active_theme_skin
        .as_ref()
        .and_then(|skin| skin.chat_header_path.as_deref());
    let fit = app
        .active_theme_skin
        .as_ref()
        .map(|skin| {
            crate::app::theme_catalog::PanelImageFit::parse(
                skin.chrome.chat_header_image_fit.as_deref(),
            )
            .unwrap_or_default()
        })
        .unwrap_or_default();
    with_panel_background(header.into(), header_bg, fit)
}

/// 构建会话操作弹出菜单
///
/// 创建一个包含重命名、归档和删除操作的弹出菜单。
///
/// # 参数
///
/// - `app`: 应用状态引用
///
/// # 返回值
///
/// 返回弹出菜单的 UI 元素
///
/// # 菜单项
///
/// - **重命名**：触发会话标题编辑
/// - **归档**：将当前会话归档
/// - **删除**：删除当前会话（带分隔线，表示危险操作）
fn session_actions_button<'a>(label: &str, msg: Message) -> Element<'a, Message> {
    let label = label.to_string();

    button(container(text(label).size(13)).width(Length::Fill).padding([2, 6]))
        .width(Length::Fill)
        .style(session_actions_button_style)
        .on_press(msg)
        .into()
}

fn session_actions_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let is_dark = theme_is_dark(theme);
    let bg = match status {
        iced::widget::button::Status::Hovered => {
            if is_dark {
                Color::from_rgba8(31, 33, 38, 0.96)
            } else {
                Color::from_rgba8(241, 243, 246, 1.0)
            }
        }
        iced::widget::button::Status::Pressed => {
            if is_dark {
                Color::from_rgba8(36, 38, 44, 0.98)
            } else {
                Color::from_rgba8(232, 236, 241, 1.0)
            }
        }
        _ => Color::TRANSPARENT,
    };

    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(45, 48, 54, 0.72)
            } else {
                Color::from_rgba8(226, 231, 237, 0.92)
            },
        },
        text_color: theme.palette().text.scale_alpha(0.92),
        ..Default::default()
    }
}

fn session_actions_popover_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = theme_is_dark(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(20, 21, 24, 0.985)
        } else {
            Color::from_rgba8(252, 252, 253, 1.0)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(44, 47, 53, 0.96)
            } else {
                Color::from_rgba8(226, 231, 237, 1.0)
            },
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.20 } else { 0.08 }),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}

fn session_actions_popover(app: &App) -> Element<'_, Message> {
    let session_id = app.active_session_id.clone().unwrap_or_default();

    let rename_msg =
        Message::Project(message::ProjectMessage::SessionRenamePressed(session_id.clone()));
    let archive_msg =
        Message::Project(message::ProjectMessage::SessionArchivePressed(session_id.clone()));
    let delete_msg =
        Message::Project(message::ProjectMessage::SessionDeletePressed(session_id.clone()));

    let content = column![
        session_actions_button("重命名", rename_msg),
        session_actions_button("归档", archive_msg),
        session_actions_button("删除", delete_msg),
    ]
    .spacing(4);

    container(content)
        .padding([6, 8])
        .style(session_actions_popover_style)
        .width(Length::Fixed(120.0))
        .into()
}
#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;
