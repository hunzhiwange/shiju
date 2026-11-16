//! Bounded UI preparation tasks; the desktop maps events to its project route.
use iced::Task;
pub enum UiEvent {
    None,
    Prepared {
        session_id: String,
        stream_generation: Option<u64>,
        phase: crate::chat_ui::PreparedChatUiPhase,
    },
}
async fn spawn_blocking_opt<F, T>(f: F) -> Option<T>
where
    F: FnOnce() -> Option<T> + Send + 'static,
    T: Send + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        f()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(f).await.unwrap_or(None)
    }
}
struct ChatUiChunkSnapshot {
    chunk_start_idx: usize,
    messages: desktop_session::session::loading::SharedChatMessages,
}

fn chat_ui_chunk_snapshot(
    chat: &desktop_session::session::loading::SharedChatMessages,
    chunk_start_idx: usize,
) -> Option<ChatUiChunkSnapshot> {
    let (chunk_start_idx, chunk_end_idx) =
        crate::chat_ui::chat_ui_chunk_bounds(chat.len(), chunk_start_idx);
    (chunk_start_idx < chunk_end_idx).then(|| ChatUiChunkSnapshot {
        chunk_start_idx,
        messages: std::sync::Arc::from(&chat[chunk_start_idx..chunk_end_idx]),
    })
}

pub fn prepare_session_ui_task(
    session_id: String,
    chat: desktop_session::session::loading::SharedChatMessages,
    chunk_start_idx: usize,
    is_base: bool,
    show_reasoning_summary: bool,
) -> Task<UiEvent> {
    let Some(snapshot) = chat_ui_chunk_snapshot(&chat, chunk_start_idx) else {
        return Task::done(UiEvent::None);
    };
    Task::perform(
        async move {
            spawn_blocking_opt(move || {
                Some((
                    session_id,
                    crate::chat_ui::prepare_chat_ui_chunk_phase(
                        &snapshot.messages,
                        snapshot.chunk_start_idx,
                        is_base,
                        show_reasoning_summary,
                    ),
                ))
            })
            .await
        },
        |res| match res {
            Some((session_id, phase)) => {
                UiEvent::Prepared { session_id, stream_generation: None, phase }
            }
            None => UiEvent::None,
        },
    )
}

pub fn prepare_stream_message_ui_task(
    session_id: String,
    message: session_ui_types::ChatMessage,
    message_idx: usize,
    stream_generation: u64,
    show_reasoning_summary: bool,
) -> Task<UiEvent> {
    Task::perform(
        async move {
            spawn_blocking_opt(move || {
                Some((
                    session_id,
                    stream_generation,
                    crate::chat_ui::prepare_chat_ui_message_phase(
                        &message,
                        message_idx,
                        show_reasoning_summary,
                    ),
                ))
            })
            .await
        },
        |res| match res {
            Some((session_id, stream_generation, phase)) => {
                UiEvent::Prepared { session_id, stream_generation: Some(stream_generation), phase }
            }
            None => UiEvent::None,
        },
    )
}

/// prepare_session_ui_chunks_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub fn prepare_session_ui_chunks_task(
    session_id: String,
    chat: desktop_session::session::loading::SharedChatMessages,
    chunk_starts: Vec<usize>,
    base_chunk_start: Option<usize>,
    show_reasoning_summary: bool,
) -> Task<UiEvent> {
    if chunk_starts.is_empty() {
        return Task::none();
    }

    Task::batch(chunk_starts.into_iter().map(|chunk_start_idx| {
        prepare_session_ui_task(
            session_id.clone(),
            chat.clone(),
            chunk_start_idx,
            base_chunk_start == Some(chunk_start_idx),
            show_reasoning_summary,
        )
    }))
}

#[cfg(test)]
#[path = "preparation_tasks_tests.rs"]
mod tests;
