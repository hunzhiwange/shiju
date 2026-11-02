use super::settings_models::{Text, enabled_count, model_metadata, text};
use vw_config_types::ui::DesktopLocale;
#[test]
fn desktop_i18n_settings_models_switches_visible_text() {
    for (locale, title, count) in [
        (DesktopLocale::SimplifiedChinese, "模型", "2 / 5 已启用"),
        (DesktopLocale::TraditionalChinese, "模型", "2 / 5 已啟用"),
        (DesktopLocale::English, "Models", "2 / 5 enabled"),
    ] {
        assert_eq!(text(locale, Text::Models), title);
        assert_eq!(enabled_count(locale, 2, 5), count);
        let raw = "custom:原始/🚀";
        assert!(model_metadata(locale, true, false, raw).ends_with(raw));
    }
}
