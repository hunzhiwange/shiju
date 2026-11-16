use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_16_lsp_state_switches_without_translating_server_content() {
    let (mut app, _) = crate::app::App::new();
    let raw = "服务/%s/%n";
    super::editor::set_lsp_status(&mut app, "LSP 失败: %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "LSP 失败: "),
        (DesktopLocale::TraditionalChinese, "LSP 失敗: "),
        (DesktopLocale::English, "LSP failed: "),
    ] {
        app.general_settings.desktop_locale = locale;
        super::editor::refresh(&mut app);
        assert_eq!(super::editor::lsp_status(&app).unwrap(), format!("{prefix}{raw}"));
    }
    app.preview_state.lsp_status = Some(raw.into());
    assert_eq!(super::editor::lsp_status(&app).as_deref(), Some(raw));
}

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
        assert!(app.navigation_tabs.preview_tabs.iter().all(|tab| tab.editor.inner.language() == expected));
        let path = format!("/tmp/{locale:?}.zip");
        let _ = preview::update(&mut app, PreviewMessage::Open(path.clone()));
        let tab = app.navigation_tabs.preview_tabs.iter().find(|tab| tab.path == path).unwrap();
        assert_eq!(tab.editor.inner.language(), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_16_preview_loading_and_failure_switch_at_display_time() {
    use crate::app::message::{PreviewMessage, preview};
    let (mut app, _) = crate::app::App::new();
    let path = "/tmp/原始%s.txt";
    let _ = preview::update(&mut app, PreviewMessage::Open(path.into()));
    assert_eq!(
        app.preview_state.preview_messages[path].render(I18n::new(DesktopLocale::English)).unwrap(),
        "Loading…"
    );
    let raw = "服务器/%s/%n/原始";
    let _ = preview::update(
        &mut app,
        PreviewMessage::OpenFailed { path: path.into(), error: raw.into() },
    );
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "读取失败："),
        (DesktopLocale::TraditionalChinese, "讀取失敗："),
        (DesktopLocale::English, "Read failed: "),
    ] {
        assert_eq!(
            app.preview_state.preview_messages[path].render(I18n::new(locale)).unwrap(),
            format!("{prefix}{raw}\n")
        );
    }
}
