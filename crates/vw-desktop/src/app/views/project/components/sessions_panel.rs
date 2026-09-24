//! 项目视图组件模块，负责会话列表和项目工具菜单等可复用界面。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use std::collections::HashSet;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Background, Color, Element, Length, Point, Theme};

use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::icon_svg;
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::status_animation::spinner_frame;
use crate::app::components::system_settings_common::{
    round_icon_btn_style, rounded_action_btn_style, settings_muted_text_style, settings_panel_style,
};
use crate::app::components::widgets::RightClickArea;
use crate::app::message::TaskBoardMessage;
use crate::app::{Message, message};
use vw_shared::session::info as session;

use super::super::styles::{is_dark_theme, session_row_highlight_color, tooltip_bubble};
use super::super::utils::{mix_color, session_title_max_chars, truncate_display_width};
use super::new_session::new_session_button;

fn icon_action_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = round_icon_btn_style(theme, status);
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    style.border.radius = 12.0.into();

    if !is_dark {
        style.background = Some(Background::Color(match status {
            iced::widget::button::Status::Hovered => {
                mix_color(Color::from_rgba8(252, 253, 255, 1.0), palette.primary.base.color, 0.08)
            }
            iced::widget::button::Status::Pressed => palette.primary.base.color.scale_alpha(0.12),
            _ => Color::from_rgba8(252, 253, 255, 0.94),
        }));
        style.border.color = match status {
            iced::widget::button::Status::Hovered => palette.primary.base.color.scale_alpha(0.28),
            iced::widget::button::Status::Pressed => palette.primary.base.color.scale_alpha(0.36),
            _ => Color::from_rgba8(222, 227, 235, 0.98),
        };
        style.shadow = match status {
            iced::widget::button::Status::Hovered => iced::Shadow {
                color: palette.primary.base.color.scale_alpha(0.10),
                offset: iced::Vector::new(0.0, 8.0),
                blur_radius: 18.0,
            },
            iced::widget::button::Status::Pressed => iced::Shadow {
                color: palette.primary.base.color.scale_alpha(0.12),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            _ => iced::Shadow {
                color: Color::BLACK.scale_alpha(0.035),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
        };
    }

    style
}

fn outline_panel_button_style(
    theme: &Theme,
    active: bool,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let active_bg = if is_dark {
        palette.primary.base.color.scale_alpha(0.14)
    } else {
        palette.primary.base.color.scale_alpha(0.08)
    };
    let background = match status {
        iced::widget::button::Status::Hovered => {
            if is_dark {
                palette.background.weak.color.scale_alpha(0.84)
            } else {
                Color::from_rgba8(243, 246, 249, 1.0)
            }
        }
        iced::widget::button::Status::Pressed => {
            palette.background.strong.color.scale_alpha(if is_dark { 0.84 } else { 0.42 })
        }
        _ => {
            if active {
                active_bg
            } else {
                Color::TRANSPARENT
            }
        }
    };
    let border_color = if active {
        palette.primary.base.color.scale_alpha(if is_dark { 0.72 } else { 0.40 })
    } else if is_dark {
        palette.background.strong.color.scale_alpha(0.72)
    } else {
        Color::from_rgba8(224, 228, 236, 0.98)
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: if active { palette.primary.base.color } else { theme.palette().text },
        border: iced::Border { radius: 10.0.into(), width: 1.0, color: border_color },
        shadow: if active {
            iced::Shadow {
                color: palette.primary.base.color.scale_alpha(if is_dark { 0.16 } else { 0.08 }),
                offset: iced::Vector::new(0.0, 8.0),
                blur_radius: 18.0,
            }
        } else {
            iced::Shadow::default()
        },
        ..Default::default()
    }
}

fn session_row_button_style(
    theme: &Theme,
    active: bool,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let highlight_bg =
        session_row_highlight_color(theme).scale_alpha(if is_dark { 0.72 } else { 0.58 });
    let background = match status {
        iced::widget::button::Status::Hovered => highlight_bg,
        iced::widget::button::Status::Pressed => {
            palette.background.strong.color.scale_alpha(if is_dark { 0.42 } else { 0.22 })
        }
        _ => {
            if active {
                highlight_bg
            } else {
                Color::TRANSPARENT
            }
        }
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text,
        border: iced::Border { radius: 16.0.into(), width: 0.0, color: Color::TRANSPARENT },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

fn session_menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let bg = match status {
        iced::widget::button::Status::Hovered => {
            if is_dark {
                p.background.weak.color.scale_alpha(0.74)
            } else {
                Color::WHITE.scale_alpha(0.92)
            }
        }
        iced::widget::button::Status::Pressed => {
            p.background.strong.color.scale_alpha(if is_dark { 0.82 } else { 0.28 })
        }
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 10.0.into(),
            width: if matches!(status, iced::widget::button::Status::Hovered) { 1.0 } else { 0.0 },
            color: p.background.strong.color.scale_alpha(0.42),
        },
        ..Default::default()
    }
}

fn menu_separator_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style { background: Some(p.background.strong.color.into()), ..Default::default() }
}

