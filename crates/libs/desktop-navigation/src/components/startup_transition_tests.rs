//! 验证本地服务启动过渡页在各主题下都能构建。

use super::startup_transition;
use super::startup_transition::NativeStartupState;

#[test]
fn startup_transition_builds_starting_view_for_light_and_dark_themes() {
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        let starting = NativeStartupState::Starting;

        let _: iced::Element<'_, ()> = startup_transition::view(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::English),
            &starting,
            3,
            &theme,
        );
    }
}
