use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_16_preview_metadata_preserves_filename() {
    let message = crate::preview::tab::preview_description("/tmp/原始%s.zip").unwrap();
    for (locale, kind) in [
        (DesktopLocale::SimplifiedChinese, "压缩包"),
        (DesktopLocale::TraditionalChinese, "壓縮檔"),
        (DesktopLocale::English, "Archive"),
    ] {
        let rendered = message.render(I18n::new(locale)).unwrap();
        assert!(rendered.contains("原始%s.zip"));
        assert!(rendered.contains(kind));
    }
}
