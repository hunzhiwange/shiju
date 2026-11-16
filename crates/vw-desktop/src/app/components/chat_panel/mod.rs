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
#[path = "mod_tests.rs"]
mod mod_tests;

use iced::widget::Space;
use iced::{Element, Length};

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
    desktop_chat_view::panel::status_reading_column(
        message_view::recovery_notice_view(app),
        chat_global_status_view(app),
    )
}

pub(crate) fn chat_panel_virtual_window(app: &App) -> ChatVirtualWindow {
    app.chat_view.chat_virtualizer.window()
}

fn chat_panel_context_menu_overlay<'a>(
    app: &'a App,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let is_open = app.session_state.chat_context_menu.target.is_some();
    let menu = chat_context_menu(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), is_open)
        .unwrap_or_else(|| Space::new().into());
    desktop_chat_view::panel::context_menu_overlay(
        content,
        menu,
        is_open,
        app.session_state.chat_context_menu.position,
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

/// Assemble session snapshots and cross-domain view ports for the conversation.
pub fn view(app: &App) -> Element<'_, Message> {
    let phase = desktop_chat_view::panel::panel_phase(
        app.session_state.active_session_id.is_some(),
        app.chat_view.active_session_view_state.ui_preparing,
        app.chat_view.active_session_view_state.base_ready,
        app.chat_view.chat.len(),
    );
    if phase == desktop_chat_view::panel::PanelPhase::Loading {
        return session_loading_panel(app);
    }

    let window = chat_panel_virtual_window(app);
    let runtime = app.current_session_runtime_ref();
    #[cfg(test)]
    performance::record_runtime_borrow();
    let requesting = runtime.is_some_and(|r| r.request_lifecycle.is_requesting);
    let scroll_content = desktop_chat_view::panel::conversation_window(
        desktop_chat_view::panel::ConversationInput {
            chat: &app.chat_view.chat,
            cache: &app.chat_view.chat_render_cache,
            metadata: &app.chat_view.active_session_view_state.message_meta_texts,
            timeline: app.session_state.paged_chat_timeline.as_ref(),
            window,
            scroll_id: app.chat_view.chat_scroll_id.clone(),
            i18n: crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
            requesting,
            model: runtime.map_or(app.session_state.model.as_str(), |r| r.model.as_str()),
            width: app.window_size.0,
            dark: self::utils::is_dark_theme(&app.theme()),
            reasoning_summary: app.general_settings.dialogue_flow_show_reasoning_summary,
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

    if phase == desktop_chat_view::panel::PanelPhase::Started {
        let header = chat_header_view(app);
        let question_jump_button = user_question_nav_overlay(
            app,
            &app.chat_view.active_session_view_state.user_question_markers,
            window.visible_start_idx,
            window.visible_end_idx,
        );
        let jump_button = jump_to_bottom_button(app);
        let todo_overlay = chat_todo_overlay(app);
        let body_base: Element<'_, Message> = match app.chat_view.chat_panel_mode {
            ChatPanelMode::Conversation => desktop_chat_view::panel::conversation_body(
                scroll_content,
                question_jump_button,
                jump_button,
                todo_overlay,
            ),
            ChatPanelMode::Trajectory => trajectory::view(app),
        };
        let body_bg =
            app.appearance.active_theme_skin.as_ref().and_then(|skin| skin.chat_body_path.as_deref());
        let fit = app.appearance.active_theme_skin
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
    if !desktop_chat_view::panel::todo_is_visible(
        app.todo_state.chat_todo_placement == TodoPanelPlacement::ChatTopRight,
        app.todo_state.chat_todo_session_id.as_deref(),
        app.session_state.active_session_id.as_deref(),
        app.todo_state.chat_todo_items.len(),
    ) {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let submit_anim = app
        .current_session_runtime_ref()
        .map(|runtime| runtime.request_lifecycle.submit_anim)
        .unwrap_or(0);
    let panel = todo_panel::todo_panel(
        app,
        app.todo_state.chat_todo_items.as_slice(),
        submit_anim,
        TodoPanelSurface::ChatTopRight,
    );

    desktop_chat_view::panel::todo_overlay(panel)
}

pub fn tool_dialog_overlay(app: &App) -> Option<Element<'_, Message>> {
    self::tools::tool_detail_dialog_view(app)
}

fn empty_fullscreen_controls(app: &App) -> Element<'_, Message> {
    desktop_chat_view::panel::empty_fullscreen_controls(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        app.is_chat_fullscreen(),
        app.is_chat_half_fullscreen(),
        header::show_half_fullscreen_control(app.navigation_mode.workspace_mode),
        Message::Chat(message::ChatMessage::ToggleFullscreen),
        Message::Chat(message::ChatMessage::ToggleHalfFullscreen),
    )
}

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
        app.chat_view.chat_auto_scroll,
        app.chat_view.chat_scroll_viewport_h,
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
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        app.chat_view.chat_scroll_viewport_h,
        markers,
        start,
        end,
    )
    .map(panel_message)
}

pub(crate) use desktop_chat_view::panel::rough_message_heights;
#[cfg(test)]
pub(crate) use desktop_chat_view::panel::user_question_indices;
