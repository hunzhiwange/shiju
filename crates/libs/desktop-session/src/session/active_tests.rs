use super::active::*;
use session_ui_types::{ChatMessage, ChatRole, ChatSessionStep, TokenUsage};
use std::collections::HashMap;
fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn message_with_model(role: ChatRole, content: &str, model: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: Some(model.to_string()),
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn step(index: u32, started_ms: u64, finished_ms: Option<u64>, model: &str) -> ChatSessionStep {
    ChatSessionStep {
        index,
        turn_id: None,
        started_ms,
        finished_ms,
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: None,
        model: Some(model.to_string()),
        tool_timings: Vec::new(),
    }
}

#[test]
fn message_meta_labels_use_step_start_for_user_and_finish_for_assistant() {
    let chat = vec![
        message_with_model(ChatRole::User, "question", "message-model"),
        message(ChatRole::System, "context"),
        message_with_model(ChatRole::Assistant, "answer", "message-model"),
        message(ChatRole::Tool, "result"),
    ];
    let step = step(1, 1_000, Some(1_500), "model-a");
    let step_index_map = HashMap::from([(1, StepUiMeta::from(&step))]);

    let labels = build_message_meta_labels(&chat, &step_index_map)
        .into_iter()
        .map(|meta| {
            meta.map(|meta| {
                meta.render(desktop_i18n::I18n::new(
                    config_ui::DesktopLocale::SimplifiedChinese,
                ))
            })
        })
        .collect::<Vec<_>>();

    let started = desktop_i18n::chat_time::format_chat_time_label(1_000);
    let finished = desktop_i18n::chat_time::format_chat_time_label(1_500);
    assert_eq!(
        labels,
        vec![
            Some(format!("message-model · {started}")),
            None,
            Some(format!("message-model · {finished}")),
            None,
        ]
    );
}

#[test]
fn message_meta_labels_omit_missing_models_but_keep_assistant_step_alignment() {
    let chat = vec![
        message(ChatRole::Assistant, "first"),
        message_with_model(ChatRole::User, "second question", "message-model-b"),
        message_with_model(ChatRole::Assistant, "second answer", "message-model-b"),
    ];
    let mut first = step(1, 1_000, Some(1_100), "unused");
    first.model = None;
    let second = step(2, 2_000, Some(2_100), "model-b");
    let step_index_map =
        HashMap::from([(1, StepUiMeta::from(&first)), (2, StepUiMeta::from(&second))]);

    let labels = build_message_meta_labels(&chat, &step_index_map)
        .into_iter()
        .map(|meta| {
            meta.map(|meta| {
                meta.render(desktop_i18n::I18n::new(
                    config_ui::DesktopLocale::SimplifiedChinese,
                ))
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(labels[0], None);
    assert!(labels[1].as_deref().is_some_and(|label| label.contains("message-model-b")));
    assert!(labels[2].as_deref().is_some_and(|label| label.contains("message-model-b")));
}

#[test]
fn task_session_source_display_uses_vibewindow_label() {
    let content = session_text::encode_task_session_source_marker(
        "source-session",
        "跨任务问题",
    )
    .expect("source marker should encode");
    let chat = vec![message(ChatRole::User, &content), message(ChatRole::User, "普通问题")];

    let labels = build_message_meta_labels(&chat, &HashMap::new())
        .into_iter()
        .map(|meta| {
            meta.map(|meta| {
                meta.render(desktop_i18n::I18n::new(
                    config_ui::DesktopLocale::SimplifiedChinese,
                ))
            })
        })
        .collect::<Vec<_>>();
    let markers = build_user_question_markers(&chat);

    assert_eq!(labels, vec![Some("由 VibeWindow 从另一项任务发起".to_string()), None]);
    assert_eq!(markers[0].preview, "跨任务问题");
    assert_eq!(markers[1].preview, "普通问题");
}
