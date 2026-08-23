//! 应用辅助状态测试，验证最近项目、Web 书签、通知与提示消息的数据契约。
//! 克隆和序列化往返应完整保留标识、可选字段及嵌套 Cookie 配置。

use super::{
    CookieConfig, Notification, RecentProjectMeta, Toast, ToastKind, WebBookmark,
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
fn web_bookmark_round_trips_nested_cookie_configuration() {
    let bookmark = WebBookmark {
        title: "Console".to_string(),
        url: "https://example.test/app".to_string(),
        width: Some(1280),
        height: Some(720),
        cookie_configs: Some(vec![CookieConfig {
            name: "session".to_string(),
            domain: Some("example.test".to_string()),
            days: Some(30),
            url_filter: Some("/app".to_string()),
        }]),
    };

    let encoded = serde_json::to_value(&bookmark).expect("bookmark should serialize");
    let decoded: WebBookmark =
        serde_json::from_value(encoded).expect("bookmark should deserialize");

    assert_eq!(decoded.title, "Console");
    assert_eq!(decoded.url, "https://example.test/app");
    assert_eq!(decoded.width, Some(1280));
    assert_eq!(decoded.height, Some(720));
    let cookies = decoded.cookie_configs.expect("cookie configuration should remain present");
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "session");
    assert_eq!(cookies[0].domain.as_deref(), Some("example.test"));
    assert_eq!(cookies[0].days, Some(30));
    assert_eq!(cookies[0].url_filter.as_deref(), Some("/app"));
}

#[test]
fn web_bookmark_preserves_absent_optional_values() {
    let bookmark: WebBookmark = serde_json::from_value(serde_json::json!({
        "title": "Minimal",
        "url": "about:blank",
        "width": null,
        "height": null,
        "cookie_configs": null
    }))
    .expect("minimal bookmark should deserialize");

    assert!(bookmark.width.is_none());
    assert!(bookmark.height.is_none());
    assert!(bookmark.cookie_configs.is_none());
}

#[test]
fn notification_clone_preserves_identity_message_and_timestamp() {
    let notification = Notification {
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
        let toast = Toast { id, kind, message: format!("{kind:?}") };
        let cloned = toast.clone();

        assert_eq!(cloned.id, id);
        assert_eq!(cloned.kind, kind);
        assert_eq!(cloned.message, format!("{kind:?}"));
    }
}
