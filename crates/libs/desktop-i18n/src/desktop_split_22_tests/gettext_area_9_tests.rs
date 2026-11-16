use crate::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_9_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "没有匹配的文件"),
        (DesktopLocale::TraditionalChinese, "沒有符合的檔案"),
        (DesktopLocale::English, "No matching files"),
    ] {
        assert_eq!(I18n::new(locale).__("没有匹配的文件"), expected);
    }
    assert!(
        include_str!("../../../../vw-desktop/src/app/components/input_panel/file_search.rs")
            .contains(".__(\"没有匹配的文件\")")
    );
}
