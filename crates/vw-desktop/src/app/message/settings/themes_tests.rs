use super::{
    ThemeChromeField, ThemeColorPickerTarget, ThemePaletteField, ThemesMessage,
    palette_hex_from_theme, update,
};
use crate::app::App;
use crate::app::message::settings::messages::SettingsMessage;
use crate::app::state::{ThemeEditorTab, ThemesSettingsMode};
use crate::app::theme_catalog::{
    ThemeAssetSources, ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore,
    builtin_overlay_slug, current_themes_root, ensure_bundled_presets, export_theme_dir_to_pack,
    import_theme_pack_into_store, list_bundled_preset_ids, save_builtin_overlay,
    set_test_themes_root, to_iced_theme,
};
use iced::Theme;

fn test_app() -> App {
    App::new().0
}

fn sample_custom_theme() -> ThemeDefinition {
    ThemeDefinition {
        id: "mint-night".to_string(),
        name: "Mint Night".to_string(),
        tagline: Some("Cool mint accents".to_string()),
        palette: ThemePaletteHex {
            background: "#0b1220".to_string(),
            text: "#e2e8f0".to_string(),
            primary: "#34d399".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
        },
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    }
}

fn with_temp_themes_root<T>(f: impl FnOnce(&ThemeStore) -> T) -> T {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());
    set_test_themes_root(Some(temp.path().to_path_buf()));
    let result = f(&store);
    set_test_themes_root(None);
    result
}

#[test]
fn themes_import_pack_installs_and_refreshes_gallery() {
    with_temp_themes_root(|store| {
        store.save(&sample_custom_theme()).expect("save conflicting theme");
        let source = tempfile::tempdir().expect("source themes");
        let source_store = ThemeStore::new(source.path());
        source_store.save(&sample_custom_theme()).expect("save source theme");
        let pack_path = source.path().join("mint-night.vwtheme");
        export_theme_dir_to_pack(source.path().join("mint-night"), &pack_path)
            .expect("export source pack");

        let mut app = test_app();
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ImportPackPicked(Some(
                pack_path.to_string_lossy().into_owned(),
            ))),
        );

        let imported = store.load("mint-night-2").expect("imported conflict copy");
        assert_eq!(imported.name, "Mint Night");
        assert!(
            app.themes_settings.entries.iter().any(|entry| entry.id == "mint-night-2"),
            "imported theme should appear after gallery refresh"
        );
        assert!(
            app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("mint-night-2")),
            "success toast should report allocated id, got {:?}",
            app.active_toast
        );
        assert!(app.themes_settings.error.is_none());
    });
}

#[test]
fn themes_export_pack_writes_vwtheme_file() {
    with_temp_themes_root(|store| {
        store.save(&sample_custom_theme()).expect("save theme");
        let output = tempfile::tempdir().expect("output");
        let pack_path = output.path().join("mint-night.vwtheme");
        let mut app = test_app();

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ExportPackPicked {
                theme_id: "mint-night".to_string(),
                path: Some(pack_path.to_string_lossy().into_owned()),
            }),
        );

        assert!(pack_path.is_file(), "export should write .vwtheme file");
        let imported_root = output.path().join("imported");
        let imported_store = ThemeStore::new(&imported_root);
        let imported =
            import_theme_pack_into_store(&pack_path, &imported_store).expect("read exported pack");
        assert_eq!(imported, sample_custom_theme());
        assert!(
            app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("mint-night")),
            "success toast should name exported theme"
        );
        assert!(app.themes_settings.error.is_none());
    });
}

