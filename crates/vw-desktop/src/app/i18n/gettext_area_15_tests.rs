use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_15_failed_skip_preserves_error_and_switches_language() {
    use crate::app::message::{ViewMessage, view};
    let (mut app, _) = crate::app::App::new();
    let raw = "服务错误/%s/%n/中文";
    let _ = view::update(
        &mut app,
        ViewMessage::SkipAppUpdateSaved(
            crate::app::desktop_update::SkippedRelease {
                source: desktop_release::UpdateSource::Shiju,
                version: "99.0.0".into(),
            },
            Err(raw.into()),
        ),
    );
    let toast = app.toast_state.active_toast.as_ref().expect("failure feedback");
    let message = toast.localized_message.as_ref().expect("semantic feedback");
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "跳过版本未保存："),
        (DesktopLocale::TraditionalChinese, "略過版本未儲存："),
        (DesktopLocale::English, "Skipped version was not saved: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}
