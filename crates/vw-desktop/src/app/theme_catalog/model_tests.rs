use super::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex};

fn valid_palette() -> ThemePaletteHex {
    ThemePaletteHex {
        background: "#1a1a1a".to_string(),
        text: "#f5f5f5".to_string(),
        primary: "#3b82f6".to_string(),
        success: "#22c55e".to_string(),
        warning: "#f59e0b".to_string(),
        danger: "#ef4444".to_string(),
    }
}

fn base_definition() -> ThemeDefinition {
    ThemeDefinition {
        id: "ok-theme".to_string(),
        name: "Ok".to_string(),
        tagline: None,
        palette: valid_palette(),
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    }
}

#[test]
fn theme_definition_rejects_invalid_id() {
    let definition = ThemeDefinition {
        id: "Bad_ID!".to_string(),
        name: "Broken".to_string(),
        tagline: None,
        palette: valid_palette(),
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    };
    assert!(definition.validate().is_err());
}

#[test]
fn theme_definition_truncates_name_and_tagline() {
    let long_name = "n".repeat(100);
    let long_tagline = "t".repeat(200);
    let mut definition = ThemeDefinition {
        id: "ok-theme".to_string(),
        name: long_name,
        tagline: Some(long_tagline),
        palette: valid_palette(),
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    };
    definition.normalize();
    definition.validate().expect("normalized definition should validate");
    assert_eq!(definition.name.chars().count(), 80);
    assert_eq!(definition.tagline.as_ref().map(|s| s.chars().count()), Some(160));
}

#[test]
fn theme_definition_accepts_assets_and_chrome() {
    let definition = ThemeDefinition {
        assets: ThemeAssets {
            cover: Some("cover.png".to_string()),
            background: Some("background.webp".to_string()),
            chat_header: Some("chat_header.png".to_string()),
            chat_body: Some("chat_body.jpg".to_string()),
            chat_input: Some("chat_input.webp".to_string()),
        },
        chrome: ThemeChrome {
            input_bg: Some("#111827".to_string()),
            input_border: Some("#374151".to_string()),
            input_text: Some("#f9fafb".to_string()),
            panel_bg: Some("#1f2937".to_string()),
            panel_border: Some("#4b5563".to_string()),
            button_bg: Some("#2563eb".to_string()),
            button_text: Some("#ffffff".to_string()),
            radius_px: Some(12),
            panel_alpha: Some(200),
            chat_header_alpha: Some(180),
            chat_body_alpha: Some(200),
            chat_input_alpha: Some(220),
            chat_header_image_fit: Some("contain".to_string()),
            chat_body_image_fit: Some("fill".to_string()),
            chat_input_image_fit: Some("scale_down".to_string()),
        },
        ..base_definition()
    };
    definition
        .validate()
        .expect("definition with assets and chrome should validate");
}

#[test]
fn theme_chrome_accepts_per_zone_image_fit() {
    let definition = ThemeDefinition {
        chrome: ThemeChrome {
            chat_header_image_fit: Some("contain".to_string()),
            chat_body_image_fit: Some("fill".to_string()),
            chat_input_image_fit: Some("none".to_string()),
            ..ThemeChrome::default()
        },
        ..base_definition()
    };
    definition
        .validate()
        .expect("per-zone image fit should validate");
    assert_eq!(
        definition.chrome.chat_header_image_fit.as_deref(),
        Some("contain")
    );
    assert_eq!(definition.chrome.chat_body_image_fit.as_deref(), Some("fill"));
    assert_eq!(
        definition.chrome.chat_input_image_fit.as_deref(),
        Some("none")
    );

    let json = serde_json::to_value(&definition).expect("serialize");
    let appearance = json.get("appearance").expect("appearance object");
    assert!(json.get("chrome").is_none(), "new theme JSON must not serialize legacy `chrome`");
    assert!(
        appearance.get("panel_image_fit").is_none(),
        "global panel_image_fit must not be serialized"
    );
    assert_eq!(appearance["chat_header_image_fit"], "contain");
    assert_eq!(appearance["chat_body_image_fit"], "fill");
    assert_eq!(appearance["chat_input_image_fit"], "none");
}

