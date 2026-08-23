//! 验证助手消息处理与模型调用计时的启动、完成和拒绝边界。
//!
//! 计时夹具只允许尾部助手消息启动周期，重复启动和非助手尾部必须保持既有计时。

use crate::app::models::{ChatMessage, ChatRole};

fn message(role: ChatRole) -> ChatMessage {
    ChatMessage {
        role,
        content: String::new(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn processing_timing_starts_and_completes_one_tail_assistant_cycle() {
    let mut messages = vec![message(ChatRole::User), message(ChatRole::Assistant)];

    assert!(super::processing_timing::start_tail_assistant_processing(&mut messages, 100));
    assert!(!super::processing_timing::start_tail_assistant_processing(&mut messages, 110));
    assert_eq!(
        messages[1].processing_timing,
        Some(vw_shared::session::ui_types::ProcessingTiming {
            started_ms: 100,
            completed_ms: None,
            processing_elapsed_ms: Some(0),
            llm_elapsed_ms: Some(0),
        })
    );

    assert!(super::processing_timing::complete_tail_assistant_processing(
        &mut messages,
        175,
        90,
        75,
    ));
    assert!(!super::processing_timing::complete_tail_assistant_processing(
        &mut messages,
        180,
        100,
        80,
    ));
    assert_eq!(
        messages[1].processing_timing,
        Some(vw_shared::session::ui_types::ProcessingTiming {
            started_ms: 100,
            completed_ms: Some(175),
            processing_elapsed_ms: Some(90),
            llm_elapsed_ms: Some(75),
        })
    );
}

#[test]
fn processing_timing_rejects_non_assistant_tail_without_changing_earlier_cycle() {
    let mut messages = vec![message(ChatRole::Assistant), message(ChatRole::User)];

    assert!(!super::processing_timing::start_tail_assistant_processing(&mut messages, 100));
    assert!(!super::processing_timing::complete_tail_assistant_processing(
        &mut messages,
        175,
        90,
        75,
    ));
    assert!(messages.iter().all(|message| message.processing_timing.is_none()));
}

#[test]
fn llm_timer_terminal_paths_persist_accumulated_elapsed() {
    let mut messages = vec![message(ChatRole::Assistant)];
    assert!(super::processing_timing::start_tail_assistant_processing(&mut messages, 1_000));

    assert!(super::processing_timing::complete_tail_assistant_processing(
        &mut messages,
        9_000,
        8_000,
        2_750,
    ));

    assert_eq!(
        messages[0].processing_timing.as_ref().and_then(|timing| timing.processing_elapsed_ms),
        Some(8_000)
    );
    assert_eq!(
        messages[0].processing_timing.as_ref().and_then(|timing| timing.llm_elapsed_ms),
        Some(2_750)
    );
}
