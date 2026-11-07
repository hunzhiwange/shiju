//! 验证账户详情弹窗的资料状态、编辑流程和敏感字段显示边界。
//!
//! 账户夹具区分已登录、加载、错误和编辑草稿，避免敏感值进入普通展示路径。

use super::*;

#[test]
fn status_labels_cover_gateway_values() {
    assert_eq!(status_label("enabled"), "启用");
    assert_eq!(status_label("ACTIVE"), "启用");
    assert_eq!(status_label("disabled"), "停用");
    assert_eq!(status_label("custom"), "custom");
}

#[test]
fn empty_and_multiple_values_have_readable_labels() {
    assert_eq!(values_label(&[]), "—");
    assert_eq!(values_label(&["AI创新组".to_string(), "平台组".to_string()]), "AI创新组、平台组");
}

#[test]
fn profile_submit_requires_a_valid_editing_draft() {
    let mut draft =
        crate::app::account::AccountProfileDraft::new("示例用户", "fixture-user").unwrap();
    assert!(profile_editor_can_submit(&AccountProfileEditorState::Editing(draft.clone())));
    assert!(!profile_editor_can_submit(&AccountProfileEditorState::Saving(draft.clone())));
    draft.set_name(" ".into());
    assert!(!profile_editor_can_submit(&AccountProfileEditorState::Editing(draft)));
    assert!(!profile_editor_can_submit(&AccountProfileEditorState::Loading));
}

#[test]
fn password_submit_allows_validation_and_retry_but_not_saving() {
    assert!(password_editor_can_submit(&AccountPasswordEditorState::Editing(Default::default())));
    assert!(password_editor_can_submit(&AccountPasswordEditorState::Failed {
        draft: Default::default(),
        message: "Fixture unavailable".into(),
    }));
    assert!(!password_editor_can_submit(&AccountPasswordEditorState::Saving(Default::default())));
    assert!(!password_editor_can_submit(&AccountPasswordEditorState::Closed));
}
