use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_11_feedback_switches_language() {
    use crate::app::message::git::GitMessage;
    let (mut app, _) = crate::app::App::new();
    let raw = "用户/%s/%n";
    let _ =
        crate::app::message::git::update(&mut app, GitMessage::StageAllFinished(Err(raw.into())));
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "暂存 Git 变更失败："),
        (DesktopLocale::TraditionalChinese, "暫存 Git 變更失敗："),
        (DesktopLocale::English, "Failed to stage Git changes: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}

#[test]
fn desktop_gettext_area_11_catalog_covers_display_paths() {
    let en = super::catalog::parse_catalog(include_str!("../../../../libs/desktop-i18n/src/locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("../../../../libs/desktop-i18n/src/locales/zh-TW.json")).unwrap();
    let literal = regex::Regex::new(r#"\.__\("([^"]+)"\)"#).unwrap();
    for source in [
        include_str!("../components/git_panel/mod.rs"),
        include_str!("../components/git_panel/left_panel.rs"),
        include_str!("../components/git_panel/header.rs"),
        include_str!("../components/git_panel/filter_options.rs"),
        include_str!("../components/git_panel/diff_view/context_menu.rs"),
        include_str!("../components/git_panel/diff_view/comment_editor.rs"),
        include_str!("../components/git_panel/diff_view/file_view/mod.rs"),
    ] {
        for capture in literal.captures_iter(source) {
            assert!(en.contains_key(&capture[1]), "missing English: {}", &capture[1]);
            assert!(tw.contains_key(&capture[1]), "missing Traditional Chinese: {}", &capture[1]);
        }
    }
}
