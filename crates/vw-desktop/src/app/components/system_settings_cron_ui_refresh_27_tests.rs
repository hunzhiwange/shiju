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
            let i18n = I18n::new(app.desktop_locale);
            let labels: Vec<_> = CRON_TABS.iter().map(|tab| i18n.__(tab.label)).collect();
            assert!(matches!(
                scene.check(app, view(app), &labels, 3),
                SettingsMessage::CronTabSelected(CronSettingsTab::Scheduler)
            ));
        },
    )
    .await;
}
