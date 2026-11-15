//! 应用状态重导出的组合测试，验证最近项目、通知与提示消息的数据契约。
//! 书签序列化测试随类型归属 desktop-apps-ui。

use super::{
    Notification, RecentProjectMeta, Toast, ToastKind,
    default_recent_project_session_auto_refresh,
    default_recent_project_session_refresh_interval_seconds,
};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("app_aux_tests"));
}

#[test]
fn recent_project_defaults_preserve_legacy_metadata_behavior() {
    let meta: RecentProjectMeta = serde_json::from_value(serde_json::json!({
        "path": "/workspace/demo",
        "name": "Demo"
    }))
    .expect("legacy recent project metadata should deserialize");

    assert_eq!(meta.path, "/workspace/demo");
    assert_eq!(meta.name, "Demo");
    assert!(meta.task_board_settings.is_none());
    assert_eq!(meta.session_auto_refresh, default_recent_project_session_auto_refresh());
    assert_eq!(
        meta.session_refresh_interval_seconds,
        default_recent_project_session_refresh_interval_seconds()
    );
    assert!(meta.icon.is_none());
    assert!(meta.icon_color.is_none());
    assert!(meta.worktree_start_command.is_none());
}

#[test]
fn recent_project_explicit_session_and_visual_metadata_round_trips() {
    let value = serde_json::json!({
        "path": "/workspace/explicit",
        "name": "Explicit",
        "task_board_settings": null,
        "session_auto_refresh": false,
        "session_refresh_interval_seconds": 0,
        "icon": "folder",
        "icon_color": "#123456",
        "worktree_start_command": "just dev"
    });

    let meta: RecentProjectMeta =
        serde_json::from_value(value.clone()).expect("explicit metadata should deserialize");
    assert!(!meta.session_auto_refresh);
    assert_eq!(meta.session_refresh_interval_seconds, 0);
    assert_eq!(meta.icon.as_deref(), Some("folder"));
    assert_eq!(meta.icon_color.as_deref(), Some("#123456"));
    assert_eq!(meta.worktree_start_command.as_deref(), Some("just dev"));
    assert_eq!(serde_json::to_value(meta).expect("explicit metadata should serialize"), value);
}

#[test]
fn notification_clone_preserves_identity_message_and_timestamp() {
    let notification = Notification {
        localized_message: None,
        id: 41,
        message: "saved".to_string(),
        created_at: web_time::SystemTime::UNIX_EPOCH,
    };

    let cloned = notification.clone();

    assert_eq!(cloned.id, 41);
    assert_eq!(cloned.message, "saved");
    assert_eq!(cloned.created_at, web_time::SystemTime::UNIX_EPOCH);
}

#[test]
fn toast_clone_preserves_every_kind_and_payload() {
    for (id, kind) in [ToastKind::Success, ToastKind::Info, ToastKind::Warning, ToastKind::Error]
        .into_iter()
        .enumerate()
    {
        let toast = Toast { localized_message: None, id, kind, message: format!("{kind:?}") };
        let cloned = toast.clone();

        assert_eq!(cloned.id, id);
        assert_eq!(cloned.kind, kind);
        assert_eq!(cloned.message, format!("{kind:?}"));
    }
}
