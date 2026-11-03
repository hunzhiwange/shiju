use super::{I18n, modals};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_15_modal_formatter_keeps_raw_arguments() {
    let raw = "中文/%s/%n";
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, format!("当前版本 {raw}")),
        (DesktopLocale::TraditionalChinese, format!("目前版本 {raw}")),
        (DesktopLocale::English, format!("Current version {raw}")),
    ] {
        assert_eq!(modals::render(I18n::new(locale), "当前版本 %s", &[raw.into()]), expected);
    }
}
