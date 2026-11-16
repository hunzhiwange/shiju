use super::card;
use crate::card_style::chat_context_target_key;
use crate::tools::apply_patch_view::apply_patch_file_expand_key;
use crate::tools::view_fixture::TestApp as App;
fn tool_apply_patch_view<'a>(
    app: &'a App,
    m: usize,
    t: usize,
    raw: &str,
) -> Option<iced::Element<'a, crate::card_input::Message<()>>> {
    crate::tools::apply_patch_view::tool_apply_patch_view(
        &card(app),
        m,
        t,
        raw,
        crate::tools::view_fixture::diff,
    )
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
    app.chat_context_menu.target = Some(chat_context_target_key(8, Some(9)));
    app.chat_context_menu.position = Some((24.0, 48.0));

    assert!(tool_apply_patch_view(&app, 8, 9, "tool apply_patch\n{}").is_some());
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