#[test]
fn themes_export_pack_writes_modified_builtin_overlay() {
    with_temp_themes_root(|_store| {
        let builtin_name = Theme::Dark.to_string();
        let root = current_themes_root().expect("themes root");
        let mut overlay = sample_custom_theme();
        overlay.name = builtin_name.clone();
        save_builtin_overlay(&root, &builtin_name, &overlay).expect("save builtin overlay");
        let output = tempfile::tempdir().expect("output");
        let pack_path = output.path().join("dark.vwtheme");
        let mut app = test_app();

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ExportPackPicked {
                theme_id: builtin_name.clone(),
                path: Some(pack_path.to_string_lossy().into_owned()),
            }),
        );

        assert!(pack_path.is_file(), "modified builtin should export");
        let imported_store = ThemeStore::new(output.path().join("imported"));
        let imported =
            import_theme_pack_into_store(&pack_path, &imported_store).expect("import overlay pack");
        assert_eq!(imported.id, builtin_overlay_slug(&builtin_name));
        assert_eq!(imported.name, builtin_name);
        assert!(app.themes_settings.error.is_none());
    });
}

#[test]
fn themes_export_pack_writes_unmodified_builtin_palette() {
    with_temp_themes_root(|_store| {
        let builtin = Theme::Dark;
        let builtin_name = builtin.to_string();
        let output = tempfile::tempdir().expect("output");
        let pack_path = output.path().join("dark.vwtheme");
        let mut app = test_app();

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ExportPackPicked {
                theme_id: builtin_name.clone(),
                path: Some(pack_path.to_string_lossy().into_owned()),
            }),
        );

        assert!(pack_path.is_file(), "unmodified builtin should export");
        let imported_store = ThemeStore::new(output.path().join("imported"));
        let imported =
            import_theme_pack_into_store(&pack_path, &imported_store).expect("import builtin pack");
        assert_eq!(imported.id, builtin_overlay_slug(&builtin_name));
        assert_eq!(imported.name, builtin_name);
        assert_eq!(imported.palette, palette_hex_from_theme(&builtin));
        assert_eq!(imported.assets, ThemeAssets::default());
        assert_eq!(imported.chrome, ThemeChrome::default());
        assert!(app.themes_settings.error.is_none());
    });
}

#[test]
fn themes_import_invalid_pack_reports_error_without_polluting_store() {
    with_temp_themes_root(|store| {
        let input = tempfile::tempdir().expect("input");
        let pack_path = input.path().join("broken.vwtheme");
        std::fs::write(&pack_path, b"not a zip archive").expect("write invalid pack");
        let mut app = test_app();

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ImportPackPicked(Some(
                pack_path.to_string_lossy().into_owned(),
            ))),
        );

        assert!(
            !store.theme_asset_path("broken", "theme.json").exists(),
            "invalid pack must not create a theme directory"
        );
        assert!(
            app.themes_settings
                .error
                .as_ref()
                .is_some_and(|error| error.contains("导入主题包失败")),
            "invalid pack should set an import error, got {:?}",
            app.themes_settings.error
        );
        assert!(
            app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("导入主题包失败"))
        );
    });
}

#[test]
fn themes_apply_custom_updates_app_theme() {
    with_temp_themes_root(|store| {
        store.save(&sample_custom_theme()).expect("save");
        let expected = to_iced_theme(&sample_custom_theme()).expect("convert");

        let mut app = test_app();
        app.app_theme = Theme::Light;
        app.active_custom_theme_id = None;

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::Refresh));
        assert!(
            app.themes_settings.entries.iter().any(|entry| entry.id == "mint-night"),
            "refresh should load custom theme into gallery"
        );

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyCustom("mint-night".to_string())),
        );

        assert!(matches!(app.app_theme, Theme::Custom(_)));
        assert_eq!(app.app_theme.to_string(), "Mint Night");
        assert_eq!(app.app_theme.palette(), expected.palette());
        assert_eq!(app.active_custom_theme_id.as_deref(), Some("mint-night"));
    });
}