fn floating_menu_panel_style(theme: &Theme) -> container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = 16.0.into();
    style
}

fn status_text_style(color: Color) -> text::Style {
    text::Style { color: Some(color) }
}

fn muted_status_icon_style(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.palette().text.scale_alpha(0.3)) }
}

fn running_status_color(breathe: f32) -> Color {
    mix_color(Color::from_rgb8(245, 158, 11), Color::from_rgb8(239, 68, 68), breathe)
}

fn queued_status_color(breathe: f32) -> Color {
    mix_color(Color::from_rgb8(245, 158, 11), Color::from_rgb8(239, 68, 68), breathe * 0.75)
}

pub(in crate::app::views::project) fn session_status_icon(
    session_id: &str,
    is_running: bool,
    queued: usize,
    has_unseen_success: bool,
    now_s: f64,
) -> Element<'static, Message> {
    let session_phase = session_id
        .bytes()
        .fold(0u32, |acc, byte| acc.wrapping_mul(16777619).wrapping_add(byte as u32));
    let phase = ((now_s / 1.8) + (session_phase as f64 / u32::MAX as f64)) * std::f64::consts::TAU;
    let breathe = ((phase.sin() + 1.0) * 0.5) as f32;
    let icon = if is_running {
        let running_color = running_status_color(breathe);
        text("●").size(12.0).style(move |_: &Theme| status_text_style(running_color))
    } else if queued > 0 {
        let queued_color = queued_status_color(breathe);
        text("●").size(8.5).style(move |_: &Theme| status_text_style(queued_color))
    } else if has_unseen_success {
        text("●").size(11).style(|_: &Theme| status_text_style(Color::from_rgb8(46, 184, 114)))
    } else {
        text("–").size(11).style(muted_status_icon_style)
    };

    container(icon).width(Length::Fixed(14.0)).align_x(iced::alignment::Horizontal::Center).into()
}

fn status_badge_style(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color.scale_alpha(0.16))),
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        text_color: Some(color),
        ..Default::default()
    }
}

fn additions_text_style(_: &Theme) -> text::Style {
    status_text_style(Color::from_rgb8(46, 184, 114))
}

fn deletions_text_style(_: &Theme) -> text::Style {
    status_text_style(Color::from_rgb8(239, 68, 68))
}

fn task_pool_icon_style(
    theme: &Theme,
    _status: iced::widget::svg::Status,
) -> iced::widget::svg::Style {
    iced::widget::svg::Style { color: Some(theme.palette().text) }
}

fn task_pool_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border.radius = 14.0.into();
    style
}

fn load_more_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = outline_panel_button_style(theme, false, status);
    style.border.width = 0.0;
    style.border.color = Color::TRANSPARENT;
    style
}

