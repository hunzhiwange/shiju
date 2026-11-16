use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_11_feedback_switches_language() {
    use crate::app::message::git::GitMessage;
    let (mut app, _) = crate::app::App::new();
    let raw = "用户/%s/%n";
    let _ =
        crate::app::message::git::update(&mut app, GitMessage::StageAllFinished(Err(raw.into())));
    let message =
        app.toast_state.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "暂存 Git 变更失败："),
        (DesktopLocale::TraditionalChinese, "暫存 Git 變更失敗："),
        (DesktopLocale::English, "Failed to stage Git changes: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}
