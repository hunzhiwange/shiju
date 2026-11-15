use super::{agent_sessions_badge_style, agent_sessions_popover_style, agent_sessions_row_style, agent_sessions_stop_style};
use iced::Theme;
#[test]
fn agent_sessions_popover_styles_support_light_and_dark_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let badge = agent_sessions_badge_style(&theme, iced::widget::button::Status::Hovered);
        let popover = agent_sessions_popover_style(&theme);
        let row = agent_sessions_row_style(&theme);
        let stop = agent_sessions_stop_style(&theme, iced::widget::button::Status::Hovered);

        assert!(badge.background.is_some());
        assert!(badge.text_color.a > 0.0);
        assert!(popover.background.is_some());
        assert!(popover.text_color.is_some());
        assert!(row.background.is_some());
        assert!(row.text_color.is_some_and(|color| color.a > 0.0));
        assert!(stop.background.is_some());
        assert!(stop.text_color.a > 0.0);
    }
}
