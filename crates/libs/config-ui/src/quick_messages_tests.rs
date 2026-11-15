use crate::AppSystemSettingsConfig;
use super::{
    QuickMessageConfig, QuickMessageGroup, QuickMessageKind,
    append_missing_builtin_general_quick_messages, append_missing_builtin_git_quick_messages,
    upgrade_legacy_quick_message_actions,
};

// 迁移只补缺失的内置动作并升级旧类型，绝不能覆盖或复制自定义项。
#[test]
fn nonempty_legacy_quick_messages_gain_missing_builtin_git_actions() {
    let defaults = AppSystemSettingsConfig::default();
    let mut legacy = defaults.quick_messages[..5].to_vec();
    legacy[2].kind = QuickMessageKind::Message;
    legacy[3].kind = QuickMessageKind::CreateNewSession;
    legacy[4].kind = QuickMessageKind::Message;

    upgrade_legacy_quick_message_actions(&mut legacy);
    append_missing_builtin_general_quick_messages(&mut legacy);
    append_missing_builtin_git_quick_messages(&mut legacy);

    assert_eq!(legacy, defaults.quick_messages);
    append_missing_builtin_general_quick_messages(&mut legacy);
    append_missing_builtin_git_quick_messages(&mut legacy);
    assert_eq!(legacy, defaults.quick_messages);

    let mut empty = Vec::new();
    append_missing_builtin_general_quick_messages(&mut empty);
    append_missing_builtin_git_quick_messages(&mut empty);
    assert!(empty.is_empty());
}

#[test]
fn existing_custom_quick_messages_gain_new_general_builtins_without_duplicates() {
    let mut messages = vec![
        QuickMessageConfig {
            text: "自定义消息".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::Message,
        },
        QuickMessageConfig {
            text: " 修复单元测试 ".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::Message,
        },
        QuickMessageConfig {
            text: "刷新 Git 变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::RefreshGitChanges,
        },
    ];

    upgrade_legacy_quick_message_actions(&mut messages);
    append_missing_builtin_general_quick_messages(&mut messages);

    assert_eq!(messages.iter().filter(|item| item.text.trim() == "修复单元测试").count(), 1);
    assert_eq!(messages[1].kind, QuickMessageKind::FillInput);
    assert!(messages.iter().any(|item| item.text == "不要兼容旧的代码"));
    let first_git = messages
        .iter()
        .position(|item| item.group == QuickMessageGroup::Git)
        .expect("custom Git action should remain");
    assert!(messages[..first_git].iter().all(|item| item.group == QuickMessageGroup::General));
    assert_eq!(messages.len(), 29);
}

#[test]
fn legacy_builtin_kinds_are_upgraded_without_changing_custom_items() {
    let mut messages = vec![
        QuickMessageConfig {
            text: "摘要并创建新会话".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::CreateNewSession,
        },
        QuickMessageConfig {
            text: "直接新建".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::CreateNewSession,
        },
        QuickMessageConfig {
            text: "修复单元测试".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::Message,
        },
        QuickMessageConfig {
            text: "自定义消息".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::Message,
        },
    ];

    upgrade_legacy_quick_message_actions(&mut messages);

    assert_eq!(messages[0].kind, QuickMessageKind::SummarizeAndCreateNewSession);
    assert_eq!(messages[0].text, "压缩当前会话");
    assert_eq!(messages[1].kind, QuickMessageKind::CreateNewSession);
    assert_eq!(messages[2].kind, QuickMessageKind::FillInput);
    assert_eq!(messages[3].kind, QuickMessageKind::Message);
}
