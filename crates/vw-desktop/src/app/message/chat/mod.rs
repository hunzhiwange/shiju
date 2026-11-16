//! 聊天模块主入口
//!
//! 本模块负责处理所有聊天相关的消息和状态更新，包括：
//! - 用户输入处理（文本输入、编辑器操作、文件引用）
//! - Agent 流式响应（消息增量更新、步骤状态、Token 统计）
//! - 任务模式管理（子任务创建、优先级调整、执行器选择）
//! - 会话控制（消息发送、取消、队列管理）
//! - 交互功能（思考块展开、工具调用、问题应答）
//!
//! # 子模块结构
//!
//! - `input`：处理用户输入相关的消息（文本编辑、文件搜索、任务模式输入）
//! - `stream`：处理 Agent 流式响应相关的消息（增量更新、步骤状态、错误处理）
//! - `context`：处理上下文插入相关的消息（文件路径、代码片段、位置选择）
//! - `session`：处理会话管理相关的消息（发送、取消、队列操作、模型选择）

use crate::app::components::chat_panel::message_measure::{
    ChatMessageMeasurement, collect_visible_chat_message_measurements,
};
use crate::app::{App, Message, TodoPanelPlacement, message};
use iced::Task;
use iced::widget::scrollable;
use std::time::Duration;

mod acp_options;
pub mod context;
pub mod input;
mod processing_timing;
pub mod session;
pub mod stream;
mod trajectory;

#[cfg(test)]
#[path = "trajectory_tests.rs"]
mod trajectory_tests;

use desktop_session::message::chat::orchestration;
#[cfg(test)]
use orchestration::AUTOSCROLL_HOLD_MS;
use orchestration::think_block_key;

pub(crate) fn refresh_active_trajectory_raw_task(app: &mut App, session_id: &str) -> Task<Message> {
    if app.chat_panel_mode != crate::app::state::ChatPanelMode::Trajectory {
        return Task::none();
    }
    trajectory::refresh_missing_raw_task(app, session_id, None)
}

fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

pub(super) fn arm_autoscroll_hold(app: &mut App) {
    orchestration::arm_autoscroll_hold(
        &mut app.chat_autoscroll_generation,
        &mut app.chat_auto_scroll,
        &mut app.chat_autoscroll_hold_until_ms,
        now_ms(),
    );
}

pub(super) fn focus_chat_on_turn(app: &mut App, user_index: usize) -> bool {
    if user_index >= app.chat_virtualizer.len() {
        return false;
    }

    app.chat_auto_scroll = false;
    app.chat_virtualizer.focus_turn_on_user(user_index);
    app.sync_chat_virtualizer_compat_fields();
    true
}

pub(super) fn scroll_chat_to_bottom_task(app: &mut App) -> Task<Message> {
    arm_autoscroll_hold(app);
    app.chat_virtualizer.snap_to_tail();
    // Scrolling changes the offset, not message heights. Rebuilding the compatibility
    // height index here turns every streaming tail-follow into a full-history update.
    app.sync_chat_scroll_compat_fields();
    iced::widget::operation::snap_to(
        app.chat_scroll_id.clone(),
        scrollable::RelativeOffset { x: Some(0.0), y: Some(1.0) },
    )
    .map(|_: ()| Message::CopyDone)
}

pub(super) fn scroll_chat_to_turn_top_task(app: &mut App, user_index: usize) -> Task<Message> {
    if !focus_chat_on_turn(app, user_index) {
        return Task::none();
    }

    let relative_y = app.chat_virtualizer.relative_y();

    Task::batch([
        iced::widget::operation::snap_to(
            app.chat_scroll_id.clone(),
            scrollable::RelativeOffset { x: Some(0.0), y: Some(relative_y) },
        )
        .map(|_: ()| Message::CopyDone),
        measure_visible_chat_messages_task(),
    ])
}

pub(super) fn throttled_stream_autoscroll_task(app: &mut App) -> Option<Task<Message>> {
    if !orchestration::stream_autoscroll_due(
        app.chat_auto_scroll, &mut app.chat_stream_autoscroll_last_ms, now_ms(),
    ) {
        return None;
    }
    Some(scroll_chat_to_bottom_task(app))
}

