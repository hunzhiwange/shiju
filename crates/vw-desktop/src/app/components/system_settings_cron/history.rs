use super::*;
pub(crate) use desktop_settings_cron::system_settings_cron::history::run_key;
pub(super) fn view(app: &App) -> Element<'_, Message> { domain::history::view(&data(app)).map(map) }
#[cfg(test)] fn with_details<'a>(app: &'a App, base: Element<'a,Message>) -> Element<'a,Message> { domain::history::with_details(&data(app),base,map) }
#[cfg(test)] #[path="history_tests.rs"] mod tests;
