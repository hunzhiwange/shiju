//! 验证主题编辑草稿、标识分配、颜色解析和表单校验。
//!
//! 草稿在新建和既有主题间切换，颜色输入覆盖合法、非法与冲突标识分配。

use super::{
    allocate_unique_theme_id, draft_from_definition, editor_tab_labels,
    new_theme_draft, slugify_theme_id, try_definition_from_draft,
};
use super::super::state::{ThemeEditorDraft, ThemeEditorTab};
use desktop_theme::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex};

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
fn slugify_theme_id_produces_valid_id() {
    assert_eq!(slugify_theme_id("Mint Night"), "mint-night");
    assert_eq!(slugify_theme_id("  Cool_Theme!! "), "cool_theme");
}

#[test]
fn allocate_unique_theme_id_appends_suffix_on_conflict() {
    let existing = vec!["mint-night".to_string(), "mint-night-2".to_string()];
    assert_eq!(allocate_unique_theme_id("mint-night", &existing), "mint-night-3");
}

#[test]
fn new_theme_draft_starts_editable_id() {
    let draft = new_theme_draft(&[]);
    assert!(!draft.id_readonly);
    assert!(!draft.id.is_empty());
    assert_eq!(draft.id, "theme");
}

#[test]
fn draft_from_definition_locks_id() {
    let draft = draft_from_definition(&sample_custom_theme());
    assert!(draft.id_readonly);
    assert_eq!(draft.id, "mint-night");
    assert_eq!(draft.name, "Mint Night");
    assert_eq!(draft.primary, "#34d399");
}

#[test]
fn try_definition_from_draft_rejects_invalid_hex() {
    let mut draft = draft_from_definition(&sample_custom_theme());
    draft.primary = "not-a-color".to_string();
    let err = try_definition_from_draft(&draft).expect_err("invalid hex");
    assert!(err.contains("primary") || err.contains("hex") || err.contains("颜色"));
}

#[test]
fn try_definition_from_draft_rejects_empty_name() {
    let mut draft = draft_from_definition(&sample_custom_theme());
    draft.name.clear();
    let err = try_definition_from_draft(&draft).expect_err("empty name");
    assert!(err.contains("名称") || err.to_lowercase().contains("name"));
}

#[test]
fn themes_editor_rejects_invalid_chrome_hex() {
    let mut draft = draft_from_definition(&sample_custom_theme());
    draft.chrome_input_bg = "not-hex".to_string();
    let err = try_definition_from_draft(&draft).expect_err("invalid chrome hex");
    assert!(
        err.contains("input_bg") || err.contains("hex") || err.contains("颜色"),
        "unexpected error: {err}"
    );
}

#[test]
fn try_definition_from_draft_keeps_relative_assets_and_chrome() {
    let mut definition = sample_custom_theme();
    definition.assets.cover = Some("cover.png".to_string());
    definition.chrome.input_bg = Some("#111827".to_string());
    definition.chrome.radius_px = Some(10);

    let draft = draft_from_definition(&definition);
    assert_eq!(draft.cover_relative.as_deref(), Some("cover.png"));
    assert_eq!(draft.chrome_input_bg, "#111827");
    assert_eq!(draft.chrome_radius_px, "10");

    let rebuilt = try_definition_from_draft(&draft).expect("valid draft");
    assert_eq!(rebuilt.assets.cover.as_deref(), Some("cover.png"));
    assert_eq!(rebuilt.chrome.input_bg.as_deref(), Some("#111827"));
    assert_eq!(rebuilt.chrome.radius_px, Some(10));
}

#[test]
fn try_definition_from_draft_keeps_per_zone_image_fit_and_alpha() {
    let mut definition = sample_custom_theme();
    definition.chrome.panel_alpha = Some(200);
    definition.chrome.chat_header_alpha = Some(160);
    definition.chrome.chat_body_alpha = Some(180);
    definition.chrome.chat_input_alpha = Some(220);
    definition.chrome.chat_header_image_fit = Some("contain".to_string());
    definition.chrome.chat_body_image_fit = Some("fill".to_string());
    definition.chrome.chat_input_image_fit = Some("scale_down".to_string());

    let draft = draft_from_definition(&definition);
    assert_eq!(draft.chrome_panel_alpha, "200");
    assert_eq!(draft.chrome_chat_header_alpha, "160");
    assert_eq!(draft.chrome_chat_body_alpha, "180");
    assert_eq!(draft.chrome_chat_input_alpha, "220");
    assert_eq!(draft.chrome_chat_header_image_fit, "contain");
    assert_eq!(draft.chrome_chat_body_image_fit, "fill");
    assert_eq!(draft.chrome_chat_input_image_fit, "scale_down");

    let rebuilt = try_definition_from_draft(&draft).expect("valid draft");
    assert_eq!(rebuilt.chrome.panel_alpha, Some(200));
    assert_eq!(rebuilt.chrome.chat_header_alpha, Some(160));
    assert_eq!(rebuilt.chrome.chat_body_alpha, Some(180));
    assert_eq!(rebuilt.chrome.chat_input_alpha, Some(220));
    assert_eq!(rebuilt.chrome.chat_header_image_fit.as_deref(), Some("contain"));
    assert_eq!(rebuilt.chrome.chat_body_image_fit.as_deref(), Some("fill"));
    assert_eq!(rebuilt.chrome.chat_input_image_fit.as_deref(), Some("scale_down"));
}

#[test]
fn editor_tab_labels_keep_expected_order_and_unique_tabs() {
    let labels = editor_tab_labels();
    assert_eq!(
        labels,
        &[
            ("基本信息", ThemeEditorTab::Basics),
            ("调色板", ThemeEditorTab::Palette),
            ("资源", ThemeEditorTab::Assets),
            ("控件皮肤", ThemeEditorTab::Chrome),
            ("预览", ThemeEditorTab::Preview),
        ]
    );
    for (_, expected_tab) in labels {
        assert_eq!(labels.iter().filter(|(_, tab)| tab == expected_tab).count(), 1);
    }
}