pub(super) fn scroll_chat_to_bottom_with_followups(app: &mut App) -> Task<Message> {
    arm_autoscroll_hold(app);
    app.chat_virtualizer.snap_to_tail();
    app.sync_chat_virtualizer_compat_fields();
    Task::batch([
        iced::widget::operation::snap_to(
            app.chat_scroll_id.clone(),
            scrollable::RelativeOffset { x: Some(0.0), y: Some(1.0) },
        )
        .map(|_: ()| Message::CopyDone),
        message::after(
            Duration::from_millis(16),
            Message::Chat(ChatMessage::FollowupScrollToBottom {
                session_id: app.active_session_id.clone(),
                generation: app.chat_autoscroll_generation,
            }),
        ),
        message::after(
            Duration::from_millis(48),
            Message::Chat(ChatMessage::FollowupScrollToBottom {
                session_id: app.active_session_id.clone(),
                generation: app.chat_autoscroll_generation,
            }),
        ),
        message::after(
            Duration::from_millis(96),
            Message::Chat(ChatMessage::FollowupScrollToBottom {
                session_id: app.active_session_id.clone(),
                generation: app.chat_autoscroll_generation,
            }),
        ),
        message::after(
            Duration::from_millis(160),
            Message::Chat(ChatMessage::FollowupScrollToBottom {
                session_id: app.active_session_id.clone(),
                generation: app.chat_autoscroll_generation,
            }),
        ),
    ])
}

pub(super) fn measure_visible_chat_messages_task() -> Task<Message> {
    iced::advanced::widget::operate(collect_visible_chat_message_measurements())
        .map(|measurements| Message::Chat(ChatMessage::VisibleMessageMeasurements(measurements)))
}

pub use desktop_session::message::chat::input::clipboard::ClipboardPastePayload;

pub use desktop_session::message::chat::events::ForkSessionTarget;

pub use desktop_session::message::chat::fork::{ForkSessionAnchor, ForkSessionDialog};

pub(crate) fn fork_session_anchor_from_user_row(app: &App, index: usize) -> Option<ForkSessionAnchor> {
    orchestration::fork_session_anchor(
        &app.chat,
        &app.chat_message_ids,
        index,
        app.active_session_id.as_deref(),
        app.active_session_id.as_deref().and_then(|id| app.known_session_directory(id)),
        app.session_view_cache.open_generation(),
    )
}

pub(crate) fn fork_session_anchor_is_current(anchor: &ForkSessionAnchor, app: &App) -> bool {
    orchestration::fork_session_anchor_is_current(
        anchor,
        app.active_session_id.as_deref(),
        app.session_view_cache.open_generation(),
        app.known_session_directory(&anchor.session_id).as_deref(),
    )
}

pub type ForkSessionLoaded = desktop_session::message::chat::events::ForkSessionLoaded<
    crate::app::session::PreparedSessionOpen,
>;

pub type ChatMessage = desktop_session::message::chat::events::ChatMessage<
    ChatMessageMeasurement,
    crate::app::session::PreparedSessionOpen,
    crate::app::state::ChatPanelMode,
    crate::app::state::TrajectoryDetailsTab,
    TodoPanelPlacement,
    crate::app::state::QuickMessageGitCommitAction,
>;

