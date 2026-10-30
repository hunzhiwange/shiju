//! 聊天面板模块
//!
//! 该模块提供聊天界面中主要面板的视图组件，负责渲染消息列表、
//! 聊天头部、滚动控制等核心 UI 元素。
//!
//! ## 主要功能
//!
//! - 渲染完整的聊天消息列表
//! - 管理聊天视图的滚动行为
//! - 提供"跳转到底部"按钮
//! - 支持空会话占位符显示
//!
//! ## 子模块
//!
//! - [`empty`] - 空会话占位符组件
//! - [`header`] - 聊天头部视图组件
//! - [`message_view`] - 单条消息的渲染组件
//! - [`tools`] - 工具相关组件
//! - [`utils`] - 通用工具函数

pub mod chunk_loader;
pub mod empty;
mod global_status;
pub mod header;
pub mod height_index;
pub mod message_measure;
pub mod message_view;
#[cfg(test)]
pub(crate) mod performance;
pub(crate) mod selectable_text;
mod tool_names;
pub mod tool_selector;
pub mod tool_text_support;
pub mod tools;
pub(crate) mod trajectory;
pub mod utils;
pub mod virtualizer;

#[cfg(test)]
#[path = "chunk_loader_tests.rs"]
mod chunk_loader_tests;
#[cfg(test)]
#[path = "global_status_tests.rs"]
mod global_status_tests;
#[cfg(test)]
#[path = "height_index_tests.rs"]
mod height_index_tests;
#[cfg(test)]
#[path = "latest_user_question_button_tests.rs"]
mod latest_user_question_button_tests;
#[cfg(test)]
#[path = "message_measure_tests.rs"]
mod message_measure_tests;
#[cfg(test)]
#[path = "message_view_tests.rs"]
mod message_view_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "performance_tests.rs"]
mod performance_tests;
#[cfg(test)]
#[path = "permission_view_tests.rs"]
mod permission_view_tests;
#[cfg(test)]
#[path = "question_view_tests.rs"]
mod question_view_tests;
#[cfg(test)]
#[path = "selectable_text_tests.rs"]
mod selectable_text_tests;
#[cfg(test)]
#[path = "tool_names_tests.rs"]
mod tool_names_tests;
#[cfg(test)]
#[path = "tool_renderer_tests.rs"]
mod tool_renderer_tests;
#[cfg(test)]
#[path = "tool_selector_tests.rs"]
mod tool_selector_tests;
#[cfg(test)]
#[path = "tool_text_support_tests.rs"]
mod tool_text_support_tests;
#[cfg(test)]
#[path = "virtualizer_tests.rs"]
mod virtualizer_tests;

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, opaque, row, scrollable, stack, text};
use iced::{Background, Border, Color, Element, Length, Padding, Point, Theme};
use once_cell::sync::Lazy;
use std::borrow::Cow;

use crate::app::assets::Icon;
use crate::app::components::input_panel::todo_panel::{self, TodoPanelSurface};
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::models::{ChatMessage, ChatRenderCacheEntry, ChatRole};
use crate::app::state::{ChatPanelMode, UserQuestionMarker};
use crate::app::theme_catalog::with_panel_background;
use crate::app::{App, Message, TodoPanelPlacement, message};

use self::empty::{empty_session_placeholder, session_loading_placeholder};
use self::global_status::chat_global_status_view;
use self::header::chat_header_view;
use self::height_index::CHAT_MESSAGE_GAP;
use self::message_view::estimate_message_height_rough;
use self::utils::{chat_context_menu, chat_scroll_direction, icon_svg};
use self::virtualizer::ChatVirtualWindow;
#[cfg(test)]
use self::virtualizer::ChatVirtualizer;

static FALLBACK_RENDER_CACHE_ENTRY: Lazy<ChatRenderCacheEntry> =
    Lazy::new(ChatRenderCacheEntry::default);

