use super::tasks::SplitOption;
use crate::app::task::TaskSplitMode;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_12_split_option_keeps_value() {
    for (locale, expected) in [
        (DesktopLocale::English, "Follow system"),
        (DesktopLocale::TraditionalChinese, "遵循系統"),
        (DesktopLocale::SimplifiedChinese, "遵循系统"),
    ] {
        let option = SplitOption(TaskSplitMode::FollowSystem, locale);
        assert_eq!(option.0, TaskSplitMode::FollowSystem);
        assert_eq!(option.to_string(), expected);
    }
}
