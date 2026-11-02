use super::settings_runtime::{Text, text};
use vw_config_types::ui::DesktopLocale;
#[test]
fn desktop_i18n_settings_runtime_switches_visible_text() {
    assert_eq!(text(DesktopLocale::SimplifiedChinese, Text::RuntimeKind), "运行时类型");
    assert_eq!(text(DesktopLocale::TraditionalChinese, Text::RuntimeKind), "執行環境類型");
    assert_eq!(text(DesktopLocale::English, Text::RuntimeKind), "Runtime type");
}