fn project_panel_title(
    path: &str,
    recent_projects: &[String],
    recent_project_edits: &[String],
) -> String {
    recent_projects
        .iter()
        .position(|p| p == path)
        .and_then(|i| recent_project_edits.get(i).cloned())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| path.to_string())
}

fn session_workspace_label(session_directory: &str, project_path: &str) -> String {
    if session_directory == project_path {
        "主工作区".to_string()
    } else {
        std::path::Path::new(session_directory)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "工作区".to_string())
    }
}

/// 构建菜单界面。
///
/// # 参数
/// - `label`: 当前视图构建所需的状态、配置或消息。
/// - `msg`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn session_menu_button<'a>(label: &str, msg: Message) -> Element<'a, Message> {
    let label = label.to_string();
    button(container(text(label).size(12)).width(Length::Fill).padding([2, 6]))
        .on_press(msg)
        .style(session_menu_button_style)
        .width(Length::Fill)
        .into()
}

fn disabled_session_menu_button<'a>(label: &str) -> Element<'a, Message> {
    let label = label.to_string();
    button(container(text(label).size(12)).width(Length::Fill).padding([2, 6]))
        .style(session_menu_button_style)
        .width(Length::Fill)
        .into()
}

fn session_compact_message(session_id: &str, project_path: &str) -> Message {
    Message::Chat(message::ChatMessage::CompactSessionRequested {
        session_id: session_id.to_string(),
        project_path: project_path.to_string(),
    })
}

/// 构建对应界面片段。
///
/// # 参数
/// - `id`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_session_menu<'a>(
    id: String,
    project_path: String,
    compaction_disabled: bool,
) -> Element<'a, Message> {
    let separator = || -> Element<'a, Message> {
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(menu_separator_style)
            .into()
    };

    let compact_action = if compaction_disabled {
        disabled_session_menu_button("压缩会话")
    } else {
        session_menu_button("压缩会话", session_compact_message(&id, &project_path))
    };
    let content = column![
        compact_action,
        session_menu_button(
            "重命名",
            Message::Project(message::ProjectMessage::SessionRenamePressed(id.clone())),
        ),
        session_menu_button(
            "复制对话",
            Message::Project(message::ProjectMessage::SessionCopyPressed(id.clone())),
        ),
        separator(),
        session_menu_button(
            "归档",
            Message::Project(message::ProjectMessage::SessionArchivePressed(id.clone())),
        ),
        session_menu_button(
            "删除",
            Message::Project(message::ProjectMessage::SessionDeletePressed(id.clone())),
        )
    ]
    .spacing(4);

    container(content)
        .padding([6, 8])
        .style(floating_menu_panel_style)
        .width(Length::Fixed(120.0))
        .into()
}