const MAX_USER_QUESTION_MARKERS: usize = 40;
pub(crate) const CONVERSATION_READING_COLUMN_MAX_WIDTH: f32 = 880.0;

fn chat_message_column<'a>() -> iced::widget::Column<'a, Message> {
    column![]
        .spacing(CHAT_MESSAGE_GAP)
        .width(Length::Fill)
        .max_width(CONVERSATION_READING_COLUMN_MAX_WIDTH)
}

// Spacers must not participate in the message column's inter-message spacing.
fn chat_virtual_content<'a>(
    messages: iced::widget::Column<'a, Message>,
    top: f32,
    bottom: f32,
) -> iced::widget::Column<'a, Message> {
    column![Space::new().height(top), messages, Space::new().height(bottom)].width(Length::Fill)
}

pub(crate) fn conversation_reading_column<'a>(
    content: impl Into<Element<'a, Message>>,
    padding: Padding,
) -> iced::widget::Container<'a, Message> {
    container(
        container(content).width(Length::Fill).max_width(CONVERSATION_READING_COLUMN_MAX_WIDTH),
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
    .padding(padding)
}

fn chat_global_status_reading_column(app: &App) -> Element<'_, Message> {
    conversation_reading_column(
        column![message_view::recovery_notice_view(app), chat_global_status_view(app)].spacing(6),
        Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
    )
    .into()
}

#[cfg(test)]
pub(crate) fn compute_visible_message_window_for_chat(
    chat_len: usize,
    message_heights: &[f32],
    scroll_offset_y: f32,
    viewport_h: f32,
) -> (usize, usize) {
    let clamped_len = chat_len.min(message_heights.len());
    if clamped_len == 0 {
        return (0, 0);
    }

    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&message_heights[..clamped_len]);
    virtualizer.set_viewport_height(viewport_h);
    virtualizer.set_relative_y(scroll_offset_y);
    let window = virtualizer.compute_window(0.0);
    (window.visible_start_idx, window.visible_end_idx)
}

pub(crate) fn chat_panel_virtual_window(app: &App) -> ChatVirtualWindow {
    app.chat_virtualizer.window()
}

pub(crate) fn chat_render_window<T>(
    chat: &[T],
    start_idx: usize,
    end_idx: usize,
) -> impl ExactSizeIterator<Item = (usize, &T)> {
    let start_idx = start_idx.min(chat.len());
    let end_idx = end_idx.max(start_idx).min(chat.len());
    chat[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(move |(offset, message)| (start_idx + offset, message))
}

fn chat_panel_context_menu_overlay<'a>(
    app: &'a App,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let is_open = app.chat_context_menu_target.is_some();
    let anchor = app.chat_context_menu_pos.unwrap_or((12.0, 26.0));
    let menu = chat_context_menu(is_open).unwrap_or_else(|| Space::new().into());

    PointBelowOverlay::new(content, menu)
        .show(is_open)
        .global_anchor(Point::new(anchor.0, anchor.1))
        .gap(0.0)
        .close_on_target_click(true)
        .on_close(Message::Chat(message::ChatMessage::CloseMessageContextMenu))
        .into()
}

