//! 验证本地服务启动过渡页在各状态与主题下都能构建。

use super::startup_transition;
use crate::app::state::NativeStartupState;

#[test]
fn startup_transition_builds_starting_and_failed_views_for_light_and_dark_themes() {
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        let starting = NativeStartupState::Starting;
        let failed = NativeStartupState::Failed("Gateway 启动失败".into());

        let _: iced::Element<'_, crate::app::Message> =
            startup_transition::view(&starting, 3, &theme);
        let _: iced::Element<'_, crate::app::Message> =
            startup_transition::view(&failed, 3, &theme);
    }
}
