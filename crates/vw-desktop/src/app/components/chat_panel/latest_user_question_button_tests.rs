//! 验证最新用户问题按钮行为。
//! 测试确保按钮只在合适状态出现并指向正确消息。

use super::{
    active_user_question_idx, is_chat_message_idx_visible, live_message_meta_fallback,
    user_question_indices,
};
use crate::app::models::ChatRole;

#[test]
fn chat_message_idx_visible_returns_true_inside_window() {
    assert!(is_chat_message_idx_visible(5, 4, 7, 420.0));
}

#[test]
fn chat_message_idx_visible_returns_false_outside_window() {
    assert!(!is_chat_message_idx_visible(3, 4, 7, 420.0));
}

#[test]
fn chat_message_idx_visible_returns_false_without_viewport() {
    assert!(!is_chat_message_idx_visible(3, 0, 4, 0.0));
    assert!(!is_chat_message_idx_visible(3, 4, 4, 420.0));
}

#[test]
fn active_user_question_idx_returns_first_visible_question() {
    let user_question_idxs = [0, 2, 4];

    assert_eq!(active_user_question_idx(&user_question_idxs, 0, 3, 420.0), Some(0));
}

#[test]
fn active_user_question_idx_returns_none_without_visible_question() {
    let user_question_idxs = [0, 2, 4];

    assert_eq!(active_user_question_idx(&user_question_idxs, 5, 6, 420.0), None);
}

#[test]
fn user_question_indices_preserve_question_order() {
    let chat = vec![
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::User,
            content: "问题一".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::Assistant,
            content: "回答一".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::User,
            content: "问题二".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    assert_eq!(user_question_indices(&chat), vec![0, 2]);
}

#[test]
fn live_message_meta_fallback_applies_to_chat_roles() {
    assert_eq!(
        live_message_meta_fallback(ChatRole::Assistant, true, true, "model-b"),
        Some("model-b · 刚刚".to_string())
    );
    assert_eq!(
        live_message_meta_fallback(ChatRole::Assistant, true, false, "model-b"),
        Some("model-b · 刚刚".to_string())
    );
    assert_eq!(
        live_message_meta_fallback(ChatRole::User, true, true, "model-b"),
        Some("model-b · 刚刚".to_string())
    );
    assert_eq!(live_message_meta_fallback(ChatRole::System, true, true, "model-b"), None);
}
