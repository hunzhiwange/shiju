use crate::app::{App, Message};
fn message(event: desktop_settings_skills::bindings_view::Event) -> Message {
    use crate::app::message::settings::skills::bindings_view::{Event, message};
    use desktop_settings_skills::bindings_view::Event as Ui;
    message(match event {
        Ui::Close => Event::Close,
        Ui::Refresh => Event::Refresh,
        Ui::Select(v0) => Event::Select(v0),
        Ui::Target(v0, v1) => Event::Target(v0, v1),
        Ui::Save => Event::Save,
        Ui::Bind(v0) => Event::Bind(v0),
        Ui::Resync => Event::Resync,
    })
}
pub(super) fn view<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    let s = &app.skills_settings.bindings;
    let applications = app
        .apps_catalog
        .rows
        .iter()
        .map(|r| desktop_settings_skills::bindings_view::BindingApplication {
            entry: &r.entry,
            observation: r.observation.as_ref().ok(),
        })
        .collect::<Vec<_>>();
    desktop_settings_skills::bindings_view::view(
        desktop_settings_skills::bindings_view::ViewState {
            open: s.open,
            busy: s.busy,
            inventory: &s.inventory,
            snapshot: &s.snapshot,
            selected: &s.selected,
            targets: &s.targets,
            results: &s.results,
            error: &s.error,
        },
        base,
        message,
        &applications,
        &app.apps_catalog.host,
    )
}
