use super::*;
#[test]
fn apply_patch_header_summary_splits_first_path() {
    let files = vec![('M', "src/main.rs".to_string()), ('A', "README.md".to_string())];

    assert_eq!(
        apply_patch_header_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &files
        ),
        ("main.rs".to_string(), "/src".to_string(), Some("等 2 个文件".to_string()))
    );
}

#[test]
fn apply_patch_header_summary_handles_empty_changes() {
    assert_eq!(
        apply_patch_header_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &[]
        ),
        (String::new(), String::new(), None)
    );
}

#[test]
fn apply_patch_header_summary_handles_single_and_absolute_paths() {
    assert_eq!(
        apply_patch_header_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &[('A', "README.md".to_string())]
        ),
        ("README.md".to_string(), String::new(), None)
    );
    assert_eq!(
        apply_patch_header_summary(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &[('D', "/workspace/src/lib.rs".to_string())]
        ),
        ("lib.rs".to_string(), "/workspace/src".to_string(), None)
    );
}

#[test]
fn unified_diff_detection_requires_all_markers() {
    assert!(diff_utils_looks_like_unified_diff(
        "  \n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-old\n+new\n"
    ));

    for invalid in [
        "",
        "   ",
        "+++ b/a.rs\n@@ -1 +1 @@",
        "--- a/a.rs\n@@ -1 +1 @@",
        "--- a/a.rs\n+++ b/a.rs",
        "prefix\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@",
    ] {
        assert!(!diff_utils_looks_like_unified_diff(invalid), "accepted {invalid:?}");
    }
}

#[test]
fn apply_patch_change_totals_builds_zero_and_nonzero_widgets() {
    let _ = apply_patch_change_totals::<()>( 0, 0);
    let _ = apply_patch_change_totals::<()>( 2, 1);
}

#[test]
fn apply_patch_header_icon_maps_supported_extensions_case_insensitively() {
    let cases = [
        ("lib.RS", Icon::Rust),
        ("component.ts", Icon::Typescript),
        ("component.TSX", Icon::Typescript),
        ("index.js", Icon::Javascript),
        ("index.JSX", Icon::Javascript),
        ("data.json", Icon::Json),
        ("Cargo.toml", Icon::Toml),
        ("workflow.yaml", Icon::Yaml),
        ("workflow.YML", Icon::Yaml),
        ("README.md", Icon::Markdown),
        ("index.html", Icon::Html),
        ("legacy.HTM", Icon::Html),
        ("styles.css", Icon::Css),
        ("script.py", Icon::Python),
        ("main.go", Icon::Go),
        ("setup.sh", Icon::Console),
        ("LICENSE", Icon::Document),
    ];

    for (path, expected) in cases {
        assert_eq!(apply_patch_header_file_icon(path), expected, "path {path}");
    }
}

#[test]
fn apply_patch_expand_key_preserves_indices_and_path() {
    assert_eq!(apply_patch_file_expand_key(12, 34, "src/nested:a.rs"), "12:34:src/nested:a.rs");
}

#[test]
fn apply_patch_secondary_text_uses_theme_specific_palette_and_alpha() {
    assert_eq!(
        apply_patch_secondary_text(&Theme::Dark, 0.42, 0.73),
        Theme::Dark.palette().text.scale_alpha(0.42)
    );
    assert_eq!(
        apply_patch_secondary_text(&Theme::Light, 0.42, 0.73),
        Theme::Light.extended_palette().secondary.base.text.scale_alpha(0.73)
    );
}

#[test]
fn apply_patch_card_and_button_styles_remain_transparent() {
    for style in
        [apply_patch_outer_card_style(&Theme::Dark), apply_patch_inner_card_style(&Theme::Light)]
    {
        assert!(style.background.is_none());
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.color, Color::TRANSPARENT);
    }

    for status in [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
        button::Status::Disabled,
    ] {
        let style = apply_patch_file_button_style(&Theme::Dark, status);
        assert!(style.background.is_none());
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.color, Color::TRANSPARENT);
        assert_eq!(style.text_color, Theme::Dark.palette().text);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deleted_file_preview_helpers_reject_missing_project_root_without_io() {

    assert!(deleted_file_preview_from_history(None, "src/deleted.rs").is_none());
    assert!(deleted_file_preview_from_repo_diff(None, "src/deleted.rs").is_none());
}
