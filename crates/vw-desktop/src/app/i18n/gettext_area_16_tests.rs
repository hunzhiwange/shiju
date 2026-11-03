use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_16_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "未选择文件"),
        (DesktopLocale::TraditionalChinese, "未選擇檔案"),
        (DesktopLocale::English, "No file selected"),
    ] {
        assert_eq!(I18n::new(locale).__("未选择文件"), expected);
    }
}

#[test]
fn desktop_gettext_area_16_editor_traditional_is_isolated() {
    use iced_code_editor::{Language, Translations};
    let traditional = Translations::new(Language::ChineseTraditional);
    let english = Translations::new(Language::English);
    assert_eq!(traditional.search_placeholder(), "搜尋...");
    assert_eq!(english.search_placeholder(), "Search...");
    assert_eq!(traditional.context_menu_copy(), "複製");
}

#[test]
fn desktop_gettext_area_16_dynamic_parameters_remain_raw() {
    let raw = "服務/%s/%n/原始";
    let message = super::format::Message::new("保存失败: %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "保存失败: "),
        (DesktopLocale::TraditionalChinese, "儲存失敗："),
        (DesktopLocale::English, "Failed to save: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}

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
        app.desktop_locale = locale;
        super::editor::refresh(&mut app);
        assert_eq!(super::editor::lsp_status(&app).unwrap(), format!("{prefix}{raw}"));
    }
    app.lsp_status = Some(raw.into());
    assert_eq!(super::editor::lsp_status(&app).as_deref(), Some(raw));
}

#[test]
fn desktop_gettext_area_16_preview_metadata_preserves_filename() {
    let message = crate::app::preview::preview_description("/tmp/原始%s.zip").unwrap();
    for (locale, kind) in [
        (DesktopLocale::SimplifiedChinese, "压缩包"),
        (DesktopLocale::TraditionalChinese, "壓縮檔"),
        (DesktopLocale::English, "Archive"),
    ] {
        let rendered = message.render(I18n::new(locale)).unwrap();
        assert!(rendered.contains("原始%s.zip"));
        assert!(rendered.contains(kind));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_16_locale_event_updates_existing_and_new_editors() {
    use crate::app::message::{PreviewMessage, preview, settings};
    let (mut app, _) = crate::app::App::new();
    let editor_setting = app.current_language;
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
        assert_eq!(app.current_language, editor_setting);
        assert_eq!(app.git_copy_modal_code_editor.language(), expected);
        assert!(app.preview_tabs.iter().all(|tab| tab.editor.ui_language() == expected));
        let path = format!("/tmp/{locale:?}.zip");
        let _ = preview::update(&mut app, PreviewMessage::Open(path.clone()));
        let tab = app.preview_tabs.iter().find(|tab| tab.path == path).unwrap();
        assert_eq!(tab.editor.ui_language(), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_16_preview_loading_and_failure_switch_at_display_time() {
    use crate::app::message::{PreviewMessage, preview};
    let (mut app, _) = crate::app::App::new();
    let path = "/tmp/原始%s.txt";
    let _ = preview::update(&mut app, PreviewMessage::Open(path.into()));
    assert_eq!(
        app.preview_messages[path].render(I18n::new(DesktopLocale::English)).unwrap(),
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
            app.preview_messages[path].render(I18n::new(locale)).unwrap(),
            format!("{prefix}{raw}\n")
        );
    }
}