/// 构建对应界面片段。
///
/// # 参数
/// - `path`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_project_tools_menu<'a>(path: String) -> Element<'a, Message> {
    let separator = || -> Element<'a, Message> {
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(menu_separator_style)
            .into()
    };

    let content = column![
        session_menu_button(
            "编辑项目",
            Message::Project(message::ProjectMessage::ProjectEditOpened(path.clone())),
        ),
        separator(),
        session_menu_button(
            "在访达中显示",
            Message::Project(message::ProjectMessage::RecentRevealPressed(path.clone())),
        ),
        session_menu_button(
            "移除项目",
            Message::Project(message::ProjectMessage::RecentRemovePressed(path)),
        )
    ]
    .spacing(4);

    container(content)
        .padding([6, 8])
        .style(floating_menu_panel_style)
        .width(Length::Fixed(132.0))
        .into()
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `title`: 当前视图构建所需的状态、配置或消息。
/// - `path`: 当前视图构建所需的状态、配置或消息。
/// - `path_max_chars`: 当前视图构建所需的状态、配置或消息。
/// - `clickable`: 当前视图构建所需的状态、配置或消息。
/// - `show_tools_menu`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn project_sessions_header<'a>(
    title: String,
    path: String,
    path_max_chars: usize,
    clickable: bool,
    show_tools_menu: bool,
) -> Element<'a, Message> {
    let title_text: Element<'a, Message> = if clickable {
        iced::widget::mouse_area(
            container(
                text(title)
                    .size(16)
                    .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
            )
            .width(Length::Fill),
        )
        .on_press(Message::Project(message::ProjectMessage::OpenRecentPressed(path.clone())))
        .into()
    } else {
        container(
            text(title)
                .size(16)
                .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
        )
        .width(Length::Fill)
        .into()
    };

    let path_for_refresh = path.clone();
    let path_for_toggle = path.clone();
    let path_for_menu = path.clone();
    let refresh_btn = iced::widget::tooltip::Tooltip::new(
        button(
            container(icon_svg(Icon::ArrowRepeat, 12.0))
                .width(Length::Fixed(26.0))
                .height(Length::Fixed(26.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .on_press(Message::Project(message::ProjectMessage::ProjectLoadSessions(path_for_refresh)))
        .padding(0)
        .style(icon_action_button_style),
        tooltip_bubble("刷新会话".to_string()),
        iced::widget::tooltip::Position::Top,
    )
    .gap(8);
    let more_btn = button(
        container(icon_svg(Icon::Sliders, 12.0))
            .width(Length::Fixed(26.0))
            .height(Length::Fixed(26.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::Project(message::ProjectMessage::ProjectToolsMenuToggled(path_for_toggle)))
    .padding(0)
    .style(icon_action_button_style);

    let tools: Element<'a, Message> =
        PointBelowOverlay::new(more_btn, build_project_tools_menu(path_for_menu))
            .show(show_tools_menu)
            .anchor(Point::new(20.0, 20.0))
            .on_close(Message::Project(message::ProjectMessage::ProjectToolsMenuClosed))
            .into();

    let action_buttons = row![refresh_btn, tools].spacing(6).align_y(iced::Alignment::Center);

    column![
        row![title_text, action_buttons]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .width(Length::Fill),
        text(vw_shared::util::truncate(&path, path_max_chars))
            .size(11)
            .style(settings_muted_text_style)
            .width(Length::Fill),
    ]
    .spacing(3)
    .width(Length::Fill)
    .into()
}

#[derive(Debug)]
struct SessionTreeRow<'a> {
    session: &'a session::Info,
}

#[derive(Debug)]
struct SessionTreeProjection<'a> {
    rows: Vec<SessionTreeRow<'a>>,
    total_roots: usize,
}

fn compare_session_recency(left: &&session::Info, right: &&session::Info, pinned: &HashSet<String>) -> std::cmp::Ordering {
    pinned.contains(&right.id).cmp(&pinned.contains(&left.id)).then_with(|| right
        .time
        .updated
        .cmp(&left.time.updated)
        .then_with(|| right.time.created.cmp(&left.time.created))
        .then_with(|| left.id.cmp(&right.id)))
}

fn project_pinned_threads(project_id: &str) -> HashSet<String> {
    let Some(home) = directories::BaseDirs::new() else { return HashSet::new(); };
    let tenant = vw_gateway_client::vw_api_types::id::TenantScope::default();
    let scope = vw_shared::session::ui_store::pinned_threads_scope(tenant.platform_id().as_ref(), tenant.company_id().as_ref(), project_id);
    vw_shared::session::ui_store::load_pinned_threads(&vw_config_types::paths::home_config_dir(home.home_dir()), &scope).unwrap_or_default()
}

fn session_tree_projection<'a>(
    sessions: &'a [session::Info],
    load_count: usize,
) -> SessionTreeProjection<'a> {
    let mut seen_ids = HashSet::new();
    let mut roots = sessions
        .iter()
        .filter(|session| {
            session.time.archived.is_none()
                && !session.is_plan_run()
                && (session.parent_id.as_deref().is_none_or(|id| id.trim().is_empty())
                    || session.is_visible_task())
                && seen_ids.insert(session.id.as_str())
        })
        .collect::<Vec<_>>();
    let pinned = sessions.first().map(|session| project_pinned_threads(&session.project_id)).unwrap_or_default();
    roots.sort_by(|left, right| compare_session_recency(left, right, &pinned));
    let total_roots = roots.len();
    let rows =
        roots.into_iter().take(load_count).map(|session| SessionTreeRow { session }).collect();

    SessionTreeProjection { rows, total_roots }
}

