use crate::app::{App, Message};
fn message(event: desktop_settings_skills::claude_view::Event) -> Message {
    use crate::app::message::settings::skills::claude_view::{Event, message};
    use desktop_settings_skills::claude_view::Event as Ui;
    message(match event {
        Ui::Close => Event::Close,
        Ui::Refresh => Event::Refresh,
        Ui::Marker(v0) => Event::Marker(v0),
        Ui::Onboarding(v0) => Event::Onboarding(v0),
    })
}
pub(super) fn view<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    let s = &app.skills_settings.claude_config;
    desktop_settings_skills::claude_view::view(
        desktop_settings_skills::claude_view::ViewState {
            open: s.open,
            busy: s.busy,
            status: &s.status,
            notice: &s.notice,
            error: &s.error,
        },
        base,
        message,
    )
}
