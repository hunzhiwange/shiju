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
    Space, button, canvas, column, container, mouse_area, responsive, row, text,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::presentation::icon_svg;
use desktop_ui_controls::animated_text::mix_color;
pub use super::header_session::*;
use desktop_assets::assets::Icon;
use desktop_ui_controls::tooltip::tooltip_dark_style;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::status_animation::{session_running_dot, session_spinner};
use super::header_usage::UsageRing;
pub use super::header_style::*;
use desktop_i18n::I18n;

#[derive(Debug, Clone)]
pub enum HeaderAction {
    None, Title(String), Mode(bool), Fullscreen, HalfFullscreen, Usage,
    ToggleActions, ClosePopovers, Rename(String), Archive(String), Delete(String),
    OpenParent(String, String), Compact { session_id: String, project_path: String },
}

pub struct HeaderInput<'a> {
    pub active_session_id: Option<String>,
    pub sessions: &'a [session_info::Info],
    pub project_sessions: &'a std::collections::HashMap<String, Vec<session_info::Info>>,
    pub project_path: Option<String>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub chat_panel_mode: bool,
    pub fullscreen: bool,
    pub half_fullscreen: bool,
    pub show_session_actions_popover: bool,
    pub status_animation_frame: usize,
    pub title: String,
    pub usage_percent: f32,
    pub usage_details: (i64, i64, f64, i64),
    pub input_limit: u64,
    pub limit_is_default: bool,
    pub runtime: (bool, usize, bool),
    pub now_ms: u64,
    pub header_background: Option<Color>,
}
fn fitted_session_title(title: &str, width: f32) -> String {
    use iced::advanced::text::{Paragraph as _, Shaping, Wrapping};

    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let measure = |content: &str| {
        iced::advanced::graphics::text::Paragraph::with_text(iced::advanced::text::Text {
            content,
            bounds: iced::Size::INFINITE,
            size: 14.0.into(),
            line_height: Default::default(),
            font: iced::Font::DEFAULT,
            align_x: Default::default(),
            align_y: iced::alignment::Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        })
        .min_width()
    };
    if measure(&title) <= width {
        return title;
    }
    if measure("...") > width {
        return String::new();
    }
    let boundaries: Vec<_> = title.char_indices().map(|(index, _)| index).collect();
    let mut start = 0;
    let mut end = boundaries.len();
    while start < end {
        let middle = (start + end).div_ceil(2);
        let candidate = format!("{}...", &title[..boundaries[middle - 1]]);
        if measure(&candidate) <= width {
            start = middle;
        } else {
            end = middle - 1;
        }
    }
    let prefix_end = if start == 0 { 0 } else { boundaries[start - 1] };
    format!("{}...", &title[..prefix_end])
}

pub fn show_half_fullscreen_control() -> bool { true }

fn chat_mode_control<'a>(app: &HeaderInput<'_>) -> Element<'a, HeaderAction> {
    let mode_button = |label, mode| {
        let active = app.chat_panel_mode == mode;
        button(text(label).size(11))
            .padding([3, 7])
            .style(move |theme, status| chat_mode_button_style(theme, status, active))
            .on_press(HeaderAction::Mode(mode))
    };
    row![
        mode_button(
            I18n::new(app.desktop_locale).__("对话"),
            false
        ),
        mode_button(
            I18n::new(app.desktop_locale).__("轨迹"),
            true
        ),
    ]
    .spacing(3)
    .align_y(Alignment::Center)
    .into()
}

fn return_to_parent_button<'a>(app: &HeaderInput<'_>, msg: HeaderAction) -> Element<'a, HeaderAction> {
    button(
        row![
            icon_svg(Icon::Back).width(Length::Fixed(11.0)).height(Length::Fixed(11.0)).style(
                |theme: &Theme, _status| svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.88)),
                }
            ),
            text(I18n::new(app.desktop_locale).__("返回主会话")).size(10),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([3, 8])
    .style(return_to_parent_button_style)
    .on_press(msg)
    .into()
}

