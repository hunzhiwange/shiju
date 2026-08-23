use super::*;

#[test]
fn is_user_allowed_supports_exact_match_and_wildcard() {
    assert!(is_user_allowed(&["alice".to_string()], "alice"));
    assert!(!is_user_allowed(&["alice".to_string()], "bob"));
    assert!(is_user_allowed(&["*".to_string()], "anyone"));
}

#[test]
fn is_group_sender_trigger_enabled_rejects_blank_sender() {
    assert!(!is_group_sender_trigger_enabled(&["*".to_string()], "  "));
    assert!(is_group_sender_trigger_enabled(&["sender-1".to_string()], " sender-1 "));
}

#[test]
fn is_user_allowed_empty_list_denies_and_does_not_trim_user_ids() {
    assert!(!is_user_allowed(&[], "alice"));
    assert!(!is_user_allowed(&["alice".to_string()], " alice "));
}

#[test]
fn group_sender_trigger_requires_exact_trimmed_match_or_wildcard() {
    assert!(is_group_sender_trigger_enabled(&["*".to_string()], "sender-2"));
    assert!(is_group_sender_trigger_enabled(&["sender-2".to_string()], "\tsender-2\n"));
    assert!(!is_group_sender_trigger_enabled(&["sender-2".to_string()], "sender-3"));
}
// 本文件验证 Discord 用户许可与群聊发送者触发规则。
