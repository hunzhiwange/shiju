use super::gallery::{
    GALLERY_COLUMNS, GALLERY_PAGE_SIZE, ThemeGallerySource, build_gallery_entries,
    clamp_gallery_page, gallery_page_count, gallery_page_range, palette_hex_from_theme,
};
use crate::app::components::system_settings::SystemTab;
use crate::app::theme_catalog::{
    ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore, ensure_bundled_presets,
    list_bundled_preset_ids, set_test_themes_root,
};
use iced::Theme;

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

#[test]
fn system_tab_all_contains_themes() {
    assert!(
        SystemTab::all().contains(&SystemTab::Themes),
        "SystemTab::all() should include Themes"
    );
    assert_eq!(SystemTab::Themes.to_string(), "主题");
}

#[test]
fn themes_gallery_lists_builtin_and_custom() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());
    store.save(&sample_custom_theme()).expect("save custom theme");
    set_test_themes_root(Some(temp.path().to_path_buf()));

    let entries = build_gallery_entries(Some(&store)).expect("build gallery");

    let builtin_count =
        entries.iter().filter(|entry| entry.source == ThemeGallerySource::Builtin).count();
    assert_eq!(builtin_count, Theme::ALL.len());
    assert!(
        entries.iter().any(|entry| {
            entry.source == ThemeGallerySource::Builtin && entry.name == Theme::Light.to_string()
        }),
        "gallery should include builtin Light"
    );

    let custom = entries
        .iter()
        .find(|entry| entry.source == ThemeGallerySource::Custom && entry.id == "mint-night")
        .expect("custom theme present");
    assert_eq!(custom.name, "Mint Night");
    assert_eq!(custom.tagline.as_deref(), Some("Cool mint accents"));
    assert_eq!(custom.palette.primary, "#34d399");
    assert!(custom.cover_path.is_none());

    let light_palette = palette_hex_from_theme(&Theme::Light);
    let light_entry = entries
        .iter()
        .find(|entry| entry.source == ThemeGallerySource::Builtin && entry.id == "Light")
        .expect("Light entry");
    assert_eq!(light_entry.palette, light_palette);

    set_test_themes_root(None);
}

#[test]
fn build_gallery_entries_marks_bundled_presets_and_cover() {
    let ids = list_bundled_preset_ids();
    assert!(!ids.is_empty(), "bundled presets must be embedded for gallery source test");
    let sample_id = ids[0].clone();

    let temp = tempfile::tempdir().expect("tempdir");
    let themes_root = temp.path().join("themes");
    let store = ThemeStore::new(&themes_root);
    ensure_bundled_presets(&themes_root).expect("seed bundled presets");
    store.save(&sample_custom_theme()).expect("save non-bundled custom");

    set_test_themes_root(Some(themes_root));
    let entries = build_gallery_entries(Some(&store)).expect("build gallery");

    let preset = entries
        .iter()
        .find(|entry| entry.id == sample_id)
        .expect("seeded preset should appear in gallery");
    assert_eq!(
        preset.source,
        ThemeGallerySource::Preset,
        "bundled id `{sample_id}` must be marked Preset, not Custom/Builtin"
    );
    assert!(
        preset
            .cover_path
            .as_ref()
            .is_some_and(|path| !path.is_empty() && std::path::Path::new(path).is_file()),
        "preset cover_path should be a non-empty existing file after seed; got {:?}",
        preset.cover_path
    );

    let custom =
        entries.iter().find(|entry| entry.id == "mint-night").expect("plain custom theme present");
    assert_eq!(custom.source, ThemeGallerySource::Custom, "non-bundled custom must stay Custom");

    let last_builtin = entries
        .iter()
        .rposition(|entry| entry.source == ThemeGallerySource::Builtin)
        .expect("builtins present");
    let first_preset = entries
        .iter()
        .position(|entry| entry.source == ThemeGallerySource::Preset)
        .expect("presets present");
    let first_custom = entries
        .iter()
        .position(|entry| entry.source == ThemeGallerySource::Custom)
        .expect("customs present");
    assert!(
        last_builtin < first_preset && first_preset < first_custom,
        "order should be Builtin → Preset → Custom; last_builtin={last_builtin}, first_preset={first_preset}, first_custom={first_custom}"
    );

    set_test_themes_root(None);
}