fn session_tree_row<'a>(
    app: &'a crate::app::App,
    path: &str,
    tree_row: SessionTreeRow<'a>,
    title_max_chars: usize,
    now_s: f64,
) -> Element<'a, Message> {
    let s = tree_row.session;
    let id = s.id.clone();
    let session_title = s.title.clone();
    let active = app.active_session_id.as_ref() == Some(&id);
    let runtime = app.session_runtime_states.get(&id);
    let is_running = runtime.map(|r| r.is_requesting).unwrap_or(false);
    let queued = runtime.map(|r| r.queue.len()).unwrap_or(0);
    let has_unseen_success = runtime.map(|r| r.has_unseen_success).unwrap_or(false);
    let status_icon = session_status_icon(&id, is_running, queued, has_unseen_success, now_s);

    let (adds, dels) =
        if let Some(sum) = &s.summary { (sum.additions, sum.deletions) } else { (0, 0) };

    let mut status_badges = row![].spacing(4);
    if is_running {
        let running_badge_color = Color::from_rgb8(239, 68, 68);
        status_badges = status_badges.push(
            container(
                row![
                    container(
                        text(spinner_frame(app.status_animation_frame))
                            .size(11)
                            .line_height(iced::widget::text::LineHeight::Relative(1.0))
                            .style(move |_: &Theme| status_text_style(running_badge_color)),
                    )
                    .height(Length::Fixed(13.0))
                    .align_y(iced::alignment::Vertical::Center),
                    container(
                        text("运行中")
                            .size(11)
                            .line_height(iced::widget::text::LineHeight::Relative(1.0)),
                    )
                    .height(Length::Fixed(13.0))
                    .align_y(iced::alignment::Vertical::Center)
                ]
                .spacing(4)
                .align_y(iced::alignment::Vertical::Center),
            )
            .padding([1, 6])
            .style(move |_: &Theme| status_badge_style(running_badge_color)),
        );
    }
    if queued > 0 {
        status_badges = status_badges.push(
            container(text(format!("排队 {}", queued)).size(11))
                .padding([1, 6])
                .style(|_: &Theme| status_badge_style(Color::from_rgb8(245, 158, 11))),
        );
    }
    if has_unseen_success && !is_running && queued == 0 {
        status_badges = status_badges.push(
            container(text("有更新").size(11))
                .padding([1, 6])
                .style(|_: &Theme| status_badge_style(Color::from_rgb8(46, 184, 114))),
        );
    }
    let dynamic_title_max_chars = title_max_chars.saturating_add(6).max(14);

    let workspace_label = session_workspace_label(&s.directory, path);
    let workspace_meta = truncate_display_width(&workspace_label, dynamic_title_max_chars / 2 + 8);
    let change_stats = row![
        text(format!("+{}", adds)).size(11).style(additions_text_style),
        text(format!("-{}", dels)).size(11).style(deletions_text_style),
    ]
    .spacing(4)
    .align_y(iced::alignment::Vertical::Center);

    let header_row = row![
        status_icon,
        text(truncate_display_width(&session_title, dynamic_title_max_chars))
            .size(13)
            .font(iced::Font {
                weight: if active { iced::font::Weight::Bold } else { iced::font::Weight::Medium },
                ..Default::default()
            })
            .wrapping(iced::widget::text::Wrapping::None)
            .width(Length::Fill),
    ]
    .spacing(6)
    .align_y(iced::alignment::Vertical::Center)
    .width(Length::Fill);

    let mut footer_row = row![
        text(workspace_meta).size(11).style(settings_muted_text_style),
        text("·").size(11).style(settings_muted_text_style),
        change_stats,
    ]
    .align_y(iced::alignment::Vertical::Center)
    .spacing(5);
    if is_running || queued > 0 || has_unseen_success {
        footer_row = footer_row.push(status_badges);
    }

    let row_content = column![header_row, footer_row]
        .spacing(3)
        .align_x(iced::alignment::Horizontal::Left)
        .width(Length::Fill);

    let hover_tip = session_title.clone();
    let session_directory = s.directory.clone();
    let session_button = button(container(row_content).padding([3, 6]))
        .on_press(Message::Project(message::ProjectMessage::OpenProjectSessionPressed(
            session_directory,
            id.clone(),
        )))
        .width(Length::Fill)
        .style(move |theme: &Theme, status| session_row_button_style(theme, active, status));
    let session_button: Element<'a, Message> = iced::widget::tooltip::Tooltip::new(
        session_button,
        tooltip_bubble(hover_tip),
        iced::widget::tooltip::Position::Top,
    )
    .gap(8)
    .into();
    let id_for_right_click = std::rc::Rc::new(id.clone());
    let right_click = Element::new(RightClickArea::new(
        session_button,
        Box::new(move |pos| {
            Message::Project(message::ProjectMessage::SessionRightClicked(
                (*id_for_right_click).clone(),
                pos.x,
                pos.y,
            ))
        }),
    ));

    if app.session_menu_id == Some(id.clone()) {
        PointBelowOverlay::new(
            right_click,
            build_session_menu(id, s.directory.clone(), is_running || s.time.compacting.is_some()),
        )
        .show(true)
        .anchor(app.session_menu_anchor.unwrap_or(Point::ORIGIN))
        .on_close(Message::Project(message::ProjectMessage::SessionMenuClose))
        .into()
    } else {
        right_click
    }
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `path`: 当前视图构建所需的状态、配置或消息。
/// - `sessions`: 当前视图构建所需的状态、配置或消息。
/// - `title_max_chars`: 当前视图构建所需的状态、配置或消息。
/// - `load_count`: 当前视图构建所需的状态、配置或消息。
/// - `is_loading`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn session_items_list<'a>(
    app: &'a crate::app::App,
    path: &str,
    sessions: Option<&'a Vec<session::Info>>,
    title_max_chars: usize,
    load_count: usize,
    is_loading: bool,
) -> Element<'a, Message> {
    let mut session_items = column![].spacing(5);
    let now_s = crate::app::time::now_ms() as f64 / 1000.0;

    if is_loading {
        session_items = session_items
            .push(container(text("加载中...").size(12)).padding([8, 4]).width(Length::Fill));
    } else if let Some(sessions) = sessions {
        let projection = session_tree_projection(sessions, load_count);
        let total_visible = projection.total_roots;
        for tree_row in projection.rows {
            session_items =
                session_items.push(session_tree_row(app, path, tree_row, title_max_chars, now_s));
        }

        if total_visible == 0 {
            session_items = session_items
                .push(container(text("没有会话").size(12)).padding([8, 4]).width(Length::Fill));
        }

        if total_visible > load_count {
            let remaining = total_visible.saturating_sub(load_count);
            let load_more_btn = button(
                container(text(format!("加载更多 (剩余 {})", remaining)).size(11))
                    .padding([6, 4])
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .on_press(Message::Project(message::ProjectMessage::ProjectLoadMoreSessions(
                path.to_string(),
            )))
            .width(Length::Fill)
            .style(load_more_button_style);
            session_items = session_items.push(load_more_btn);
        }
    } else {
        let load_btn = button(
            container(text("加载会话").size(12))
                .padding([6, 4])
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::Project(message::ProjectMessage::ProjectLoadSessions(path.to_string())))
        .width(Length::Fill)
        .style(|theme: &Theme, status| outline_panel_button_style(theme, false, status));
        session_items = session_items.push(load_btn);
    }

    session_items.into()
}

