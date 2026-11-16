use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_16_locale_event_updates_existing_and_new_editors() {
    use crate::app::message::{PreviewMessage, preview, settings};
    let (mut app, _) = crate::app::App::new();
    let editor_setting = app.preview_state.current_language;
    let _ = preview::update(&mut app, PreviewMessage::Open("/tmp/原始%s.zip".into()));
    for locale in [
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
        DesktopLocale::SimplifiedChinese,
    ] {
        let _ = settings::update(
            &mut app,
            settings::SettingsMessage::General(settings::GeneralSettingsMessage::LocaleSelected(
                locale,
            )),
        );
        let expected = super::editor::language(locale);
        assert_eq!(app.preview_state.current_language, editor_setting);
        assert_eq!(app.git.git_copy_modal_code_editor.language(), expected);
        assert!(
            app.navigation_tabs
                .preview_tabs
                .iter()
                .all(|tab| tab.editor.inner.language() == expected)
        );
        let path = format!("/tmp/{locale:?}.zip");
        let _ = preview::update(&mut app, PreviewMessage::Open(path.clone()));
        let tab = app.navigation_tabs.preview_tabs.iter().find(|tab| tab.path == path).unwrap();
        assert_eq!(tab.editor.inner.language(), expected);
    }
}
