// 本文件验证 Telegram 消息拆分、确认表情选择与请求字段保留。

use super::message_utils::{
    TELEGRAM_ACK_REACTIONS, TELEGRAM_MAX_MESSAGE_LENGTH, build_telegram_ack_reaction_request,
    random_telegram_ack_reaction, split_message_for_telegram,
};

const TELEGRAM_CONTINUATION_OVERHEAD: usize = 30;

#[test]
fn split_message_keeps_short_message_single_chunk() {
    assert_eq!(split_message_for_telegram("short"), vec!["short".to_string()]);
}

#[test]
fn split_message_keeps_exact_limit_message_single_chunk() {
    let message = "a".repeat(TELEGRAM_MAX_MESSAGE_LENGTH);

    assert_eq!(split_message_for_telegram(&message), vec![message]);
}

#[test]
fn split_message_prefers_newline_when_break_is_far_enough() {
    let chunk_limit = TELEGRAM_MAX_MESSAGE_LENGTH - TELEGRAM_CONTINUATION_OVERHEAD;
    let first_line = "a".repeat(chunk_limit - 10);
    let remainder = "b".repeat(100);
    let message = format!("{first_line}\n{remainder}");

    let chunks = split_message_for_telegram(&message);

    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].ends_with('\n'));
    assert_eq!(chunks[0], format!("{first_line}\n"));
    assert_eq!(chunks[1], remainder);
}

#[test]
fn split_message_falls_back_to_space_when_newline_is_too_early() {
    let early_line = "a".repeat(1000);
    let near_limit_word = "b".repeat(3000);
    let tail = "c".repeat(200);
    let message = format!("{early_line}\n{near_limit_word} {tail}");

    let chunks = split_message_for_telegram(&message);

    assert_eq!(chunks.len(), 2);
    assert!(!chunks[0].ends_with('\n'));
    assert!(chunks[0].ends_with(' '));
    assert_eq!(chunks.concat(), message);
}

#[test]
fn split_message_prefers_space_when_no_newline_exists() {
    let chunk_limit = TELEGRAM_MAX_MESSAGE_LENGTH - TELEGRAM_CONTINUATION_OVERHEAD;
    let first_word = "a".repeat(chunk_limit - 10);
    let tail = "b".repeat(100);
    let message = format!("{first_word} {tail}");

    let chunks = split_message_for_telegram(&message);

    assert_eq!(chunks, vec![format!("{first_word} "), tail]);
    assert_eq!(chunks.concat(), message);
}

#[test]
fn split_message_hard_splits_when_early_newline_has_no_later_space() {
    let early_line = "a".repeat(1000);
    let tail = "测".repeat(TELEGRAM_MAX_MESSAGE_LENGTH);
    let message = format!("{early_line}\n{tail}");

    let chunks = split_message_for_telegram(&message);

    assert_eq!(chunks.len(), 2);
    assert!(chunks.iter().all(|chunk| chunk.chars().count() <= TELEGRAM_MAX_MESSAGE_LENGTH));
    assert_eq!(chunks.concat(), message);
}

#[test]
// 仅在找不到换行或空格时硬切分，尽量保持自然语义。
fn split_message_hard_splits_when_no_natural_boundary_exists() {
    let message = "测".repeat(TELEGRAM_MAX_MESSAGE_LENGTH + 10);

    let chunks = split_message_for_telegram(&message);

    assert_eq!(chunks.len(), 2);
    assert!(chunks.iter().all(|chunk| chunk.chars().count() <= TELEGRAM_MAX_MESSAGE_LENGTH));
    assert_eq!(chunks.concat(), message);
}

#[test]
fn random_ack_reaction_is_selected_from_supported_list() {
    assert!(TELEGRAM_ACK_REACTIONS.contains(&random_telegram_ack_reaction()));
}

#[test]
fn ack_reaction_request_preserves_fields() {
    let request = build_telegram_ack_reaction_request("chat", 42, "👍");

    assert_eq!(request["chat_id"], "chat");
    assert_eq!(request["message_id"], 42);
    assert_eq!(request["reaction"][0]["type"], "emoji");
    assert_eq!(request["reaction"][0]["emoji"], "👍");
}
