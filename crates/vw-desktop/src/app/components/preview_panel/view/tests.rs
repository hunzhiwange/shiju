use std::path::Path;

use super::{preview_breadcrumb_segments, project_relative_segments, relative_segments_from_root};

#[cfg(not(target_arch = "wasm32"))]
use super::{
    BadgeTone, active_preview_lsp_badge_colors, active_preview_lsp_badge_dot_color,
    active_preview_lsp_badge_state,
};
#[cfg(not(target_arch = "wasm32"))]
use iced::Theme;

#[test]
fn relative_segments_from_root_rejects_paths_outside_project() {
    let segments =
        relative_segments_from_root(Path::new("/tmp/other/file.rs"), Path::new("/tmp/project"));

    assert!(segments.is_none());
}

#[test]
fn preview_breadcrumb_segments_keep_file_name_without_project() {
    let segments = preview_breadcrumb_segments(Path::new("notes/today.md"), None);

    assert_eq!(segments, vec!["notes/today.md".to_string()]);
}

#[test]
fn relative_segments_from_root_rejects_project_root_itself() {
    let segments =
        relative_segments_from_root(Path::new("/tmp/project"), Path::new("/tmp/project"));

    assert!(segments.is_none());
}

#[test]
fn project_relative_segments_uses_canonical_paths_when_lexical_roots_differ() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let active_path = Path::new(manifest_dir).join("src/app");
    let project_path = format!("{manifest_dir}/src/..");

    let segments = project_relative_segments(&active_path, Some(project_path.as_str()));

    assert_eq!(segments, Some(vec!["src".to_string(), "app".to_string()]));
}

#[test]
fn preview_breadcrumb_segments_falls_back_for_project_root_itself() {
    let segments = preview_breadcrumb_segments(Path::new("/tmp/project"), Some("/tmp/project"));

    assert_eq!(segments, vec!["/tmp/project".to_string()]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_state_ignores_progress_without_server() {
    let (label, tone) = active_preview_lsp_badge_state(None, true, true);

    assert_eq!(label, "LSP");
    assert_eq!(tone, BadgeTone::Error);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_dot_color_maps_each_tone_to_semantic_palette() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    assert_eq!(
        active_preview_lsp_badge_dot_color(&theme, BadgeTone::Ready),
        palette.success.base.color
    );
    assert_eq!(
        active_preview_lsp_badge_dot_color(&theme, BadgeTone::Working),
        palette.warning.base.color
    );
    assert_eq!(
        active_preview_lsp_badge_dot_color(&theme, BadgeTone::Error),
        palette.danger.base.color
    );
    assert_eq!(
        active_preview_lsp_badge_dot_color(&theme, BadgeTone::Neutral),
        palette.background.strong.color
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_colors_use_warning_palette_for_working() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    assert_eq!(
        active_preview_lsp_badge_colors(&theme, BadgeTone::Working),
        (
            palette.warning.base.color.scale_alpha(0.12),
            palette.warning.base.color,
            palette.warning.base.color.scale_alpha(0.32),
        )
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_colors_use_danger_palette_for_error() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    assert_eq!(
        active_preview_lsp_badge_colors(&theme, BadgeTone::Error),
        (
            palette.danger.base.color.scale_alpha(0.10),
            palette.danger.base.color,
            palette.danger.base.color.scale_alpha(0.26),
        )
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_colors_use_background_palette_for_neutral() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    assert_eq!(
        active_preview_lsp_badge_colors(&theme, BadgeTone::Neutral),
        (
            palette.background.weak.color,
            palette.background.weak.text,
            palette.background.strong.color.scale_alpha(0.18),
        )
    );
}
