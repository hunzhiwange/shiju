use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_12_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "统一管理调度、工作池与批量任务动作。"),
        (DesktopLocale::TraditionalChinese, "統一管理排程、工作池與批次任務動作。"),
        (DesktopLocale::English, "Manage scheduling, work pools, and bulk task actions."),
    ] {
        assert_eq!(I18n::new(locale).__("统一管理调度、工作池与批量任务动作。"), expected);
    }
}