#[test]
fn theme_chrome_folds_legacy_panel_image_fit_into_zones() {
    let json = r##"{
        "id": "legacy-fit",
        "name": "Legacy Fit",
        "palette": {
            "background": "#1a1a1a",
            "text": "#f5f5f5",
            "primary": "#3b82f6",
            "success": "#22c55e",
            "warning": "#f59e0b",
            "danger": "#ef4444"
        },
        "appearance": {
            "panel_image_fit": "contain"
        }
    }"##;
    let definition: ThemeDefinition =
        serde_json::from_str(json).expect("legacy panel_image_fit should deserialize");
    assert_eq!(
        definition.chrome.chat_header_image_fit.as_deref(),
        Some("contain")
    );
    assert_eq!(
        definition.chrome.chat_body_image_fit.as_deref(),
        Some("contain")
    );
    assert_eq!(
        definition.chrome.chat_input_image_fit.as_deref(),
        Some("contain")
    );
    definition
        .validate()
        .expect("folded legacy fit should validate");

    let mixed = r##"{
        "id": "mixed-fit",
        "name": "Mixed Fit",
        "palette": {
            "background": "#1a1a1a",
            "text": "#f5f5f5",
            "primary": "#3b82f6",
            "success": "#22c55e",
            "warning": "#f59e0b",
            "danger": "#ef4444"
        },
        "appearance": {
            "panel_image_fit": "contain",
            "chat_header_image_fit": "fill"
        }
    }"##;
    let mixed_def: ThemeDefinition =
        serde_json::from_str(mixed).expect("mixed legacy + zone fit should deserialize");
    assert_eq!(
        mixed_def.chrome.chat_header_image_fit.as_deref(),
        Some("fill"),
        "explicit zone fit must win over legacy"
    );
    assert_eq!(
        mixed_def.chrome.chat_body_image_fit.as_deref(),
        Some("contain")
    );
    assert_eq!(
        mixed_def.chrome.chat_input_image_fit.as_deref(),
        Some("contain")
    );
}

#[test]
fn theme_definition_rejects_invalid_zone_image_fit() {
    let mut definition = base_definition();
    definition.chrome.chat_body_image_fit = Some("stretch".to_string());
    let err = definition
        .validate()
        .expect_err("unknown zone image fit should be rejected");
    assert!(
        err.to_string().contains("image_fit") || err.to_string().contains("stretch"),
        "unexpected error: {err}"
    );

    definition.chrome.chat_body_image_fit = None;
    definition.chrome.chat_header_image_fit = Some("stretch".to_string());
    assert!(definition.validate().is_err());

    definition.chrome.chat_header_image_fit = None;
    definition.chrome.chat_input_image_fit = Some("stretch".to_string());
    assert!(definition.validate().is_err());
}

#[test]
fn theme_definition_rejects_invalid_asset_name() {
    let mut definition = base_definition();
    definition.assets.cover = Some("../cover.png".to_string());
    let err = definition
        .validate()
        .expect_err("relative path traversal should be rejected");
    assert!(
        err.to_string().contains("cover") || err.to_string().contains("asset"),
        "error should mention the invalid asset: {err}"
    );

    definition.assets.cover = Some("/abs/cover.png".to_string());
    assert!(definition.validate().is_err());

    definition.assets.cover = Some("subdir/cover.png".to_string());
    assert!(definition.validate().is_err());

    definition.assets.cover = Some("cover.png".to_string());
    definition.chrome.input_bg = Some("not-a-color".to_string());
    assert!(definition.validate().is_err());
}

#[test]
fn theme_definition_deserializes_without_optional_sections() {
    let json = r##"{
        "id": "legacy-theme",
        "name": "Legacy",
        "palette": {
            "background": "#1a1a1a",
            "text": "#f5f5f5",
            "primary": "#3b82f6",
            "success": "#22c55e",
            "warning": "#f59e0b",
            "danger": "#ef4444"
        }
    }"##;
    let definition: ThemeDefinition =
        serde_json::from_str(json).expect("theme.json without optional sections should load");
    assert_eq!(definition.assets, ThemeAssets::default());
    assert_eq!(definition.chrome, ThemeChrome::default());
    definition
        .validate()
        .expect("legacy definition should validate");
}
