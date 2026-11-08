//! 应用中心弹窗测试，覆盖书签新增编辑、激活优先级和提交条件。
//! 选择索引失效时仍需安全构建界面，且 Cookie 示例必须展示必要字段契约。

use crate::app::message::ViewMessage;
use crate::app::{App, Message};

fn new_app() -> App {
    App::new().0
}

#[test]
fn active_modal_returns_none_without_modal_state() {
    let app = new_app();
    assert!(super::active_modal(&app).is_none());
}

#[test]
fn active_modal_prefers_edit_modal_over_add_modal() {
    let mut app = new_app();
    app.show_web_links_menu = true;
    app.editing_web_bookmark = Some(2);

    let Some((_modal, close)) = super::active_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn active_modal_returns_add_bookmark_close_message() {
    let mut app = new_app();
    app.show_web_links_menu = true;

    let Some((_modal, close)) = super::active_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn can_add_web_bookmark_requires_non_blank_title() {
    assert!(!super::can_add_web_bookmark("", "https://example.com"));
    assert!(!super::can_add_web_bookmark(" \n\t", "https://example.com"));
}

#[test]
fn can_add_web_bookmark_requires_non_blank_url() {
    assert!(!super::can_add_web_bookmark("Example", ""));
    assert!(!super::can_add_web_bookmark("Example", " \n\t"));
}

#[test]
fn can_add_web_bookmark_accepts_trimmed_non_empty_fields() {
    assert!(super::can_add_web_bookmark("  Example  ", "  https://example.com  "));
}

#[test]
fn add_web_bookmark_modal_builds_for_incomplete_input() {
    let app = new_app();

    let Some((_modal, close)) = super::add_web_bookmark_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn add_web_bookmark_modal_builds_for_valid_input() {
    let mut app = new_app();
    app.web_bookmark_title_input = "Example".into();
    app.web_bookmark_url_input = "https://example.com".into();

    let Some((_modal, close)) = super::add_web_bookmark_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn edit_web_bookmark_modal_builds_for_selected_bookmark() {
    let mut app = new_app();
    app.editing_web_bookmark = Some(7);

    let Some((_modal, close)) = super::edit_web_bookmark_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn edit_web_bookmark_modal_uses_safe_index_fallback_without_selection() {
    let app = new_app();

    let Some((_modal, close)) = super::edit_web_bookmark_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn cookie_config_example_documents_required_cookie_fields() {
    let example = super::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE;

    assert!(example.starts_with('['));
    assert!(example.ends_with(']'));
    for field in ["\"name\"", "\"domain\"", "\"days\"", "\"url_filter\""] {
        assert!(example.contains(field));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_04_bookmarks_render_contextual_help() {
    use crate::app::components::system_settings_apps::preferences_tests::{
        HELP_LOCALES, verify_help,
    };

    const CHILD: &str = "SHIJU_HELP_ROLLOUT_04_BOOKMARK_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            br#"{"host":"[","servers":[]}"#,
        )
        .unwrap();
        let module = module_path!().split_once("::").unwrap().1;
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("{module}::help_rollout_04_bookmarks_render_contextual_help"),
            ])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path())
            .env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let mut app = new_app();
    app.show_web_links_menu = true;
    app.web_bookmark_title_input = "Fixture bookmark".into();
    app.web_bookmark_url_input = "https://example.invalid/fixture".into();
    for mode in &HELP_LOCALES {
        app.desktop_locale = mode.0;
        app.app_theme = mode.1.clone();
        app.editing_web_bookmark = None;
        let add = verify_help(
            super::super::view(&app),
            "bookmark-add-help",
            mode,
            &[("新增网址书签", "保存一个可在应用中心快速打开的网页入口。")],
            &["标题", "网址"],
            "添加",
        )
        .await;
        assert!(matches!(add.as_slice(), [Message::View(ViewMessage::WebBookmarkAddSave)]));

        app.editing_web_bookmark = Some(0);
        let edit = verify_help(
            super::super::view(&app),
            "bookmark-edit-help",
            mode,
            &[
                ("书签信息", "设置标题、网址以及独立窗口的默认尺寸。"),
                ("Cookie 配置", "支持按 JSON 为指定站点注入 Cookie。"),
            ],
            &["Cookie配置(JSON)", "插入例子"],
            "保存",
        )
        .await;
        assert!(matches!(edit.as_slice(), [Message::View(ViewMessage::WebBookmarkEditSave)]));
    }
}