/// 构建面板界面。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `settings_panel_width`: 当前视图构建所需的状态、配置或消息。
/// - `left_rail_width`: 当前视图构建所需的状态、配置或消息。
/// - `session_panel_width_scale`: 当前视图构建所需的状态、配置或消息。
/// - `target_path`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn project_sessions_panel(
    app: &crate::app::App,
    settings_panel_width: f32,
    left_rail_width: f32,
    session_panel_width_scale: f32,
    target_path: Option<String>,
) -> Element<'_, Message> {
    let panel_w = ((settings_panel_width - left_rail_width) * session_panel_width_scale).max(0.0);

    if let Some(path) = target_path {
        let horizontal_padding = 14.0;
        let title = project_panel_title(&path, &app.recent_projects, &app.recent_projects_edits);

        let is_loading = app.project_sessions_loading.contains(&path);
        let load_count = app.project_session_load_counts.get(&path).copied().unwrap_or(10);
        let has_vertical_scrollbar =
            app.project_session_has_vertical_scrollbar.get(&path).copied().unwrap_or(false);
        let session_list_width_reduction = if has_vertical_scrollbar { 5.0 } else { 0.0 };
        let sessions = app.project_sessions.get(&path);
        let inner_w = (panel_w - horizontal_padding * 2.0).max(0.0);
        let path_max_chars = (inner_w / 5.5).max(8.0) as usize;
        let title_max_chars =
            session_title_max_chars((inner_w - session_list_width_reduction).max(0.0));
        let header = project_sessions_header(
            title,
            path.clone(),
            path_max_chars,
            false,
            app.project_tools_menu_path.as_ref() == Some(&path),
        );
        let new_session_btn = new_session_button(app, path.clone());
        let session_items =
            session_items_list(app, &path, sessions, title_max_chars, load_count, is_loading);
        let task_pool_btn = button(
            container(
                row![
                    iced::widget::svg::Svg::new(crate::app::assets::get_icon(
                        crate::app::assets::Icon::Grid1x2
                    ))
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(task_pool_icon_style),
                    text("任务池").size(12),
                ]
                .align_y(iced::alignment::Vertical::Center)
                .spacing(6),
            )
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::TaskBoard(TaskBoardMessage::ToggleBoard))
        .padding([10, 14])
        .style(task_pool_button_style);

        let static_content = column![
            header,
            iced::widget::Space::new().height(Length::Fixed(6.0)),
            task_pool_btn,
            iced::widget::Space::new().height(Length::Fixed(6.0)),
            new_session_btn,
            iced::widget::Space::new().height(Length::Fixed(6.0)),
        ]
        .spacing(0)
        .width(Length::Fill);

        let path_for_scroll = path.clone();
        let sessions_scroll =
            scrollable(container(session_items).width(Length::Fill).padding(iced::Padding {
                top: 0.0,
                right: session_list_width_reduction,
                bottom: 0.0,
                left: 0.0,
            }))
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .on_scroll(move |viewport| {
                Message::Project(message::ProjectMessage::ProjectSessionListScrollChanged {
                    project_path: path_for_scroll.clone(),
                    has_vertical_scrollbar: viewport.content_bounds().height
                        > viewport.bounds().height,
                })
            })
            .width(Length::Fill)
            .height(Length::Fill);

        return column![
            container(static_content).width(Length::Fill).padding(iced::Padding {
                top: 16.0,
                right: horizontal_padding,
                bottom: 0.0,
                left: horizontal_padding,
            }),
            container(sessions_scroll).width(Length::Fill).height(Length::Fill).padding(
                iced::Padding {
                    top: 0.0,
                    right: horizontal_padding,
                    bottom: 14.0,
                    left: horizontal_padding,
                }
            ),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let empty_label =
        if app.recent_projects.is_empty() { "暂无项目" } else { "请选择一个项目" };
    container(text(empty_label).size(12))
        .padding([20, 20])
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}
#[cfg(test)]
#[path = "sessions_panel_tests.rs"]
mod sessions_panel_tests;