fn session_loading_panel(app: &App) -> Element<'_, Message> {
    column![
        stack![
            conversation_reading_column(
                session_loading_placeholder(app),
                Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
            )
            .height(Length::Fill),
            empty_fullscreen_controls(app)
        ]
        .width(Length::Fill)
        .height(Length::Fill),
        chat_global_status_reading_column(app)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[allow(dead_code)]
fn refine_message_heights(app: &mut App) {
    app.refine_chat_message_estimated_heights(0, app.chat.len());
}

#[cfg(test)]
pub(crate) fn user_question_indices(chat: &[ChatMessage]) -> Vec<usize> {
    chat.iter()
        .enumerate()
        .filter_map(|(idx, message)| (message.role == ChatRole::User).then_some(idx))
        .collect()
}

#[cfg(test)]
pub(crate) fn is_chat_message_idx_visible(
    target_idx: usize,
    visible_start_idx: usize,
    visible_end_idx: usize,
    viewport_h: f32,
) -> bool {
    if viewport_h <= 0.0 || visible_end_idx <= visible_start_idx {
        return false;
    }

    (visible_start_idx..visible_end_idx).contains(&target_idx)
}

#[cfg(test)]
pub(crate) fn active_user_question_idx(
    user_question_idxs: &[usize],
    visible_start_idx: usize,
    visible_end_idx: usize,
    viewport_h: f32,
) -> Option<usize> {
    if viewport_h <= 0.0 || visible_end_idx <= visible_start_idx {
        return None;
    }

    let position = user_question_idxs.partition_point(|&idx| idx < visible_start_idx);
    user_question_idxs.get(position).copied().filter(|&idx| idx < visible_end_idx)
}

fn active_user_question_marker_position(
    user_question_markers: &[UserQuestionMarker],
    visible_start_idx: usize,
    visible_end_idx: usize,
    viewport_h: f32,
) -> Option<usize> {
    if viewport_h <= 0.0 || visible_end_idx <= visible_start_idx {
        return None;
    }

    let position =
        user_question_markers.partition_point(|marker| marker.message_idx < visible_start_idx);
    user_question_markers
        .get(position)
        .filter(|marker| marker.message_idx < visible_end_idx)
        .map(|_| position)
}

pub(crate) fn project_user_question_marker_positions(
    marker_len: usize,
    active_position: Option<usize>,
) -> Vec<usize> {
    if marker_len <= MAX_USER_QUESTION_MARKERS {
        return (0..marker_len).collect();
    }

    let mut positions = Vec::with_capacity(MAX_USER_QUESTION_MARKERS);
    push_projected_question_marker_position(&mut positions, marker_len, 0);
    push_projected_question_marker_position(&mut positions, marker_len, marker_len - 1);
    if let Some(active_position) = active_position {
        push_projected_question_marker_position(&mut positions, marker_len, active_position);
        push_projected_question_marker_position(
            &mut positions,
            marker_len,
            active_position.saturating_sub(1),
        );
        push_projected_question_marker_position(
            &mut positions,
            marker_len,
            active_position.saturating_add(1),
        );
    }

    let sample_slots = MAX_USER_QUESTION_MARKERS - positions.len();
    for slot in 0..sample_slots {
        push_projected_question_marker_position(
            &mut positions,
            marker_len,
            slot * (marker_len - 1) / (sample_slots - 1),
        );
    }
    positions.sort_unstable();
    positions
}

fn push_projected_question_marker_position(
    positions: &mut Vec<usize>,
    marker_len: usize,
    position: usize,
) {
    if position < marker_len
        && !positions.contains(&position)
        && positions.len() < MAX_USER_QUESTION_MARKERS
    {
        positions.push(position);
    }
}

pub(crate) fn user_question_preview(content: &str) -> String {
    const PREVIEW_CHAR_LIMIT: usize = 100;

    let mut preview = String::new();
    let mut char_count = 0;
    let mut pending_space = false;
    let mut truncated = false;

    for character in content.chars() {
        if character.is_whitespace() {
            pending_space = !preview.is_empty();
            continue;
        }

        if char_count == PREVIEW_CHAR_LIMIT {
            truncated = true;
            break;
        }
        if pending_space {
            preview.push(' ');
            char_count += 1;
            pending_space = false;
            if char_count == PREVIEW_CHAR_LIMIT {
                truncated = true;
                break;
            }
        }
        preview.push(character);
        char_count += 1;
    }

    if preview.is_empty() {
        return "空提问".to_string();
    }
    if truncated {
        preview.push('…');
    }
    preview
}

pub(crate) fn live_message_meta_fallback(
    role: ChatRole,
    is_requesting: bool,
    is_last: bool,
    model: &str,
) -> Option<String> {
    (is_requesting && is_last && matches!(role, ChatRole::Assistant | ChatRole::User))
        .then(|| format!("{model} · 刚刚"))
}

fn user_question_tooltip_content(label: String) -> Element<'static, Message> {
    container(text(label).size(11))
        .padding([4, 8])
        .style(|_theme: &Theme| iced::widget::container::Style {
            text_color: Some(Color::from_rgb8(0xF3, 0xF4, 0xF6)),
            background: Some(Background::Color(Color::from_rgb8(0x0B, 0x0B, 0x0B))),
            border: Border {
                width: 1.0,
                color: Color::from_rgb8(0x14, 0x14, 0x14),
                radius: 6.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.25),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            },
            ..Default::default()
        })
        .into()
}