#[test]
fn themes_delete_custom_removes_from_store() {
    with_temp_themes_root(|store| {
        store.save(&sample_custom_theme()).expect("save");
        let theme = to_iced_theme(&sample_custom_theme()).expect("convert");

        let mut app = test_app();
        app.app_theme = theme;
        app.active_custom_theme_id = Some("mint-night".to_string());

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::Refresh));
        assert!(app.themes_settings.entries.iter().any(|entry| entry.id == "mint-night"));

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DeleteCustom("mint-night".to_string())),
        );

        assert!(store.load("mint-night").is_err(), "theme file should be deleted");
        assert!(
            !app.themes_settings.entries.iter().any(|entry| entry.id == "mint-night"),
            "gallery should refresh without deleted theme"
        );
        assert_eq!(app.app_theme.to_string(), Theme::Light.to_string());
        assert!(app.active_custom_theme_id.is_none());
    });
}

#[test]
fn themes_apply_bundled_preset_but_cannot_delete_it() {
    with_temp_themes_root(|store| {
        let root = current_themes_root().expect("themes root");
        ensure_bundled_presets(&root).expect("seed bundled presets");
        let preset_id = list_bundled_preset_ids().into_iter().next().expect("bundled presets");
        let mut app = test_app();

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::Refresh));
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyCustom(preset_id.clone())),
        );
        assert_eq!(app.active_custom_theme_id.as_deref(), Some(preset_id.as_str()));

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DeleteCustom(preset_id.clone())),
        );
        assert!(store.load(&preset_id).is_ok(), "preset must not be deleted");
        assert_eq!(app.active_custom_theme_id.as_deref(), Some(preset_id.as_str()));
        assert!(
            app.themes_settings.error.as_deref().is_some_and(|error| error.contains("不可删除"))
        );
    });
}

#[test]
fn themes_delete_custom_rejects_bundled_preset() {
    with_temp_themes_root(|store| {
        let root = current_themes_root().expect("themes root");
        ensure_bundled_presets(&root).expect("seed bundled presets");
        let preset_id = list_bundled_preset_ids().into_iter().next().expect("bundled preset");
        let theme_json = root.join(&preset_id).join("theme.json");
        let before = std::fs::read(&theme_json).expect("read preset before delete");
        let mut app = test_app();

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DeleteCustom(preset_id.clone())),
        );

        assert!(store.load(&preset_id).is_ok(), "bundled preset must remain loadable");
        assert_eq!(
            std::fs::read(&theme_json).expect("read preset after rejected delete"),
            before,
            "rejected delete must not change bundled preset files"
        );
        assert!(
            app.themes_settings.error.as_deref().is_some_and(|error| error.contains("不可删除")),
            "rejection should set an error, got {:?}",
            app.themes_settings.error
        );
        assert!(
            app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("不可删除")),
            "rejection should show an error toast, got {:?}",
            app.active_toast
        );
    });
}

#[test]
fn themes_restore_preset_default_reseeds_and_clears_modified() {
    with_temp_themes_root(|store| {
        let root = current_themes_root().expect("themes root");
        ensure_bundled_presets(&root).expect("seed bundled presets");
        let preset_id = list_bundled_preset_ids().into_iter().next().expect("bundled preset");
        let original = store.load(&preset_id).expect("load embedded default");
        let mut edited = original.clone();
        edited.name = "user-edited-preset".to_string();
        edited.palette.primary = "#010203".to_string();
        store.save(&edited).expect("save edited preset");

        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::Refresh));
        assert!(
            app.themes_settings
                .entries
                .iter()
                .find(|entry| entry.id == preset_id)
                .is_some_and(|entry| entry.modified),
            "edited preset should be marked modified before restore"
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyCustom(preset_id.clone())),
        );
        let _ =
            update(&mut app, SettingsMessage::Themes(ThemesMessage::EditCustom(preset_id.clone())));

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::RestorePresetDefault(preset_id.clone())),
        );

        assert_eq!(
            store.load(&preset_id).expect("load restored preset"),
            original,
            "restore should reseed the embedded preset definition"
        );
        assert!(
            app.themes_settings
                .entries
                .iter()
                .find(|entry| entry.id == preset_id)
                .is_some_and(|entry| !entry.modified),
            "restored gallery entry should no longer be modified"
        );
        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::List);
        assert!(app.themes_settings.draft.is_none(), "restoring edited preset should close editor");
        assert_eq!(app.active_custom_theme_id.as_deref(), Some(preset_id.as_str()));
        assert_eq!(
            app.app_theme.palette().primary,
            to_iced_theme(&original).expect("convert embedded default").palette().primary,
            "active preset should be reapplied after restore"
        );
    });
}

