use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_16_preview_loading_and_failure_switch_at_display_time() {
    use crate::preview::message::PreviewMessage;
    let mut state = super::fixture::Fixture::default();
    let path = "/tmp/原始%s.txt";
    state.tab(PreviewMessage::Open(path.into()));
    assert_eq!(
        state.preview_messages[path].render(I18n::new(DesktopLocale::English)).unwrap(),
        "Loading…"
    );
    let raw = "服务器/%s/%n/原始";
    state.tab(PreviewMessage::OpenFailed { path: path.into(), error: raw.into() });
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "读取失败："),
        (DesktopLocale::TraditionalChinese, "讀取失敗："),
        (DesktopLocale::English, "Read failed: "),
    ] {
        assert_eq!(
            state.preview_messages[path].render(I18n::new(locale)).unwrap(),
            format!("{prefix}{raw}\n")
        );
    }
}
