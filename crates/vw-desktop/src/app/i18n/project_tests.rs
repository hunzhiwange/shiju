use super::{I18n, input::Feedback, project::search_error};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_10_search_error_preserves_external_content() {
    let raw = "用户表达式 %s %n";
    let feedback = Feedback::new("正则表达式错误: %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "正则表达式错误: "),
        (DesktopLocale::TraditionalChinese, "規則運算式錯誤: "),
        (DesktopLocale::English, "Regular expression error: "),
    ] {
        let i18n = I18n::new(locale);
        assert_eq!(
            search_error(i18n, &feedback.fallback, Some(&feedback)),
            format!("{prefix}{raw}")
        );
        assert_eq!(search_error(i18n, raw, None), raw);
        assert_eq!(search_error(i18n, raw, Some(&feedback)), raw);
    }
}

#[test]
fn desktop_gettext_area_10_worktree_labels_keep_names_as_data() {
    let name = "主工作区/%s/%n";
    for (locale, prefix, create) in [
        (DesktopLocale::SimplifiedChinese, "独立工作区", "创建新的独立工作区"),
        (DesktopLocale::TraditionalChinese, "獨立工作區", "建立新的獨立工作區"),
        (DesktopLocale::English, "Independent workspace", "Create a new independent workspace"),
    ] {
        let i18n = I18n::new(locale);
        assert_eq!(
            super::project::worktree_label(i18n, "/work", name, false),
            format!("{prefix} ({name})")
        );
        assert_eq!(super::project::worktree_label(i18n, "__create_worktree__", name, true), create);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_10_attachment_errors_switch_after_collection() {
    let (mut app, _) = crate::app::App::new();
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("用户文件-%s-%n.txt");
    crate::app::message::project::helpers::append_local_attachments(
        &mut app,
        vec![
            directory.path().to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
        ],
    );
    let notification = app.notifications.last().unwrap();
    assert_eq!(notification.message.lines().count(), 2);
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "不支持文件夹："),
        (DesktopLocale::TraditionalChinese, "不支援資料夾："),
        (DesktopLocale::English, "Folders are not supported: "),
    ] {
        let rendered = notification.render(I18n::new(locale));
        assert!(rendered.starts_with(expected));
        assert!(rendered.contains(missing.to_str().unwrap()));
        assert_eq!(rendered.lines().count(), 2);
    }
}