#[test]
fn themes_new_theme_requested_is_accepted_without_saving() {
    let mut app = test_app();
    let _task = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));
    assert_eq!(app.themes_settings.mode, ThemesSettingsMode::Edit);
    assert_eq!(app.themes_settings.editor_tab, ThemeEditorTab::Basics);
    assert!(app.themes_settings.draft.is_some());
}

#[test]
fn themes_editor_tab_selected_switches_and_closes_color_picker() {
    let mut app = test_app();
    let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));
    let _ = update(
        &mut app,
        SettingsMessage::Themes(ThemesMessage::ColorPickerToggled(
            ThemeColorPickerTarget::Palette(ThemePaletteField::Primary),
        )),
    );
    assert!(app.themes_settings.color_picker_target.is_some());

    let _ = update(
        &mut app,
        SettingsMessage::Themes(ThemesMessage::EditorTabSelected(ThemeEditorTab::Assets)),
    );
    assert_eq!(app.themes_settings.editor_tab, ThemeEditorTab::Assets);
    assert!(app.themes_settings.color_picker_target.is_none());
}

#[test]
fn themes_editor_save_creates_custom_theme() {
    with_temp_themes_root(|store| {
        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftNameChanged("Ocean Breeze".to_string())),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftTaglineChanged(
                "Soft blue accents".to_string(),
            )),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Background,
                "#0a1628".to_string(),
            )),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Text,
                "#e8f1ff".to_string(),
            )),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Primary,
                "#38bdf8".to_string(),
            )),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Success,
                "#22c55e".to_string(),
            )),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Warning,
                "#f59e0b".to_string(),
            )),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Danger,
                "#ef4444".to_string(),
            )),
        );

        let draft_id = app.themes_settings.draft.as_ref().expect("draft").id.clone();

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::SaveDraft));

        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::List);
        assert!(app.themes_settings.draft.is_none());
        assert!(
            app.themes_settings
                .entries
                .iter()
                .any(|entry| entry.id == draft_id && entry.name == "Ocean Breeze"),
            "saved theme should appear in gallery"
        );
        let loaded = store.load(&draft_id).expect("theme.json on disk");
        assert_eq!(loaded.name, "Ocean Breeze");
        assert_eq!(loaded.palette.primary, "#38bdf8");
    });
}

#[test]
fn themes_editor_rejects_invalid_palette_hex() {
    with_temp_themes_root(|store| {
        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftNameChanged("Bad Colors".to_string())),
        );
        let draft_id = app.themes_settings.draft.as_ref().expect("draft").id.clone();
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Primary,
                "zzz".to_string(),
            )),
        );

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::SaveDraft));

        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::Edit);
        assert!(app.themes_settings.draft.is_some());
        assert!(
            app.themes_settings.error.as_ref().is_some_and(|err| {
                err.contains("primary") || err.contains("hex") || err.contains("颜色")
            }),
            "should surface validation error, got {:?}",
            app.themes_settings.error
        );
        assert!(store.load(&draft_id).is_err(), "invalid palette must not write theme.json");
    });
}

/// 最小合法 1×1 PNG。
fn minimal_png_bytes() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8,
        0xff, 0xff, 0x3f, 0x00, 0x05, 0xfe, 0x02, 0xfe, 0xdc, 0xcc, 0x59, 0xe7, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

