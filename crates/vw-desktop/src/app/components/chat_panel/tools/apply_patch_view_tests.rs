use super::apply_patch_view::{tool_apply_patch_view, apply_patch_file_expand_key};
use crate::app::App;
use crate::app::components::chat_panel::utils::chat_context_target_key;

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
