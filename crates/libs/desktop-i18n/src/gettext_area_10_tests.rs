use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_10_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "搜索文件 / 项目 / 历史会话"),
        (DesktopLocale::TraditionalChinese, "搜尋檔案 / 專案 / 歷史對話"),
        (DesktopLocale::English, "Search files / projects / conversation history"),
    ] {
        assert_eq!(I18n::new(locale).__("搜索文件 / 项目 / 历史会话"), expected);
    }
}
