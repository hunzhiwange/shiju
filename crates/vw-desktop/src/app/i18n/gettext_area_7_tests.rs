use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_7_general_search_keeps_chinese_and_stable_keywords() {
    let (mut app, _) = crate::app::App::new();
    app.general_settings.desktop_locale = DesktopLocale::English;
    let entries = crate::app::components::system_settings_search::catalog(&app);
    let entry = entries.iter().find(|entry| entry.label == "General settings").unwrap();
    assert!(entry.matches("常规"));
    assert!(entry.matches("general"));
    assert!(entry.matches("General settings"));
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_7_history_rebuilds_on_locale_selection_without_translating_output() {
    use crate::app::message::settings::{GeneralSettingsMessage, SettingsMessage};
    let (mut app, _) = crate::app::App::new();
    let raw = "原始输出/%s/%n";
    app.cron_settings.runs_modal = vec![gateway_cron::CronRunDto {
        id: 1,
        job_id: "job".into(),
        started_at: "start".into(),
        finished_at: "end".into(),
        status: "raw-status".into(),
        output: Some(raw.into()),
        duration_ms: None,
    }];
    for (locale, label) in [
        (DesktopLocale::English, "Status: raw-status"),
        (DesktopLocale::TraditionalChinese, "狀態：raw-status"),
        (DesktopLocale::SimplifiedChinese, "状态: raw-status"),
    ] {
        let _ = crate::app::message::settings::update(
            &mut app,
            SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(locale)),
        );
        let text = app.cron_settings.runs_modal_editor.text();
        assert!(text.contains(label), "{text}");
        assert!(text.ends_with(raw));
        assert_eq!(app.cron_settings.runs_modal[0].output.as_deref(), Some(raw));
    }
}