#[test]
fn themes_editor_save_persists_cover_image() {
    with_temp_themes_root(|store| {
        let fixtures = tempfile::tempdir().expect("fixtures");
        let cover_src = fixtures.path().join("photo.png");
        std::fs::write(&cover_src, minimal_png_bytes()).expect("write cover");

        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftNameChanged("Cover Theme".to_string())),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::CoverPicked(Some(
                cover_src.to_string_lossy().into_owned(),
            ))),
        );

        let draft_id = app.themes_settings.draft.as_ref().expect("draft").id.clone();

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::SaveDraft));

        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::List);
        let loaded = store.load(&draft_id).expect("theme.json on disk");
        assert_eq!(loaded.assets.cover.as_deref(), Some("cover.png"));
        let cover_file = store.theme_asset_path(&draft_id, "cover.png");
        assert!(cover_file.is_file(), "cover.png should exist at {}", cover_file.display());
        assert_eq!(std::fs::read(&cover_file).expect("read cover"), minimal_png_bytes());

        let gallery = app
            .themes_settings
            .entries
            .iter()
            .find(|entry| entry.id == draft_id)
            .expect("gallery entry");
        assert!(
            gallery.cover_path.as_ref().is_some_and(|path| {
                std::path::Path::new(path).file_name().and_then(|n| n.to_str()) == Some("cover.png")
            }),
            "gallery entry should expose cover path, got {:?}",
            gallery.cover_path
        );
    });
}

#[test]
fn themes_editor_save_persists_chat_panel_assets_and_resolves_skin() {
    with_temp_themes_root(|store| {
        let fixtures = tempfile::tempdir().expect("fixtures");
        let header_src = fixtures.path().join("header.png");
        std::fs::write(&header_src, minimal_png_bytes()).expect("write header");

        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftNameChanged("Panel Theme".to_string())),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ChatHeaderPicked(Some(
                header_src.to_string_lossy().into_owned(),
            ))),
        );

        let draft_id = app.themes_settings.draft.as_ref().expect("draft").id.clone();

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::SaveDraft));

        let loaded = store.load(&draft_id).expect("theme.json on disk");
        assert_eq!(loaded.assets.chat_header.as_deref(), Some("chat_header.png"));
        assert!(store.theme_asset_path(&draft_id, "chat_header.png").is_file());

        let _ =
            update(&mut app, SettingsMessage::Themes(ThemesMessage::ApplyCustom(draft_id.clone())));
        let skin = app.active_theme_skin.as_ref().expect("active skin");
        assert_eq!(
            skin.chat_header_path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()),
            Some("chat_header.png")
        );
        assert!(skin.chat_header_path.as_ref().is_some_and(|p| p.is_file()));
    });
}

#[test]
fn themes_editor_saves_per_zone_image_fit_and_alpha() {
    with_temp_themes_root(|store| {
        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftNameChanged("Zone Fit Theme".to_string())),
        );

        for (field, value) in [
            (ThemeChromeField::PanelAlpha, "200"),
            (ThemeChromeField::ChatHeaderAlpha, "160"),
            (ThemeChromeField::ChatBodyAlpha, "180"),
            (ThemeChromeField::ChatInputAlpha, "220"),
            (ThemeChromeField::ChatHeaderImageFit, "contain"),
            (ThemeChromeField::ChatBodyImageFit, "fill"),
            (ThemeChromeField::ChatInputImageFit, "scale_down"),
        ] {
            let _ = update(
                &mut app,
                SettingsMessage::Themes(ThemesMessage::DraftChromeChanged(
                    field,
                    value.to_string(),
                )),
            );
        }

        let draft = app.themes_settings.draft.as_ref().expect("draft");
        assert_eq!(draft.chrome_panel_alpha, "200");
        assert_eq!(draft.chrome_chat_header_alpha, "160");
        assert_eq!(draft.chrome_chat_body_alpha, "180");
        assert_eq!(draft.chrome_chat_input_alpha, "220");
        assert_eq!(draft.chrome_chat_header_image_fit, "contain");
        assert_eq!(draft.chrome_chat_body_image_fit, "fill");
        assert_eq!(draft.chrome_chat_input_image_fit, "scale_down");

        let draft_id = draft.id.clone();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::SaveDraft));

        let loaded = store.load(&draft_id).expect("theme.json on disk");
        assert_eq!(loaded.chrome.panel_alpha, Some(200));
        assert_eq!(loaded.chrome.chat_header_alpha, Some(160));
        assert_eq!(loaded.chrome.chat_body_alpha, Some(180));
        assert_eq!(loaded.chrome.chat_input_alpha, Some(220));
        assert_eq!(loaded.chrome.chat_header_image_fit.as_deref(), Some("contain"));
        assert_eq!(loaded.chrome.chat_body_image_fit.as_deref(), Some("fill"));
        assert_eq!(loaded.chrome.chat_input_image_fit.as_deref(), Some("scale_down"));

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::EditCustom(draft_id)));
        let draft = app.themes_settings.draft.as_ref().expect("reopened draft");
        assert_eq!(draft.chrome_chat_header_image_fit, "contain");
        assert_eq!(draft.chrome_chat_body_image_fit, "fill");
        assert_eq!(draft.chrome_chat_input_image_fit, "scale_down");
        assert_eq!(draft.chrome_chat_header_alpha, "160");
        assert_eq!(draft.chrome_chat_body_alpha, "180");
        assert_eq!(draft.chrome_chat_input_alpha, "220");
        assert_eq!(draft.chrome_panel_alpha, "200");
    });
}