/// 渲染聊天面板的主视图
///
/// 该函数是聊天面板的核心渲染入口，负责构建完整的聊天界面，
/// 包括消息列表、头部区域和滚动控制。
///
/// # 参数
///
/// - `app` - 应用状态引用，包含聊天消息、会话信息等运行时数据
///
/// # 返回值
///
/// 返回一个 [`Element`]，包含完整的聊天面板 UI 结构
///
/// # 布局结构
///
/// 当会话已开始（有活跃会话且消息列表非空）时：
/// ```text
/// +------------------+
/// |   聊天头部       |
/// +------------------+
/// |                  |
/// |   消息列表       |
/// |   (可滚动)       |
/// |                  |
/// | [跳转到底部按钮] |
/// +------------------+
/// ```
///
/// 当会话未开始时，显示空会话占位符
///
/// # 示例
///
/// ```ignore
/// let chat_panel = view(&app);
/// // 在主布局中使用
/// let main_view = container(chat_panel);
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let is_loading_session_ui = app.active_session_id.is_some()
        && app.active_session_view_state.ui_preparing
        && !app.active_session_view_state.base_ready;
    if is_loading_session_ui {
        return session_loading_panel(app);
    }

    let window = chat_panel_virtual_window(app);
    let start_idx = window.render_start_idx;
    let end_idx = window.render_end_idx;
    let top_spacer_h = window.top_spacer_h;
    let bottom_spacer_h = window.bottom_spacer_h;
    let visible_start_idx = window.visible_start_idx;
    let visible_end_idx = window.visible_end_idx;
    let question_visible_start_idx = window.visible_start_idx;
    let question_visible_end_idx = window.visible_end_idx;
    let user_question_markers = &app.active_session_view_state.user_question_markers;

    #[cfg(test)]
    performance::record_mounted_rows(end_idx.saturating_sub(start_idx));

    // 构建消息列，每条消息之间间隔 18px
    let mut col = chat_message_column();

    let runtime = app.current_session_runtime_ref();
    #[cfg(test)]
    performance::record_runtime_borrow();
    let runtime_is_requesting = runtime.is_some_and(|runtime| runtime.is_requesting);
    let runtime_model = runtime.map_or(app.model.as_str(), |runtime| runtime.model.as_str());

    // 遍历聊天消息并添加到列中
    for (i, m) in chat_render_window(&app.chat, start_idx, end_idx) {
        // 流式响应标志：当前消息是助手消息、正在请求中且是最后一条消息
        let is_last = i + 1 == app.chat.len();
        let _is_streaming = m.role == ChatRole::Assistant && runtime_is_requesting && is_last;
        let render_cache = app.chat_render_cache.get(&i).unwrap_or(&FALLBACK_RENDER_CACHE_ENTRY);
        let revision_key = app.paged_chat_timeline.as_ref().and_then(|timeline| {
            let target = timeline.target_at(i)?;
            Some(message_view::RevisionRenderKey::new(
                target.key,
                target.revision,
                app.window_size.0,
                self::utils::is_dark_theme(&app.theme()),
                app.dialogue_flow_show_reasoning_summary,
            ))
        });
        let enable_heavy_tool_content = i >= visible_start_idx && i < visible_end_idx;
        let message_meta = app
            .active_session_view_state
            .message_meta_texts
            .get(i)
            .and_then(|meta| meta.as_deref())
            .map(Cow::Borrowed)
            .or_else(|| {
                live_message_meta_fallback(m.role, runtime_is_requesting, is_last, runtime_model)
                    .map(Cow::Owned)
            });

        let message = message_view::message_view_with_runtime(
            app,
            i,
            m.role,
            &m.content,
            &m.think_timing,
            m.processing_timing.as_ref(),
            message_meta,
            render_cache,
            enable_heavy_tool_content,
            runtime_is_requesting,
            revision_key.as_ref(),
        );
        let target = app.paged_chat_timeline.as_ref().and_then(|timeline| timeline.target_at(i));
        col = col.push(message_measure::MeasuredChatMessage::new(i, message).with_target(target));
    }

    // 创建可滚动容器，包含消息列
    let scroll_content = scrollable(conversation_reading_column(
        chat_virtual_content(col, top_spacer_h, bottom_spacer_h),
        Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
    ))
    .direction(chat_scroll_direction())
    .id(app.chat_scroll_id.clone())
    // 监听滚动事件，更新滚动位置和视口高度
    .on_scroll(|v| {
        Message::Chat(message::ChatMessage::ScrollChanged {
            offset_y: v.relative_offset().y,
            absolute_offset_y: v.absolute_offset().y,
            viewport_h: v.bounds().height,
        })
    });

    // 判断会话是否已开始：存在活跃会话且消息列表非空
    let has_started = if app.active_session_id.is_some() { !app.chat.is_empty() } else { false };

    if has_started {
        let header = chat_header_view(app);
        let question_jump_button = user_question_nav_overlay(
            app,
            user_question_markers,
            question_visible_start_idx,
            question_visible_end_idx,
        );
        let jump_button = jump_to_bottom_button(app);
        let todo_overlay = chat_todo_overlay(app);
        let body_base: Element<'_, Message> = match app.chat_panel_mode {
            ChatPanelMode::Conversation => {
                stack![scroll_content, question_jump_button, jump_button, todo_overlay]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
            ChatPanelMode::Trajectory => trajectory::view(app),
        };
        let body_bg =
            app.active_theme_skin.as_ref().and_then(|skin| skin.chat_body_path.as_deref());
        let fit = app
            .active_theme_skin
            .as_ref()
            .map(|skin| {
                crate::app::theme_catalog::PanelImageFit::parse(
                    skin.chrome.chat_body_image_fit.as_deref(),
                )
                .unwrap_or_default()
            })
            .unwrap_or_default();
        let body = with_panel_background(body_base, body_bg, fit);
        let panel_base = column![header, body, chat_global_status_reading_column(app)]
            .spacing(6)
            .width(Length::Fill)
            .height(Length::Fill);
        chat_panel_context_menu_overlay(app, panel_base)
    } else {
        column![
            stack![
                conversation_reading_column(
                    empty_session_placeholder(app),
                    Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
                )
                .height(Length::Fill),
                empty_fullscreen_controls(app)
            ]
            .width(Length::Fill)
            .height(Length::Fill),
            chat_global_status_reading_column(app)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn chat_todo_overlay<'a>(app: &'a App) -> Element<'a, Message> {
    if app.chat_todo_placement != TodoPanelPlacement::ChatTopRight
        || app.chat_todo_session_id != app.active_session_id
        || app.chat_todo_items.is_empty()
    {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let submit_anim =
        app.current_session_runtime_ref().map(|runtime| runtime.submit_anim).unwrap_or(0);
    let panel = todo_panel::todo_panel(
        app,
        app.chat_todo_items.as_slice(),
        submit_anim,
        TodoPanelSurface::ChatTopRight,
    );

    container(opaque(panel))
        .padding(iced::Padding { top: 14.0, right: 22.0, bottom: 0.0, left: 0.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Top)
        .into()
}

pub fn tool_dialog_overlay(app: &App) -> Option<Element<'_, Message>> {
    self::tools::tool_detail_dialog_view(app)
}

#[allow(dead_code)]
pub(crate) fn refresh_virtualization_metrics(app: &mut App) {
    refine_message_heights(app);
}

pub(crate) fn rough_message_heights(chat: &[ChatMessage]) -> Vec<f32> {
    chat.iter().map(|message| estimate_message_height_rough(&message.content)).collect()
}

fn empty_fullscreen_controls<'a>(app: &'a App) -> Element<'a, Message> {
    let fullscreen_icon =
        if app.is_chat_fullscreen() { Icon::FullscreenExit } else { Icon::Fullscreen };
    let fullscreen_label = if app.is_chat_fullscreen() { "退出全屏" } else { "全屏" };

    let mut controls = row![].spacing(6).align_y(iced::Alignment::Center);
    if header::show_half_fullscreen_control(app.workspace_mode) {
        controls = controls.push(header::header_icon_button(
            Icon::LayoutTextWindow,
            if app.is_chat_half_fullscreen() { "退出半屏" } else { "半屏" },
            Message::Chat(message::ChatMessage::ToggleHalfFullscreen),
        ));
    }
    let controls = controls.push(header::header_icon_button(
        fullscreen_icon,
        fullscreen_label,
        Message::Chat(message::ChatMessage::ToggleFullscreen),
    ));

    container(controls)
        .width(Length::Fill)
        .height(Length::Shrink)
        .align_x(iced::alignment::Horizontal::Right)
        .padding(iced::Padding { top: 16.0, right: 22.0, bottom: 0.0, left: 0.0 })
        .into()
}