#[test]
fn build_gallery_entries_marks_modified_bundled_preset() {
    let sample_id = list_bundled_preset_ids().into_iter().next().expect("bundled preset");
    let temp = tempfile::tempdir().expect("tempdir");
    let themes_root = temp.path().join("themes");
    let store = ThemeStore::new(&themes_root);
    ensure_bundled_presets(&themes_root).expect("seed bundled presets");
    set_test_themes_root(Some(themes_root));

    let clean_entries = build_gallery_entries(Some(&store)).expect("build clean gallery");
    let clean_preset =
        clean_entries.iter().find(|entry| entry.id == sample_id).expect("clean preset entry");
    assert!(!clean_preset.modified, "freshly seeded preset should not be marked modified");

    let mut edited = store.load(&sample_id).expect("load seeded preset");
    edited.name = "user-edited-preset".to_string();
    store.save(&edited).expect("save edited preset");

    let edited_entries = build_gallery_entries(Some(&store)).expect("build edited gallery");
    let edited_preset =
        edited_entries.iter().find(|entry| entry.id == sample_id).expect("edited preset entry");
    assert!(edited_preset.modified, "locally edited bundled preset should be marked modified");

    set_test_themes_root(None);
}

#[test]
fn themes_gallery_exposes_cover_path_when_asset_present() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixtures = temp.path().join("fixtures");
    std::fs::create_dir_all(&fixtures).expect("fixtures");
    let cover_src = fixtures.join("photo.png");
    std::fs::write(
        &cover_src,
        [
            0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xd7, 0x63, 0xf8, 0xff, 0xff, 0x3f, 0x00, 0x05, 0xfe, 0x02, 0xfe, 0xdc, 0xcc, 0x59,
            0xe7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ],
    )
    .expect("write cover");

    let store = ThemeStore::new(temp.path().join("themes"));
    let mut definition = sample_custom_theme();
    let saved = store
        .save_with_sources(
            &definition,
            &crate::app::theme_catalog::ThemeAssetSources {
                cover: Some(cover_src),
                background: None,
                ..Default::default()
            },
        )
        .expect("save with cover");
    definition = saved;

    set_test_themes_root(Some(temp.path().join("themes")));
    let entries = build_gallery_entries(Some(&store)).expect("build gallery");
    let custom = entries.iter().find(|entry| entry.id == definition.id).expect("custom");
    assert!(
        custom.cover_path.as_ref().is_some_and(|path| {
            std::path::Path::new(path).file_name().and_then(|n| n.to_str()) == Some("cover.png")
        }),
        "expected cover path, got {:?}",
        custom.cover_path
    );
    set_test_themes_root(None);
}

#[test]
fn themes_gallery_pages_six_cards_three_columns() {
    assert_eq!(GALLERY_COLUMNS, 3);
    assert_eq!(GALLERY_PAGE_SIZE, 6);
    assert_eq!(GALLERY_PAGE_SIZE % GALLERY_COLUMNS, 0);

    let total = 13;
    assert_eq!(gallery_page_count(total), 3);

    let (start, end) = gallery_page_range(total, 0);
    assert_eq!((start, end), (0, 6));
    assert_eq!(end - start, 6);

    let (start, end) = gallery_page_range(total, 1);
    assert_eq!((start, end), (6, 12));
    assert_eq!(end - start, 6);

    let (start, end) = gallery_page_range(total, 2);
    assert_eq!((start, end), (12, 13));
    assert_eq!(end - start, 1, "last page should keep remainder");
}

#[test]
fn themes_gallery_page_clamps_after_shrink() {
    // 13 条时末页为 2；下一页/越界均夹紧到末页。
    assert_eq!(clamp_gallery_page(0, 13), 0);
    assert_eq!(clamp_gallery_page(2, 13), 2);
    assert_eq!(clamp_gallery_page(100, 13), 2);

    // 条目缩减到一页后，原 page_index=2 回退到 0。
    assert_eq!(clamp_gallery_page(2, 6), 0);
    assert_eq!(clamp_gallery_page(1, 6), 0);

    // 7 条仍有两页时，page 1 保留。
    assert_eq!(clamp_gallery_page(1, 7), 1);
    assert_eq!(gallery_page_count(0), 1);
    assert_eq!(clamp_gallery_page(3, 0), 0);
}
