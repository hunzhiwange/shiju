// 本文件验证 Discord 群聊内容的提及移除、发送者规则与空消息过滤。

use super::*;

#[test]
fn normalize_group_reply_allowed_sender_ids_trims_sorts_and_dedups() {
    let ids = vec![
        "  bob  ".to_string(),
        "alice".to_string(),
        "".to_string(),
        "bob".to_string(),
        "   ".to_string(),
    ];

    assert_eq!(normalize_group_reply_allowed_sender_ids(ids), vec!["alice", "bob"]);
}

#[test]
fn normalize_incoming_content_requires_and_removes_bot_mention() {
    assert_eq!(normalize_incoming_content("hello", true, "42"), None);
    assert_eq!(
        normalize_incoming_content("<@42>  hello <@!42>", true, "42"),
        Some("hello".to_string())
    );
    assert_eq!(normalize_incoming_content("  hello  ", false, "42"), Some("hello".to_string()));
}

#[test]
fn mention_helpers_cover_plain_nick_and_absent_mentions() {
    let tags = mention_tags("42");

    assert_eq!(tags, ["<@42>".to_string(), "<@!42>".to_string()]);
    assert!(contains_bot_mention("hi <@42>", "42"));
    assert!(contains_bot_mention("hi <@!42>", "42"));
    assert!(!contains_bot_mention("hi <@24>", "42"));
    assert!(!contains_bot_mention("hi <@420>", "42"));
    assert!(!contains_bot_mention("hi <@!420>", "42"));
}

#[test]
fn normalize_incoming_content_rejects_empty_or_mention_only_messages() {
    assert_eq!(normalize_incoming_content("", false, "42"), None);
    assert_eq!(normalize_incoming_content("   ", false, "42"), None);
    assert_eq!(normalize_incoming_content("<@42> <@!42>", true, "42"), None);
}

#[test]
// 删除单词间的提及时保留分隔，避免拼接出不同语义的新词。
fn normalize_incoming_content_keeps_words_separated_after_removing_mentions() {
    assert_eq!(
        normalize_incoming_content("run<@42>status and<@!42>report", true, "42"),
        Some("run status and report".to_string())
    );
}
