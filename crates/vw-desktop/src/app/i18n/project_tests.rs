use super::{I18n, input::Feedback, project::search_error};
use config_ui::DesktopLocale;

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
    let notification = app.notification_state.notifications.last().unwrap();
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
