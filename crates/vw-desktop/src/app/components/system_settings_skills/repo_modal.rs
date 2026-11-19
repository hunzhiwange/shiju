use crate::app::{App, Message};
pub(super) fn view<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    let market = &app.skills_settings.market;
    let repos = &app.skills_settings.repositories;
    let report = if market.context == repos.context
        && market.project == repos.project
        && market.host == repos.host
    {
        match &market.results {
            Some(desktop_settings_skills::market_view::Results::Discovery(report)) => Some(report),
            _ => None,
        }
    } else {
        None
    };
    desktop_settings_skills::repo_modal::view_with_discovery(
        repos,
        base,
        crate::app::message::settings::skills::repo_modal::message,
        report,
    )
}
