use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments() {
    use crate::app::message::project::ProjectMessage;
    let (mut app, _) = crate::app::App::new();
    let raw = "用户目录/%n/%s/原始错误";
    app.file_tree_interaction.file_tree_clipboard = None;
    let _ = app.show_localized_toast(
        crate::app::state::ToastKind::Success,
        "已重命名为 %s",
        vec![raw.into()],
    );
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "已重命名为 "),
        (DesktopLocale::TraditionalChinese, "已重新命名為 "),
        (DesktopLocale::English, "Renamed to "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
    app.file_tree_interaction.file_tree_rename_path = Some("/project/file".into());
    app.file_tree_interaction.file_tree_rename_value = "../invalid".into();
    let _ = crate::app::message::project::update(&mut app, ProjectMessage::FileTreeRenameSave);
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "文件名不合法"),
        (DesktopLocale::TraditionalChinese, "檔案名稱無效"),
        (DesktopLocale::English, "Invalid file name"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}
