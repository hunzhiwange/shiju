//! 验证本地服务启动过渡页在各主题下都能构建。

use super::startup_transition;
use crate::app::state::NativeStartupState;

#[test]
fn startup_transition_builds_starting_view_for_light_and_dark_themes() {
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        let starting = NativeStartupState::Starting;

        let _: iced::Element<'_, crate::app::Message> =
            startup_transition::view(&starting, 3, &theme);
    }
}
