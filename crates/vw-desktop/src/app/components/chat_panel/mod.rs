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
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "performance_tests.rs"]
mod performance_tests;
#[cfg(test)]
#[path = "tool_selector_tests.rs"]
mod tool_selector_tests;

use iced::widget::{Space, column};
use iced::{Element, Length, Padding};

use crate::app::components::input_panel::todo_panel::{self, TodoPanelSurface};
use crate::app::state::{ChatPanelMode, UserQuestionMarker};
use crate::app::theme_catalog::with_panel_background;
use crate::app::{App, Message, TodoPanelPlacement, message};

use self::empty::{empty_session_placeholder, session_loading_placeholder};
use self::global_status::chat_global_status_view;
use self::header::chat_header_view;

use self::utils::chat_context_menu;
use self::virtualizer::ChatVirtualWindow;

fn chat_global_status_reading_column(app: &App) -> Element<'_, Message> {
    conversation_reading_column(
        column![message_view::recovery_notice_view(app), chat_global_status_view(app)].spacing(6),
        Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
    )
    .into()
}

pub(crate) fn chat_panel_virtual_window(app: &App) -> ChatVirtualWindow {
    app.chat_virtualizer.window()
}

fn chat_panel_context_menu_overlay<'a>(
    app: &'a App,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let is_open = app.chat_context_menu.target.is_some();
    let menu = chat_context_menu(crate::app::i18n::I18n::new(app.desktop_locale), is_open)
        .unwrap_or_else(|| Space::new().into());
    desktop_chat_view::panel::context_menu_overlay(
        content,
        menu,
        is_open,
        app.chat_context_menu.position,
        Message::Chat(message::ChatMessage::CloseMessageContextMenu),
    )
}

fn session_loading_panel(app: &App) -> Element<'_, Message> {
    desktop_chat_view::panel::placeholder_view(
        session_loading_placeholder(app),
        empty_fullscreen_controls(app),
        chat_global_status_reading_column(app),
    )
}

#[allow(dead_code)]
fn refine_message_heights(app: &mut App) {
    app.refine_chat_message_estimated_heights(0, app.chat.len());
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
    let runtime = app.current_session_runtime_ref();
    #[cfg(test)]
    performance::record_runtime_borrow();
    let requesting = runtime.is_some_and(|r| r.request_lifecycle.is_requesting);
    let scroll_content = desktop_chat_view::panel::conversation_window(
        desktop_chat_view::panel::ConversationInput {
            chat: &app.chat,
            cache: &app.chat_render_cache,
            metadata: &app.active_session_view_state.message_meta_texts,
            timeline: app.paged_chat_timeline.as_ref(),
            window,
            scroll_id: app.chat_scroll_id.clone(),
            i18n: crate::app::i18n::I18n::new(app.desktop_locale),
            requesting,
            model: runtime.map_or(app.model.as_str(), |r| r.model.as_str()),
            width: app.window_size.0,
            dark: self::utils::is_dark_theme(&app.theme()),
            reasoning_summary: app.dialogue_flow_show_reasoning_summary,
        },
        |i, m, meta, cache, heavy, revision| {
            message_view::message_view_with_runtime(
                app,
                i,
                m.role,
                &m.content,
                &m.think_timing,
                m.processing_timing.as_ref(),
                meta,
                cache,
                heavy,
                requesting,
                revision,
            )
        },
        |v| {
            Message::Chat(message::ChatMessage::ScrollChanged {
                offset_y: v.relative_offset().y,
                absolute_offset_y: v.absolute_offset().y,
                viewport_h: v.bounds().height,
            })
        },
    );

    // 判断会话是否已开始：存在活跃会话且消息列表非空
    let has_started = if app.active_session_id.is_some() { !app.chat.is_empty() } else { false };

    if has_started {
        let header = chat_header_view(app);
        let question_jump_button = user_question_nav_overlay(
            app,
            &app.active_session_view_state.user_question_markers,
            window.visible_start_idx,
            window.visible_end_idx,
        );
        let jump_button = jump_to_bottom_button(app);
        let todo_overlay = chat_todo_overlay(app);
        let body_base: Element<'_, Message> = match app.chat_panel_mode {
            ChatPanelMode::Conversation => desktop_chat_view::panel::conversation_body(
                scroll_content,
                question_jump_button,
                jump_button,
                todo_overlay,
            ),
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
        let panel_base = desktop_chat_view::panel::started_view(
            header,
            body,
            chat_global_status_reading_column(app),
        );
        chat_panel_context_menu_overlay(app, panel_base)
    } else {
        desktop_chat_view::panel::placeholder_view(
            empty_session_placeholder(app),
            empty_fullscreen_controls(app),
            chat_global_status_reading_column(app),
        )
    }
}

fn chat_todo_overlay<'a>(app: &'a App) -> Element<'a, Message> {
    if app.chat_todo_placement != TodoPanelPlacement::ChatTopRight
        || app.chat_todo_session_id != app.active_session_id
        || app.chat_todo_items.is_empty()
    {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let submit_anim = app
        .current_session_runtime_ref()
        .map(|runtime| runtime.request_lifecycle.submit_anim)
        .unwrap_or(0);
    let panel = todo_panel::todo_panel(
        app,
        app.chat_todo_items.as_slice(),
        submit_anim,
        TodoPanelSurface::ChatTopRight,
    );

    desktop_chat_view::panel::todo_overlay(panel)
}

pub fn tool_dialog_overlay(app: &App) -> Option<Element<'_, Message>> {
    self::tools::tool_detail_dialog_view(app)
}

#[allow(dead_code)]
pub(crate) fn refresh_virtualization_metrics(app: &mut App) {
    refine_message_heights(app);
}

fn empty_fullscreen_controls(app: &App) -> Element<'_, Message> {
    desktop_chat_view::panel::empty_fullscreen_controls(
        crate::app::i18n::I18n::new(app.desktop_locale),
        app.is_chat_fullscreen(),
        app.is_chat_half_fullscreen(),
        header::show_half_fullscreen_control(app.navigation_mode.workspace_mode),
        Message::Chat(message::ChatMessage::ToggleFullscreen),
        Message::Chat(message::ChatMessage::ToggleHalfFullscreen),
    )
}

/// 创建跳转到底部按钮
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
fn panel_message(event: desktop_chat_view::panel::Message) -> Message {
    Message::Chat(match event {
        desktop_chat_view::panel::Message::ScrollToBottom => message::ChatMessage::ScrollToBottom,
        desktop_chat_view::panel::Message::LocateChatMessageIndex(index) => {
            message::ChatMessage::LocateChatMessageIndex(index)
        }
    })
}
fn jump_to_bottom_button(app: &App) -> Element<'_, Message> {
    desktop_chat_view::panel::jump_to_bottom_button(
        app.chat_auto_scroll,
        app.chat_scroll_viewport_h,
    )
    .map(panel_message)
}
fn user_question_nav_overlay<'a>(
    app: &'a App,
    markers: &[UserQuestionMarker],
    start: usize,
    end: usize,
) -> Element<'a, Message> {
    desktop_chat_view::panel::user_question_nav_overlay(
        crate::app::i18n::I18n::new(app.desktop_locale),
        app.chat_scroll_viewport_h,
        markers,
        start,
        end,
    )
    .map(panel_message)
}

#[cfg(test)]
pub(crate) use desktop_chat_view::panel::user_question_indices;
pub(crate) use desktop_chat_view::panel::{
    chat_message_column, chat_render_window, chat_virtual_content, conversation_reading_column,
    live_message_meta_fallback, rough_message_heights,
};