/// 处理聊天消息并更新应用状态
///
/// 这是聊天模块的核心消息处理函数，负责：
/// 1. 根据消息类型路由到对应的子模块处理函数
/// 2. 处理流式响应的特殊逻辑（思考块状态同步）
/// 3. 同步消息编辑器状态
///
/// # 参数
///
/// - `app`：应用状态的可变引用，包含所有聊天相关的数据
/// - `message`：要处理的聊天消息
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，可能包含需要执行的异步操作或 UI 命令
///
/// # 消息路由策略
///
/// 消息被分为四大类，由对应的子模块处理：
///
/// - **输入类**（`input::update`）：处理用户界面交互，如文本输入、悬停状态、滚动等
/// - **流式响应类**（`stream::update`）：处理 Agent 的实时响应流，包括增量更新和步骤状态
/// - **上下文插入类**（`context::update`）：处理文件引用和代码片段的插入操作
/// - **会话管理类**（`session::update`）：处理消息发送、取消、队列管理、模型选择等
///
/// # 流式响应的特殊处理
///
/// 对于 `AgentStreamDelta` 消息，函数会额外执行以下操作：
///
/// 1. 读取流式尾状态中的思考块数量和开闭状态
/// 2. 跟踪当前正在展开的思考块索引
/// 3. 清理已删除思考块的编辑器状态
/// 4. 自动将思考块滚动到底部以显示最新内容
///
/// # 示例
///
/// ```ignore
/// let task = update(&mut app, ChatMessage::InputChanged("Hello".to_string()));
/// // task 可能包含 UI 更新命令
/// ```
pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    let local_ui_event = input::is_composer_event(&message)
        || matches!(
            &message,
            ChatMessage::SubmitTick | ChatMessage::TodoAnimTick | ChatMessage::SessionSaveAck
        );
    let is_stream_delta = message.tracks_stream_thinking();

    // 根据消息类型路由到对应的子模块处理函数
    let mut task = match message.route() {
        desktop_session::message::chat::routing::Route::AcpOptions => acp_options::update(app, message),
        desktop_session::message::chat::routing::Route::Input => input::update(app, message),
        desktop_session::message::chat::routing::Route::Trajectory => trajectory::update(app, message),
        desktop_session::message::chat::routing::Route::Stream => stream::update(app, message),
        desktop_session::message::chat::routing::Route::Context => context::update(app, message),
        desktop_session::message::chat::routing::Route::Session => session::update(app, message),
    };

    if app.input_popovers.agent_sessions
        && crate::app::components::input_panel::agent_sessions::cached_running_agent_sessions(app)
            .is_empty()
    {
        app.input_popovers.agent_sessions = false;
    }

    if local_ui_event {
        return task;
    }

    // 对流式增量消息执行特殊处理：同步思考块状态
    if is_stream_delta {
        let stream_tail = app.active_session_id.as_deref().and_then(|session_id| {
            let state = &app.session_runtime_states.get(session_id)?.stream_tail_state;
            state
                .message_idx()
                .map(|message_idx| (message_idx, state.think_count(), state.thinking_open()))
        });
        if let Some((i, think_count, thinking_open)) = stream_tail {
            let outcome = orchestration::sync_stream_think_tracking(
                &mut app.chat_stream_think_msg_idx,
                &mut app.chat_stream_think_count,
                &mut app.chat_stream_think_open_idx,
                i, think_count, thinking_open,
            );
            for think_idx in outcome.removed_indexes {
                let key = think_block_key(i, think_idx);
                app.chat_think_editors.remove(&key);
                app.chat_think_expanded.remove(&key);
                app.chat_think_collapsed.remove(&key);
            }

            if outcome.scroll_to_open_block
                && let Some(scroll_task) = throttled_stream_autoscroll_task(app)
            {
                task = task.chain(scroll_task);
            }
        }
    } else {
        let (visible_start_idx, visible_end_idx) = app.visible_chat_message_window();
        let pending_chunk_starts =
            app.pending_chat_message_editors_window_chunks(visible_start_idx, visible_end_idx);
        if !pending_chunk_starts.is_empty()
            && let Some(session_id) = app.active_session_id.clone()
        {
            app.mark_chat_ui_chunks_preparing(&pending_chunk_starts);
            task = task.chain(message::project::prepare_session_ui_chunks_task(
                session_id,
                app.active_shared_chat_messages(),
                pending_chunk_starts,
                None,
                app.dialogue_flow_show_reasoning_summary,
            ));
        }
    }

    task
}
#[cfg(test)]
#[path = "input_tests.rs"]
mod input_tests;
#[cfg(test)]
mod tests;
