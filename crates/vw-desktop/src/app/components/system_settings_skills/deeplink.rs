use crate::app::{App, Message};
fn message(event: desktop_settings_skills::deeplink::Event) -> Message {
    use crate::app::message::settings::skills::repo_modal::{Event, message};
    use desktop_settings_skills::deeplink::Event as Ui;
    message(match event {
        Ui::SourceLink(v0) => Event::SourceLink(v0),
        Ui::PreviewSource => Event::PreviewSource,
        Ui::CancelSource => Event::CancelSource,
        Ui::ConfirmSource => Event::ConfirmSource,
    })
}
pub(super) fn view<'a>(app: &'a App) -> iced::Element<'a, Message> {
    let s = &app.skills_settings.repositories;
    desktop_settings_skills::deeplink::view(
        desktop_settings_skills::deeplink::ViewState {
            source_link: &s.source_link,
            busy: s.busy,
            source_preview: &s.source_preview,
            snapshot: &s.snapshot,
        },
        message,
    )
}