pub fn session_workspace_badge<'a>(app: &HeaderInput<'_>) -> Option<Element<'a, HeaderAction>> {
    let (label, is_primary) = session_workspace_badge_state(app)?;

    Some(
        container(text(label).size(10))
            .padding([3, 8])
            .style(move |theme: &Theme| session_workspace_badge_style(theme, is_primary))
            .into(),
    )
}

pub fn header_icon_button<'a, Message: Clone + 'a>(
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

pub fn chat_header_view<'a>(app: &HeaderInput<'_>) -> Element<'a, HeaderAction> {
    let session_title = app.title.clone();
    let usage_percent = app.usage_percent;
    let (input_tokens, context_limit, estimated_cost, total_tokens) = app.usage_details;
    let input_limit = app.input_limit;
    let (is_running, queued, has_unseen_success, breathe, running_color) = get_session_status(app);

    let status_icon: Element<'_, HeaderAction> = {
        if is_running {
            session_running_dot(running_color).into()
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

    let mut status_badges: Element<'_, HeaderAction> = Space::new().width(Length::Fixed(0.0)).into();
    if is_running || queued > 0 {
        let mut badges_row = row![].spacing(4).align_y(Alignment::Center);
        if is_running {
            let running_badge_color = Color::from_rgb8(239, 68, 68);
            badges_row = badges_row.push(
                container(
                    row![
                        session_spinner(app.status_animation_frame, 9.0, running_badge_color),
                        text(I18n::new(app.desktop_locale).__("运行中")).size(9)
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
                container(
                    text(
                        I18n::new(app.desktop_locale)
                            .format("排队 %u", &[queued.into()])
                            .expect("valid usage template"),
                    )
                    .size(9),
                )
                .padding([1, 6])
                .style(|_: &Theme| queued_status_badge_style()),
            );
        }
        status_badges = badges_row.into();
    }

    let active_session_id = app.active_session_id.clone();
    let title_element = responsive(move |size| {
        let title = mouse_area(
            text(fitted_session_title(&session_title, size.width))
                .size(14)
                .font(iced::Font::DEFAULT)
                .shaping(iced::widget::text::Shaping::Advanced)
                .wrapping(iced::widget::text::Wrapping::None)
                .width(Length::Fill)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.96)),
                }),
        )
        .on_press(if let Some(session_id) = active_session_id.clone() {
            HeaderAction::Title(session_id)
        } else {
            HeaderAction::None
        });
        Tooltip::new(
            title,
            container(text(session_title.clone()).size(12))
                .max_width(480)
                .padding([6, 8])
                .style(tooltip_dark_style),
            TooltipPosition::Bottom,
        )
        .gap(6)
        .into()
    })
    .height(Length::Fixed(21.0));

    let fullscreen_icon =
        if app.fullscreen { Icon::FullscreenExit } else { Icon::Fullscreen };
    let fullscreen_label = if app.fullscreen {
        I18n::new(app.desktop_locale).__("退出全屏")
    } else {
        I18n::new(app.desktop_locale).__("全屏")
    };
    let fullscreen_button = header_icon_button(
        fullscreen_icon,
        fullscreen_label,
        HeaderAction::Fullscreen,
    );
    let mut fullscreen_controls = row![].spacing(6).align_y(Alignment::Center);
    if show_half_fullscreen_control() {
        fullscreen_controls = fullscreen_controls.push(header_icon_button(
            Icon::LayoutTextWindow,
            if app.half_fullscreen {
                I18n::new(app.desktop_locale).__("退出半屏")
            } else {
                I18n::new(app.desktop_locale).__("半屏")
            },
            HeaderAction::HalfFullscreen,
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
            text(
                I18n::new(app.desktop_locale)
                    .format("输入窗口: %d / %u", &[input_tokens.into(), input_limit.into()])
                    .expect("valid usage template")
            )
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
            text(
                I18n::new(app.desktop_locale)
                    .format("上下文上限: %d", &[context_limit.into()])
                    .expect("valid usage template")
            )
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
            text(
                I18n::new(app.desktop_locale)
                    .format("使用率: %.1f%%", &[usage_percent.into()])
                    .expect("valid usage template")
            )
            .size(12)
            .style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::from_rgb8(255, 225, 80)) }
            }),
            text(
                I18n::new(app.desktop_locale)
                    .format("累计 Token: %d", &[total_tokens.into()])
                    .expect("valid usage template")
            )
            .size(12)
            .style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.9)) }
            }),
            text(
                I18n::new(app.desktop_locale)
                    .format("预估成本: $%.4f", &[estimated_cost.into()])
                    .expect("valid usage template")
            )
            .size(12)
            .style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.9)) }
            }),
            if app.limit_is_default {
                text(I18n::new(app.desktop_locale).__("限制：使用保守默认值"))
                    .size(12)
                    .style(|_theme: &Theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(255, 225, 80)),
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
        .on_press(HeaderAction::Usage);

    let usage_rate_btn: Element<'_, HeaderAction> =
        Tooltip::new(usage_btn, usage_tooltip_content, TooltipPosition::Bottom).into();

    let session_menu_icon = icon_svg(Icon::DotsThreeVertical)
        .width(Length::Fixed(13.0))
        .height(Length::Fixed(13.0))
        .style(|theme: &Theme, _status| svg::Style { color: Some(theme.palette().text) });

    let session_menu_content = container(session_menu_icon)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let mut session_menu_btn: Element<'_, HeaderAction> = button(session_menu_content)
        .padding(0)
        .style(move |theme: &Theme, status| round_icon_button_style(theme, status, true))
        .on_press(HeaderAction::ToggleActions)
        .into();

    if app.show_session_actions_popover && app.active_session_id.is_some() {
        session_menu_btn = BelowOverlay::new(session_menu_btn, session_actions_popover(app))
            .show(true)
            .on_close(HeaderAction::ClosePopovers)
            .into();
    }

    let mode_control = chat_mode_control(app);
    let mut left =
        row![status_icon, mode_control, title_element].spacing(6).align_y(Alignment::Center);
    if let Some(tag) = session_workspace_badge(app) {
        left = left.push(tag);
    }
    if let Some(return_msg) = return_to_parent_session_pressed(app) {
        left = left.push(return_to_parent_button(app, return_msg));
    }
    left = left.push(status_badges).width(Length::Fill);

    let header_background = app.header_background;
    let header = container(
        row![left, usage_rate_btn, session_menu_btn, fullscreen_controls]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(iced::Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 })
    .style(move |theme: &Theme| header_surface_style(theme, header_background));

    header.into()
}

