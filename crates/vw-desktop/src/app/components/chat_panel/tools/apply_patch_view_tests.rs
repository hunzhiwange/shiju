use super::apply_patch_view::{
    apply_patch_change_totals, apply_patch_file_button_style, apply_patch_file_expand_key,
    apply_patch_header_file_icon, apply_patch_header_summary, apply_patch_inner_card_style,
    apply_patch_outer_card_style, apply_patch_secondary_text, diff_utils_looks_like_unified_diff,
    tool_apply_patch_view,
};
#[cfg(not(target_arch = "wasm32"))]
use super::apply_patch_view::{
    deleted_file_preview_from_history, deleted_file_preview_from_repo_diff,
};
use crate::app::App;
use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::chat_context_target_key;
use iced::widget::button;
use iced::{Color, Theme};

#[test]
fn apply_patch_header_summary_splits_first_path() {
    let files = vec![('M', "src/main.rs".to_string()), ('A', "README.md".to_string())];

    assert_eq!(
        apply_patch_header_summary(&files),
        ("main.rs".to_string(), "/src".to_string(), Some("等 2 个文件".to_string()))
    );
}

#[test]
fn apply_patch_header_summary_handles_empty_changes() {
    assert_eq!(apply_patch_header_summary(&[]), (String::new(), String::new(), None));
}