#[test]
fn themes_editor_rejects_invalid_chrome_hex() {
    with_temp_themes_root(|store| {
        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftNameChanged("Bad Chrome".to_string())),
        );
        let draft_id = app.themes_settings.draft.as_ref().expect("draft").id.clone();
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftChromeChanged(
                ThemeChromeField::InputBg,
                "zzz".to_string(),
            )),
        );

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::SaveDraft));

        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::Edit);
        assert!(app.themes_settings.draft.is_some());
        assert!(
            app.themes_settings.error.as_ref().is_some_and(|err| {
                err.contains("input_bg") || err.contains("hex") || err.contains("颜色")
            }),
            "should surface chrome validation error, got {:?}",
            app.themes_settings.error
        );
        assert!(store.load(&draft_id).is_err(), "invalid chrome must not write theme.json");
    });
}

#[test]
fn themes_editor_save_refreshes_applied_custom_theme() {
    with_temp_themes_root(|store| {
        store.save(&sample_custom_theme()).expect("save");
        let mut app = test_app();
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyCustom("mint-night".to_string())),
        );
        assert_eq!(app.active_custom_theme_id.as_deref(), Some("mint-night"));

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::EditCustom("mint-night".to_string())),
        );
        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::Edit);

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Primary,
                "#14b8a6".to_string(),
            )),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftNameChanged(
                "Mint Night Updated".to_string(),
            )),
        );

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::SaveDraft));

        let loaded = store.load("mint-night").expect("updated on disk");
        assert_eq!(loaded.id, "mint-night");
        assert_eq!(loaded.name, "Mint Night Updated");
        assert_eq!(loaded.palette.primary, "#14b8a6");

        let expected = to_iced_theme(&loaded).expect("convert");
        assert!(matches!(app.app_theme, Theme::Custom(_)));
        assert_eq!(app.app_theme.to_string(), "Mint Night Updated");
        assert_eq!(app.app_theme.palette(), expected.palette());
        assert_eq!(app.active_custom_theme_id.as_deref(), Some("mint-night"));
        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::List);
    });
}