/// 创建"跳转到底部"按钮
///
/// 该函数渲染一个浮动按钮，当用户向上滚动离开最新消息时显示，
/// 点击可将视图滚动回消息列表底部。
///
/// # 参数
///
/// - `app` - 应用状态引用，用于检查滚动状态
///
/// # 返回值
///
/// 返回一个 [`Element`]，包含跳转按钮或不可见占位符
///
/// # 显示逻辑
///
/// 按钮在以下情况下**不显示**（返回高度为 0 的占位符）：
/// - 当前处于自动滚动模式（`chat_auto_scroll` 为 true）
/// - 视口高度为 0（视图尚未初始化）
///
/// # 视觉样式
///
/// 按钮呈现为圆形气泡，包含向下箭头图标：
/// - 位置：底部居中
/// - 尺寸：24x24 像素
/// - 深色模式：背景 #2B2B2B，边框 #3A3A3A
/// - 浅色模式：背景 #E6E7EA，边框 #D2D5DA
/// - 圆角：完全圆角（radius: 999）
/// - 阴影：黑色半透明，向下偏移 4px，模糊半径 10px
///
/// # 示例
///
/// ```ignore
/// let button = jump_to_bottom_button(&app);
/// // 在 stack 布局中与滚动内容叠加
/// let stack = stack![scroll_content, button];
/// ```
fn jump_to_bottom_button<'a>(app: &'a App) -> Element<'a, Message> {
    // 如果处于自动滚动模式或视口未初始化，不显示按钮
    if app.chat_auto_scroll || app.chat_scroll_viewport_h == 0.0 {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    // 创建向下箭头图标，使用主题文字颜色
    let chevron = icon_svg(Icon::ChevronDown)
        .style(|theme: &Theme, _status| svg::Style { color: Some(theme.palette().text) });

    // 创建圆形气泡容器，包含箭头图标
    let bubble = container(chevron)
        .width(Length::Fixed(30.0))
        .height(Length::Fixed(30.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|theme: &Theme| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let bg = if is_dark {
                Color::from_rgba8(24, 25, 29, 0.96)
            } else {
                Color::from_rgba8(252, 252, 253, 1.0)
            };
            let border = if is_dark {
                Color::from_rgba8(45, 48, 54, 0.94)
            } else {
                Color::from_rgba8(226, 231, 237, 1.0)
            };

            iced::widget::container::Style {
                background: Some(Background::Color(bg)),
                border: Border { width: 1.0, color: border, radius: 999.0.into() },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.08 }),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 18.0,
                },
                ..Default::default()
            }
        });

    let button = button(bubble)
        .padding(0)
        .width(Length::Fixed(30.0))
        .height(Length::Fixed(30.0))
        .style(iced::widget::button::text)
        .on_press(Message::Chat(message::ChatMessage::ScrollToBottom));

    // 最终容器：底部居中定位，底部留 12px 间距
    container(button)
        .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 18.0, left: 0.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Bottom)
        .into()
}

