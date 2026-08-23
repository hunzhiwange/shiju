use super::*;

#[test]
fn split_message_for_discord_keeps_short_messages_intact() {
    assert_eq!(split_message_for_discord("hello"), vec!["hello".to_string()]);
}

#[test]
fn split_message_for_discord_respects_unicode_character_limit() {
    let message = "界".repeat(DISCORD_MAX_MESSAGE_LENGTH + 1);
    let chunks = split_message_for_discord(&message);

    assert_eq!(chunks.len(), 2);
    assert!(chunks.iter().all(|chunk| chunk.chars().count() <= DISCORD_MAX_MESSAGE_LENGTH));
    assert_eq!(chunks.concat(), message);
}

#[test]
fn split_message_for_discord_prefers_newline_then_space_then_hard_split() {
    let newline = format!("{}\n{}", "a".repeat(1500), "b".repeat(700));
    let chunks = split_message_for_discord(&newline);
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].ends_with('\n'));

    let spaced = format!("{} {}", "a".repeat(1500), "b".repeat(700));
    let chunks = split_message_for_discord(&spaced);
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].ends_with(' '));

    let solid = "x".repeat(DISCORD_MAX_MESSAGE_LENGTH * 2 + 3);
    let chunks = split_message_for_discord(&solid);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[2].len(), 3);
}

#[test]
fn split_message_for_discord_uses_space_when_newline_is_too_early() {
    let message = format!("{}\n{} {}", "a".repeat(100), "b".repeat(1399), "c".repeat(700));
    let chunks = split_message_for_discord(&message);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chars().count(), 1501);
    assert!(chunks[0].ends_with(' '));
    assert_eq!(chunks.concat(), message);
}

#[test]
fn split_message_for_discord_hard_splits_when_early_newline_has_no_space_fallback() {
    let message = format!("{}\n{}", "a".repeat(100), "b".repeat(2100));
    let chunks = split_message_for_discord(&message);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chars().count(), DISCORD_MAX_MESSAGE_LENGTH);
    assert_eq!(chunks[1].chars().count(), 201);
    assert_eq!(chunks.concat(), message);
}
// 本文件验证 Discord 消息按字符上限和自然分隔符安全拆分。