pub fn session_actions_button<'a>(label: &str, msg: Option<HeaderAction>) -> Element<'a, HeaderAction> {
    let label = label.to_string();

    let button = button(text(label).size(13))
        .padding([7, 6])
        .width(Length::Fill)
        .style(session_actions_button_style);
    match msg {
        Some(msg) => button.on_press(msg).into(),
        None => button.into(),
    }
}

pub fn session_actions_popover<'a>(app: &HeaderInput<'_>) -> Element<'a, HeaderAction> {
    let session_id = app.active_session_id.clone().unwrap_or_default();

    let rename_msg =
        HeaderAction::Rename(session_id.clone());
    let archive_msg =
        HeaderAction::Archive(session_id.clone());
    let delete_msg =
        HeaderAction::Delete(session_id.clone());

    let content = column![
        session_actions_button(
            I18n::new(app.desktop_locale).__("压缩当前会话"),
            session_compact_message(app)
        ),
        session_actions_button(
            I18n::new(app.desktop_locale).__("重命名"),
            Some(rename_msg)
        ),
        session_actions_button(
            I18n::new(app.desktop_locale).__("归档"),
            Some(archive_msg)
        ),
        session_actions_button(
            I18n::new(app.desktop_locale).__("删除"),
            Some(delete_msg)
        ),
    ]
    .spacing(4);

    container(content)
        .padding([6, 8])
        .style(session_actions_popover_style)
        .width(Length::Fixed(164.0))
        .into()
}
