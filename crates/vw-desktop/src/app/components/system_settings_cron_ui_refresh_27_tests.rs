use super::*;
use crate::app::i18n::I18n;
fn fixture() -> App {
    App::new().0
}
#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_cron() {
    use super::super::system_settings_common::tab_appearance_tests;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_cron"),
        fixture,
        |scene, app| {
            app.cron_settings.active_tab = CronSettingsTab::Jobs;
            let i18n = I18n::new(app.general_settings.desktop_locale);
            let labels: Vec<_> = CRON_TABS.iter().map(|tab| i18n.__(tab.label)).collect();
            assert!(matches!(
                scene.check(app, view(app), &labels, 3),
                SettingsMessage::CronTabSelected(CronSettingsTab::Scheduler)
            ));
        },
    )
    .await;
}

#[path = "../../../../libs/desktop-settings-shell/src/cron_render_test_support.rs"]
mod rendering;
use iced::{Point, Size, mouse};
use rendering::{Geometry, assert_fits, capture, click, frame, geometry, renderer, scene, wheel};
impl rendering::TestMessage for Message {
    fn is_none(&self) -> bool {
        matches!(self, Message::None)
    }
}
fn isolated(test: &str) -> bool {
    rendering::isolated(&format!("{}::{test}", module_path!()))
}
#[path = "../../../../libs/desktop-settings-cron/src/job_test_fixture.rs"]
mod job_fixture;
#[path = "system_settings_cron/list_tests.rs"]
mod list_tests;

fn job_row<'a>(app: &'a App, job: &'a gateway_cron::CronJobDto) -> Element<'a, Message> {
    domain::list::job_row(&data(app), job).map(map)
}