#[test]
fn active_theme_background_path_set_on_apply_custom() {
    with_temp_themes_root(|store| {
        let fixtures = tempfile::tempdir().expect("fixtures");
        let background_src = fixtures.path().join("wall.png");
        std::fs::write(&background_src, minimal_png_bytes()).expect("write background");

        let mut definition = sample_custom_theme();
        definition.id = "wall-night".to_string();
        definition.name = "Wall Night".to_string();
        let saved = store
            .save_with_sources(
                &definition,
                &ThemeAssetSources {
                    cover: None,
                    background: Some(background_src),
                    ..Default::default()
                },
            )
            .expect("save with background");
        assert_eq!(saved.assets.background.as_deref(), Some("background.png"));

        let mut app = test_app();
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyCustom("wall-night".to_string())),
        );

        assert_eq!(app.active_custom_theme_id.as_deref(), Some("wall-night"));
        let skin = app.active_theme_skin.as_ref().expect("active skin");
        assert_eq!(
            skin.background_path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()),
            Some("background.png")
        );
        assert!(
            skin.background_path.as_ref().is_some_and(|p| p.is_file()),
            "background path should point to theme directory file"
        );
    });
}

#[test]
fn active_theme_background_cleared_on_builtin() {
    with_temp_themes_root(|store| {
        let fixtures = tempfile::tempdir().expect("fixtures");
        let background_src = fixtures.path().join("wall.png");
        std::fs::write(&background_src, minimal_png_bytes()).expect("write background");

        let mut definition = sample_custom_theme();
        definition.id = "wall-night".to_string();
        let _ = store
            .save_with_sources(
                &definition,
                &ThemeAssetSources {
                    cover: None,
                    background: Some(background_src),
                    ..Default::default()
                },
            )
            .expect("save");

        let mut app = test_app();
        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyCustom("wall-night".to_string())),
        );
        assert!(app.active_theme_skin.is_some());

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyBuiltin(Theme::Light.to_string())),
        );

        assert!(app.active_custom_theme_id.is_none());
        assert!(app.active_theme_skin.is_none(), "builtin theme must clear wallpaper cache");
    });
}

#[test]
fn themes_edit_builtin_save_and_restore_overlay() {
    with_temp_themes_root(|_store| {
        let builtin_name = Theme::Dark.to_string();
        let mut app = test_app();

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::EditBuiltin(builtin_name.clone())),
        );
        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::Edit);
        let draft = app.themes_settings.draft.as_ref().expect("draft");
        assert_eq!(draft.builtin_base.as_deref(), Some(builtin_name.as_str()));
        assert!(draft.id_readonly);

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
                ThemePaletteField::Primary,
                "#14b8a6".to_string(),
            )),
        );
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::SaveDraft));

        assert_eq!(app.themes_settings.mode, ThemesSettingsMode::List);
        let root = crate::app::theme_catalog::current_themes_root().expect("root");
        let overlay = crate::app::theme_catalog::load_builtin_overlay(&root, &builtin_name)
            .expect("load")
            .expect("overlay saved");
        assert_eq!(overlay.palette.primary, "#14b8a6");

        let gallery = app
            .themes_settings
            .entries
            .iter()
            .find(|entry| entry.id == builtin_name)
            .expect("gallery builtin");
        assert!(gallery.modified, "gallery should mark builtin as modified");
        assert_eq!(gallery.palette.primary, "#14b8a6");

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyBuiltin(builtin_name.clone())),
        );
        assert_eq!(app.app_theme.to_string(), builtin_name);
        assert_eq!(app.app_theme.palette().primary, iced::Color::from_rgb8(0x14, 0xb8, 0xa6));
        assert!(app.active_custom_theme_id.is_none());

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::RestoreBuiltinDefault(builtin_name.clone())),
        );

        let overlay_dir = root
            .join("overlays")
            .join(crate::app::theme_catalog::builtin_overlay_slug(&builtin_name));
        assert!(!overlay_dir.exists(), "restore must remove overlay directory");
        assert_eq!(app.app_theme, Theme::Dark);
        assert_eq!(app.app_theme.palette(), Theme::Dark.palette());
        let gallery = app
            .themes_settings
            .entries
            .iter()
            .find(|entry| entry.id == builtin_name)
            .expect("gallery builtin after restore");
        assert!(!gallery.modified);
    });
}

