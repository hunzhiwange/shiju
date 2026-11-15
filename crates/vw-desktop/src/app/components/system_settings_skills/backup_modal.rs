use crate::app::{App, Message};
fn message(event: desktop_settings_skills::backup_modal::Event) -> Message {
    use crate::app::message::settings::skills::backup_modal::{Event, message};
    use desktop_settings_skills::backup_modal::Event as Ui;
    message(match event {
        Ui::Close => Event::Close,
        Ui::Refresh => Event::Refresh,
        Ui::Select(v0) => Event::Select(v0),
        Ui::Target(v0, v1) => Event::Target(v0, v1),
        Ui::Delete => Event::Delete,
        Ui::Restore => Event::Restore,
    })
}
pub(super) fn view<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    let s = &app.skills_settings.backups;
    desktop_settings_skills::backup_modal::view(
        desktop_settings_skills::backup_modal::ViewState {
            open: s.open,
            busy: s.busy,
            backups: &s.backups,
            selected: &s.selected,
            available: &s.available,
            targets: &s.targets,
            version: s.version,
            error: &s.error,
        },
        base,
        message,
    )
}
