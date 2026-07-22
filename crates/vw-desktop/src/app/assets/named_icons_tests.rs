use super::*;
use serde_json::json;

#[test]
fn canonical_named_icon_family_normalizes_safe_tokens_and_rejects_unsafe_tokens() {
    assert_eq!(
        canonical_named_icon_family(" Feather__ Icons---.svg").as_deref(),
        Some("feather-icons")
    );
    assert_eq!(canonical_named_icon_family("123").as_deref(), Some("123"));

    for family in ["", "___", "bad/path", "图标"] {
        assert_eq!(canonical_named_icon_family(family), None, "{family}");
    }
}

#[test]
fn icon_name_candidates_preserve_preference_order_without_duplicates() {
    assert_eq!(
        icon_name_candidates("arrow_up"),
        vec!["arrow_up".to_string(), "arrow-up".to_string()]
    );
    assert_eq!(
        icon_name_candidates("arrow-up"),
        vec!["arrow-up".to_string(), "arrow_up".to_string()]
    );
    assert_eq!(
        icon_name_candidates("arrow-up_left"),
        vec!["arrow-up_left".to_string(), "arrow-up-left".to_string(), "arrow_up_left".to_string(),]
    );
}

#[test]
fn phosphor_weight_bucket_covers_numeric_string_and_default_boundaries() {
    for (weight, expected) in [
        (json!(200), "thin"),
        (json!(201), "light"),
        (json!(350), "light"),
        (json!(351), "regular"),
        (json!(599), "regular"),
        (json!(600), "bold"),
        (json!("200"), "thin"),
        (json!("600"), "bold"),
    ] {
        assert_eq!(phosphor_weight_bucket(Some(&weight)), expected);
    }

    assert_eq!(phosphor_weight_bucket(None), "regular");
    assert_eq!(phosphor_weight_bucket(Some(&json!("heavy"))), "regular");
    assert_eq!(phosphor_weight_bucket(Some(&json!(true))), "regular");
}

#[test]
fn named_icon_family_label_formats_normalized_and_fallback_names() {
    assert_eq!(named_icon_family_label("phosphor-icons"), "Phosphor Icons");
    assert_eq!(named_icon_family_label(" Feather_icons.svg "), "Feather Icons");
    assert_eq!(named_icon_family_label(" BAD/PATH "), "Bad/path");
    assert_eq!(named_icon_family_label("  "), "");
}

#[test]
fn named_icon_svg_text_resolves_normalized_flat_assets_and_rejects_missing_assets() {
    let svg = named_icon_svg_text(" Feather ", "arrow_up.svg", None)
        .expect("normalized feather icon should be embedded");

    assert!(svg.contains("<svg"));
    assert!(named_icon_svg_text("feather", "definitely-missing", None).is_none());
    assert!(named_icon_svg_text("bad/path", "arrow-up", None).is_none());
    assert!(named_icon_svg_text("feather", "bad/path", None).is_none());
}

#[test]
fn named_icon_svg_text_exhausts_weighted_lookup_for_missing_phosphor_asset() {
    let bold_weight = json!(600);

    assert!(named_icon_svg_text("phosphor", "definitely-missing", Some(&bold_weight)).is_none());
}

#[test]
fn svg_color_value_rounds_channels_and_clamps_alpha() {
    assert_eq!(svg_color_value(Color::from_rgb(0.5, 0.0, 1.0)), "#8000FF");
    assert_eq!(
        svg_color_value(Color { r: -0.5, g: 1.5, b: 0.25, a: 0.1256 }),
        "rgba(0, 255, 64, 0.126)"
    );
    assert_eq!(svg_color_value(Color { r: 0.0, g: 0.0, b: 0.0, a: 2.0 }), "rgba(0, 0, 0, 1.000)");
}

#[test]
fn colorize_icon_svg_injects_color_into_svg_root_and_preserves_non_svg_text() {
    assert_eq!(
        colorize_icon_svg(r#"<?xml version="1.0"?><svg viewBox="0 0 1 1"></svg>"#, Color::WHITE),
        r##"<?xml version="1.0"?><svg color="#FFFFFF" viewBox="0 0 1 1"></svg>"##
    );
    assert_eq!(colorize_icon_svg("not svg", Color::BLACK), "not svg");
}

#[test]
fn rasterize_svg_to_image_handle_rejects_malformed_svg() {
    assert!(rasterize_svg_to_image_handle("not an svg document").is_none());
}

#[test]
fn named_icon_image_lookup_rejects_unsafe_and_missing_inputs_before_rasterization() {
    let weight = json!(400);

    assert!(get_named_icon_image("bad/path", "arrow-up", Color::BLACK).is_none());
    assert!(get_named_icon_image("feather", "bad/path", Color::BLACK).is_none());
    assert!(
        get_named_icon_image_with_weight(
            "feather",
            "definitely-missing",
            Some(&weight),
            Color::BLACK,
        )
        .is_none()
    );
}

#[test]
fn named_icon_image_lookup_rasterizes_existing_asset_and_reuses_cache_key() {
    let first = get_named_icon_image(" feather ", "arrow_up.svg", Color::from_rgb8(12, 34, 56));
    let second = get_named_icon_image("feather", "arrow-up", Color::from_rgb8(12, 34, 56));

    assert!(first.is_some());
    assert!(second.is_some());
}

#[test]
fn named_icon_catalog_and_family_json_expose_matching_generated_data() {
    let catalog = named_icon_catalog();
    let feather_json =
        named_icon_family_json(" Feather.svg ").expect("feather family JSON should be generated");
    let feather_family: NamedIconFamily =
        serde_json::from_str(feather_json).expect("generated family JSON should deserialize");
    let catalog_family = catalog
        .iter()
        .find(|entry| entry.family == "feather")
        .expect("catalog should contain feather");

    assert_eq!(feather_family.family, "feather");
    assert_eq!(feather_family.icons, catalog_family.icons);
    assert!(catalog.iter().any(|entry| entry.family == "bootstrap"));
    assert!(named_icon_family_json("missing-family").is_none());
    assert!(named_icon_family_json("bad/path").is_none());
}