#[test]
fn apply_patch_header_summary_handles_single_and_absolute_paths() {
    assert_eq!(
        apply_patch_header_summary(&[('A', "README.md".to_string())]),
        ("README.md".to_string(), String::new(), None)
    );
    assert_eq!(
        apply_patch_header_summary(&[('D', "/workspace/src/lib.rs".to_string())]),
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
    let _ = apply_patch_change_totals(0, 0);
    let _ = apply_patch_change_totals(2, 1);
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

#[test]
fn apply_patch_view_rejects_malformed_envelopes_bad_tool_and_invalid_json() {
    let app = App::new().0;

    assert!(tool_apply_patch_view(&app, 0, 0, "").is_none());
    assert!(tool_apply_patch_view(&app, 0, 0, "tool apply_patch").is_none());
    assert!(tool_apply_patch_view(&app, 0, 0, "apply_patch\n{}").is_none());
    assert!(tool_apply_patch_view(&app, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_apply_patch_view(&app, 0, 0, "tool apply_patch\nnot-json").is_none());
}

#[test]
fn apply_patch_view_renders_running_error_denied_and_empty_completed_states() {
    let app = App::new().0;

    for visible in [
        r#"tool apply_patch
{"status":"running","input":"*** Begin Patch\n*** End Patch"}"#,
        r#"tool apply_patch
{"status":"error","error":"patch failed","input":"*** Begin Patch\n*** End Patch"}"#,
        r#"tool apply_patch
{"status":"denied","error":"denied by user","input":"*** Begin Patch\n*** End Patch"}"#,
        r#"tool apply_patch
{}"#,
    ] {
        assert!(tool_apply_patch_view(&app, 1, 1, visible).is_some());
    }
}

#[test]
fn apply_patch_view_wraps_content_when_context_menu_is_open() {
    let mut app = App::new().0;
    app.chat_context_menu_target = Some(chat_context_target_key(8, Some(9)));
    app.chat_context_menu_pos = Some((24.0, 48.0));

    assert!(tool_apply_patch_view(&app, 8, 9, "tool apply_patch\n{}").is_some());
}

#[test]
fn apply_patch_view_renders_unstructured_file_summary_and_diff_output() {
    let app = App::new().0;

    assert!(
        tool_apply_patch_view(
            &app,
            2,
            3,
            concat!(
                "tool apply_patch\n",
                "{\"input\":\"--- a/src/main.rs\\n+++ b/src/main.rs\\n@@ -1 +1 @@\\n-old\\n+new\",",
                "\"output\":\"M src/main.rs\\n\\n<diff>\\n--- a/src/main.rs\\n+++ b/src/main.rs\\n@@ -1 +1 @@\\n-old\\n+new\\n</diff>\",",
                "\"metadata\":{\"outputPath\":\"patch-output.txt\"}}"
            )
        )
        .is_some()
    );
}

#[test]
fn apply_patch_view_renders_change_summary_with_expanded_inline_diff() {
    let mut app = App::new().0;
    app.chat_tool_hovered_idx = Some((1_u64 << 32) | 1);
    app.chat_tool_file_expanded.insert("1:1:src/main.rs".to_string());

    assert!(
        tool_apply_patch_view(
            &app,
            1,
            1,
            concat!(
                "tool apply_patch\n",
                "{\"input\":\"*** Begin Patch\\n*** Update File: src/main.rs\\n-old\\n+new\\n*** End Patch\",",
                "\"output\":\"M src/main.rs +1 -1\\n\\n<changes>\\n",
                "{\\\"files\\\":[{\\\"path\\\":\\\"src/main.rs\\\",\\\"additions\\\":1,\\\"deletions\\\":1,\\\"before\\\":\\\"old\\\\n\\\",\\\"after\\\":\\\"new\\\\n\\\"}]}",
                "\\n</changes>\"}"
            )
        )
        .is_some()
    );
}

#[test]
fn apply_patch_view_uses_change_metadata_when_summary_lines_are_absent() {
    let app = App::new().0;

    assert!(
        tool_apply_patch_view(
            &app,
            6,
            7,
            concat!(
                "tool apply_patch\n",
                "{\"output\":\"<changes>\\n",
                "{\\\"files\\\":[{\\\"path\\\":\\\"src/deleted.rs\\\",\\\"additions\\\":0,\\\"deletions\\\":2,\\\"before\\\":\\\"one\\\\ntwo\\\\n\\\",\\\"after\\\":\\\"\\\"}]}",
                "\\n</changes>\"}"
            )
        )
        .is_some()
    );
}

#[test]
fn apply_patch_view_renders_structured_added_modified_and_deleted_files() {
    let mut app = App::new().0;
    for path in ["src/added.rs", "src/modified.rs", "src/deleted.rs"] {
        app.chat_tool_file_expanded.insert(apply_patch_file_expand_key(4, 5, path));
    }

    assert!(
        tool_apply_patch_view(
            &app,
            4,
            5,
            concat!(
                "tool apply_patch\n",
                "{\"status\":\"completed\",\"result\":{\"content\":[",
                "{\"type\":\"structured_patch\",\"hunks\":[",
                "{\"path\":\"src/added.rs\",\"header\":\"@@ -0,0 +1 @@\",\"lines\":[\"+added\"]},",
                "{\"path\":\"src/modified.rs\",\"header\":\"@@ -1 +1 @@\",\"lines\":[\"-old\",\"+new\"]},",
                "{\"path\":\"src/deleted.rs\",\"header\":\"@@ -1 +0,0 @@\",\"lines\":[\"-deleted\"]}",
                "]}]}}"
            )
        )
        .is_some()
    );
}

#[test]
fn apply_patch_view_renders_permission_target_without_file_summary() {
    let app = App::new().0;
    let visible = concat!(
        "tool apply_patch\n",
        r#"{"status":"denied","input":"*** Begin Patch\n*** End Patch","permission":{"state":"denied"}}"#,
    );

    assert!(tool_apply_patch_view(&app, 10, 11, visible).is_some());
}

#[test]
fn apply_patch_view_renders_expanded_rows_when_preview_body_is_missing() {
    let mut app = App::new().0;
    for path in ["src/missing.rs", "src/deleted.rs"] {
        app.chat_tool_file_expanded.insert(apply_patch_file_expand_key(12, 13, path));
    }
    let visible = concat!(
        "tool apply_patch\n",
        "{\"output\":\"<changes>\\n",
        "{\\\"files\\\":[",
        "{\\\"path\\\":\\\"src/missing.rs\\\",\\\"additions\\\":1,\\\"deletions\\\":0,\\\"before\\\":\\\"\\\",\\\"after\\\":\\\"\\\"},",
        "{\\\"path\\\":\\\"src/deleted.rs\\\",\\\"additions\\\":0,\\\"deletions\\\":1,\\\"before\\\":\\\"\\\",\\\"after\\\":\\\"\\\"}",
        "]}\\n</changes>\"}",
    );

    assert!(tool_apply_patch_view(&app, 12, 13, visible).is_some());
}

#[test]
fn apply_patch_view_uses_cached_editor_for_error_and_diff_bodies() {
    use super::tool_text_key;
    use iced::widget::text_editor;

    let mut app = App::new().0;
    app.chat_tool_text_editors
        .insert(tool_text_key(14, 15, 0), text_editor::Content::with_text("cached"));

    assert!(
        tool_apply_patch_view(
            &app,
            14,
            15,
            "tool apply_patch\n{\"status\":\"error\",\"error\":\"failed\"}",
        )
        .is_some()
    );
    assert!(
        tool_apply_patch_view(
            &app,
            14,
            15,
            concat!(
                "tool apply_patch\n",
                "{\"output\":\"<diff>\\n--- a/a.rs\\n+++ b/a.rs\\n@@ -1 +1 @@\\n-old\\n+new\\n</diff>\"}",
            ),
        )
        .is_some()
    );
}

#[test]
fn apply_patch_view_counts_apply_patch_input_when_output_has_no_totals() {
    let app = App::new().0;

    assert!(
        tool_apply_patch_view(
            &app,
            16,
            17,
            concat!(
                "tool apply_patch\n",
                "{\"status\":\"completed\",\"input\":\"*** Begin Patch\\n",
                "*** Add File: src/new.rs\\n+fn main() {}\\n",
                "*** Update File: src/lib.rs\\n@@\\n-old\\n+new\\n",
                "*** End Patch\\n\"}",
            ),
        )
        .is_some()
    );
}

#[test]
fn apply_patch_view_renders_output_path_without_inline_diff() {
    let app = App::new().0;

    assert!(
        tool_apply_patch_view(
            &app,
            18,
            19,
            "tool apply_patch\n{\"metadata\":{\"outputPath\":\"/tmp/patch-output.txt\"}}",
        )
        .is_some()
    );
}

#[test]
fn apply_patch_view_renders_structured_summary_without_expanded_preview() {
    let app = App::new().0;

    assert!(
        tool_apply_patch_view(
            &app,
            20,
            21,
            concat!(
                "tool apply_patch\n",
                "{\"status\":\"completed\",\"result\":{\"content\":[",
                "{\"type\":\"structured_patch\",\"hunks\":[",
                "{\"path\":\"src/added.rs\",\"header\":\"@@ -0,0 +1,2 @@\",\"lines\":[\"+one\",\"+two\"]},",
                "{\"path\":\"src/deleted.rs\",\"header\":\"@@ -1,2 +0,0 @@\",\"lines\":[\"-one\",\"-two\"]}",
                "]}]}}"
            ),
        )
        .is_some()
    );
}

#[test]
fn apply_patch_view_renders_deleted_summary_without_change_metadata() {
    let app = App::new().0;

    assert!(
        tool_apply_patch_view(
            &app,
            22,
            23,
            "tool apply_patch\n{\"output\":\"D src/removed.rs\\n\"}",
        )
        .is_some()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deleted_file_preview_helpers_reject_missing_project_root_without_io() {
    let app = App::new().0;

    assert!(deleted_file_preview_from_history(&app, "src/deleted.rs").is_none());
    assert!(deleted_file_preview_from_repo_diff(&app, "src/deleted.rs").is_none());
}
