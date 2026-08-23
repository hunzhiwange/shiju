//! 维护尾部助手消息的处理耗时与模型调用耗时记录。

use crate::app::models::{ChatMessage, ChatRole};
use vw_shared::session::ui_types::ProcessingTiming;

/// 仅为尚未计时的尾部助手消息启动处理计时。
pub(super) fn start_tail_assistant_processing(
    messages: &mut [ChatMessage],
    started_ms: u64,
) -> bool {
    let Some(message) = messages.last_mut() else {
        return false;
    };
    if message.role != ChatRole::Assistant || message.processing_timing.is_some() {
        return false;
    }

    message.processing_timing = Some(ProcessingTiming {
        started_ms,
        completed_ms: None,
        processing_elapsed_ms: Some(0),
        llm_elapsed_ms: Some(0),
    });
    true
}

/// 完成尾部助手消息计时；重复完成或消息不匹配时不修改状态。
pub(super) fn complete_tail_assistant_processing(
    messages: &mut [ChatMessage],
    completed_ms: u64,
    processing_elapsed_ms: u64,
    llm_elapsed_ms: u64,
) -> bool {
    let Some(message) = messages.last_mut() else {
        return false;
    };
    if message.role != ChatRole::Assistant {
        return false;
    }
    let Some(timing) = message.processing_timing.as_mut() else {
        return false;
    };
    if timing.completed_ms.is_some() {
        return false;
    }

    timing.completed_ms = Some(completed_ms);
    timing.processing_elapsed_ms = Some(processing_elapsed_ms);
    timing.llm_elapsed_ms = Some(llm_elapsed_ms);
    true
}

/// 暂停运行时计时器时截取的累计耗时。
pub(super) struct ProcessingElapsed {
    /// 整体处理阶段累计毫秒数。
    pub(super) processing_ms: u64,
    /// 模型调用阶段累计毫秒数。
    pub(super) llm_ms: u64,
}

/// 使用同一时间点暂停两个计时器，避免分别采样造成耗时偏差。
pub(super) fn pause_runtime_processing_timers(
    runtime: &mut crate::app::state::SessionRuntimeState,
) -> ProcessingElapsed {
    let now = web_time::Instant::now();
    runtime.llm_timer.pause_at(now);
    runtime.processing_timer.pause_at(now);
    ProcessingElapsed {
        processing_ms: runtime.processing_timer.elapsed_ms_at(now),
        llm_ms: runtime.llm_timer.elapsed_ms_at(now),
    }
}