#[test]
fn themes_editor_color_picker_sets_palette_hex() {
    let mut app = test_app();
    let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));

    let _ = update(
        &mut app,
        SettingsMessage::Themes(ThemesMessage::ColorPickerToggled(
            ThemeColorPickerTarget::Palette(ThemePaletteField::Primary),
        )),
    );
    assert_eq!(
        app.themes_settings.color_picker_target,
        Some(ThemeColorPickerTarget::Palette(ThemePaletteField::Primary))
    );

    let _ = update(
        &mut app,
        SettingsMessage::Themes(ThemesMessage::ColorPickerColorChanged("#aabbcc".to_string())),
    );
    let draft = app.themes_settings.draft.as_ref().expect("draft");
    assert_eq!(draft.primary, "#aabbcc");

    let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::ColorPickerClosed));
    assert!(app.themes_settings.color_picker_target.is_none());
    let draft = app.themes_settings.draft.as_ref().expect("draft after close");
    assert_eq!(draft.primary, "#aabbcc", "closing picker must keep the committed hex");
}

#[test]
fn themes_editor_manual_hex_input_updates_draft() {
    let mut app = test_app();
    let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::NewThemeRequested));

    let _ = update(
        &mut app,
        SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
            ThemePaletteField::Background,
            "#112233".to_string(),
        )),
    );
    let draft = app.themes_settings.draft.as_ref().expect("draft");
    assert_eq!(draft.background, "#112233");

    let _ = update(
        &mut app,
        SettingsMessage::Themes(ThemesMessage::DraftChromeChanged(
            ThemeChromeField::InputBg,
            "#445566".to_string(),
        )),
    );
    let draft = app.themes_settings.draft.as_ref().expect("draft");
    assert_eq!(draft.chrome_input_bg, "#445566");
}

#[test]
fn themes_gallery_page_clamps_on_refresh_after_shrink() {
    with_temp_themes_root(|store| {
        // 额外自定义主题把总数推过一页，方便切到末页后再缩回。
        for index in 0..6 {
            let mut theme = sample_custom_theme();
            theme.id = format!("extra-{index}");
            theme.name = format!("Extra {index}");
            store.save(&theme).expect("save extra");
        }

        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::Refresh));
        let total = app.themes_settings.entries.len();
        assert!(
            total > crate::app::components::system_settings_themes::gallery::GALLERY_PAGE_SIZE,
            "expected more than one page, got {total}"
        );

        let page_size = crate::app::components::system_settings_themes::gallery::GALLERY_PAGE_SIZE;
        let last_page = (total - 1) / page_size;
        app.themes_settings.page_index = last_page;
        assert!(app.themes_settings.page_index > 0);

        // 删掉所有自定义主题后刷新，页码应夹紧回 0（仅剩内置一页内）。
        for index in 0..6 {
            store.delete(&format!("extra-{index}")).expect("delete");
        }
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::Refresh));

        let remaining = app.themes_settings.entries.len();
        let max_page = remaining.div_ceil(page_size).saturating_sub(1);
        assert!(
            app.themes_settings.page_index <= max_page,
            "page_index {} should clamp to <= {max_page} after shrink to {remaining}",
            app.themes_settings.page_index
        );
    });
}

#[test]
fn themes_gallery_next_prev_page_clamps() {
    with_temp_themes_root(|store| {
        store.save(&sample_custom_theme()).expect("save");
        let mut app = test_app();
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::Refresh));

        app.themes_settings.page_index = 0;
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::GalleryPrevPage));
        assert_eq!(app.themes_settings.page_index, 0);

        let total = app.themes_settings.entries.len();
        let page_size = crate::app::components::system_settings_themes::gallery::GALLERY_PAGE_SIZE;
        let max_page = total.div_ceil(page_size).saturating_sub(1);
        app.themes_settings.page_index = max_page;
        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::GalleryNextPage));
        assert_eq!(app.themes_settings.page_index, max_page);

        if max_page > 0 {
            app.themes_settings.page_index = 0;
            let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::GalleryNextPage));
            assert_eq!(app.themes_settings.page_index, 1);
        }
    });
}