fn user_question_nav_overlay<'a>(
    app: &'a App,
    user_question_markers: &[UserQuestionMarker],
    visible_start_idx: usize,
    visible_end_idx: usize,
) -> Element<'a, Message> {
    if user_question_markers.is_empty() || app.chat_scroll_viewport_h <= 0.0 {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let mut dots = column![].spacing(5).align_x(iced::alignment::Horizontal::Center);
    let active_question_position = active_user_question_marker_position(
        user_question_markers,
        visible_start_idx,
        visible_end_idx,
        app.chat_scroll_viewport_h,
    );

    for marker_position in project_user_question_marker_positions(
        user_question_markers.len(),
        active_question_position,
    ) {
        let marker = &user_question_markers[marker_position];
        let is_active = active_question_position == Some(marker_position);
        let tooltip_label = format!("问题 {}: {}", marker_position + 1, marker.preview);

        let dot = container(Space::new().width(Length::Shrink).height(Length::Shrink))
            .width(Length::Fixed(if is_active { 7.0 } else { 5.0 }))
            .height(Length::Fixed(if is_active { 7.0 } else { 5.0 }))
            .style(move |theme: &Theme| {
                let primary = theme.extended_palette().primary.base.color;
                let muted = if theme.palette().background.r
                    + theme.palette().background.g
                    + theme.palette().background.b
                    < 1.5
                {
                    Color::from_rgba8(92, 97, 108, 0.92)
                } else {
                    Color::from_rgba8(162, 168, 178, 0.96)
                };
                iced::widget::container::Style {
                    background: Some(Background::Color(if is_active {
                        primary.scale_alpha(0.96)
                    } else {
                        muted
                    })),
                    border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                    ..Default::default()
                }
            });

        let bubble = container(dot)
            .width(Length::Fixed(15.0))
            .height(Length::Fixed(15.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center);

        let button = button(bubble)
            .padding(0)
            .width(Length::Fixed(15.0))
            .height(Length::Fixed(15.0))
            .style(iced::widget::button::text)
            .on_press(Message::Chat(message::ChatMessage::LocateChatMessageIndex(
                marker.message_idx,
            )));

        let button_with_tooltip: Element<'a, Message> = Tooltip::new(
            button,
            user_question_tooltip_content(tooltip_label),
            TooltipPosition::Left,
        )
        .gap(6)
        .into();

        dots = dots.push(button_with_tooltip);
    }

    let rail = container(dots).padding([8, 4]).style(|theme: &Theme| {
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        let bg = if is_dark {
            Color::from_rgba8(20, 22, 27, 0.84)
        } else {
            Color::from_rgba8(244, 246, 249, 0.92)
        };
        let border = if is_dark {
            Color::from_rgba8(42, 45, 52, 0.92)
        } else {
            Color::from_rgba8(225, 230, 237, 1.0)
        };

        iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            border: Border { width: 1.0, color: border, radius: 14.0.into() },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.08 }),
                offset: iced::Vector::new(0.0, 8.0),
                blur_radius: 18.0,
            },
            ..Default::default()
        }
    });

    container(rail)
        .padding(iced::Padding { top: 0.0, right: 7.0, bottom: 0.0, left: 0.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}
