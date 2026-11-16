use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments() {
    use crate::app::message::project::ProjectMessage;
    let (mut app, _) = crate::app::App::new();
    app.file_tree_interaction.file_tree_rename_path = Some("/project/file".into());
    app.file_tree_interaction.file_tree_rename_value = "../invalid".into();
    let _ = crate::app::message::project::update(&mut app, ProjectMessage::FileTreeRenameSave);
    let message =
        app.toast_state.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "文件名不合法"),
        (DesktopLocale::TraditionalChinese, "檔案名稱無效"),
        (DesktopLocale::English, "Invalid file name"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}
