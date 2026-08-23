//! 验证 Git 差异工具对结构化输出、补丁分组、统计和异常载荷的解析。
//! 直接数据优先于内容块，元数据行和畸形分块不得被误计为文件变更。

use super::git_diff_view::{
    GitDiffPreview, append_preview_gap, append_preview_line, git_diff_body, git_diff_title,
    is_git_diff_tool, parse_git_diff_previews, secondary_text, structured_git_diff_output,
    tool_card_style, tool_content_style, tool_git_diff_view, tool_item_button_style,
};
use crate::app::{App, Message};
use iced::Theme;
use serde_json::json;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn preview(path: &str, additions: usize, deletions: usize, cached: bool) -> GitDiffPreview {
    GitDiffPreview {
        path: path.to_string(),
        before: "before".to_string(),
        after: "after".to_string(),
        additions,
        deletions,
        cached,
    }
}

// 分段追加必须保留原有换行，否则后续统一差异解析会把相邻标记粘连。
#[test]
fn append_preview_helpers_preserve_line_boundaries() {
    let mut buf = String::new();
    append_preview_gap(&mut buf);
    assert_eq!(buf, "");

    append_preview_line(&mut buf, "first");
    append_preview_line(&mut buf, "second");
    append_preview_gap(&mut buf);
    append_preview_line(&mut buf, "last");
    assert_eq!(buf, "first\nsecond\n\nlast");

    append_preview_gap(&mut buf);
    append_preview_gap(&mut buf);
    append_preview_gap(&mut buf);

    assert_eq!(buf, "first\nsecond\n\nlast\n\n");
}

#[test]
fn git_operations_diff_is_treated_as_git_diff_tool() {
    assert!(is_git_diff_tool("git_diff", ""));
    assert!(is_git_diff_tool("git_operations", r#"{"operation":"diff"}"#));
    assert!(is_git_diff_tool("git_operations", "  {\"operation\":\"diff\"}  "));
    assert!(!is_git_diff_tool("git_operations", r#"{"operation":"status"}"#));
    assert!(!is_git_diff_tool("git_operations", r#"{"operation":42}"#));
    assert!(!is_git_diff_tool("git_operations", "not json"));
    assert!(!is_git_diff_tool("read", r#"{"operation":"diff"}"#));
}

#[test]
fn parse_git_diff_previews_returns_none_without_diff_data() {
    assert!(parse_git_diff_previews("", &json!({})).is_none());
}

#[test]
fn structured_git_diff_output_reads_data_or_json_content_blocks() {
    let direct = json!({"result":{"data":{"hunks":[]}}});
    assert_eq!(structured_git_diff_output(&direct), Some(json!({"hunks":[]})));

    let content = json!({
        "result": {
            "content": [
                {"type":"text","text":"ignored"},
                {"type":"json","value":{"hunks":[]}}
            ]
        }
    });
    assert_eq!(structured_git_diff_output(&content), Some(json!({"hunks":[]})));
}

#[test]
fn structured_git_diff_output_prefers_direct_data_and_filters_invalid_blocks() {
    let value = json!({
        "result": {
            "data": {"hunks":[{"file":"direct.rs"}]},
            "content": [
                {"type":"json","value":{"hunks":[{"file":"fallback.rs"}]}}
            ]
        }
    });
    assert_eq!(structured_git_diff_output(&value), Some(json!({"hunks":[{"file":"direct.rs"}]})));

    let fallback = json!({
        "result": {
            "data": {"message":"missing hunks"},
            "content": [
                {"type":"text","value":{"hunks":[]}},
                {"type":"json","value":{"message":"missing hunks"}},
                {"type":"json","value":{"hunks":[{"file":"fallback.rs"}]}}
            ]
        }
    });
    assert_eq!(
        structured_git_diff_output(&fallback),
        Some(json!({"hunks":[{"file":"fallback.rs"}]}))
    );

    assert!(structured_git_diff_output(&json!({"result":{"content":{}}})).is_none());
    assert!(
        structured_git_diff_output(&json!({
            "result":{"content":[{"type":"json","value":{"message":"no hunks"}}]}
        }))
        .is_none()
    );
}

#[test]
fn parse_git_diff_previews_groups_hunks_and_counts_changes() {
    let input = r#"{"cached":true}"#;
    let value = json!({
        "result": {
            "data": {
                "hunks": [
                    {
                        "file": "src/main.rs",
                        "header": "@@ -1 +1 @@",
                        "lines": [
                            {"text":"diff --git a/src/main.rs b/src/main.rs"},
                            {"text":"--- a/src/main.rs"},
                            {"text":"+++ b/src/main.rs"},
                            {"text":"-old"},
                            {"text":"+new"},
                            {"text":" context"}
                        ]
                    },
                    {
                        "file": "src/main.rs",
                        "lines": [{"text":"+again"}]
                    },
                    {
                        "file": "",
                        "lines": [{"text":"+skip"}]
                    }
                ]
            }
        }
    });

    let previews = parse_git_diff_previews(input, &value).expect("diff previews");

    assert_eq!(previews.len(), 1);
    assert_eq!(previews[0].path, "src/main.rs");
    assert_eq!(previews[0].additions, 2);
    assert_eq!(previews[0].deletions, 1);
    assert!(previews[0].cached);
    assert!(previews[0].before.contains("old"));
    assert!(previews[0].after.contains("again"));
}

#[test]
fn parse_git_diff_previews_accepts_output_json_string() {
    let value = json!({
        "output": "{\"hunks\":[{\"file\":\"src/lib.rs\",\"lines\":[{\"text\":\"+pub fn lib() {}\"}]}]}"
    });

    let previews = parse_git_diff_previews("{}", &value).expect("output json previews");

    assert_eq!(previews[0].path, "src/lib.rs");
    assert_eq!(previews[0].additions, 1);

    let fallback = json!({
        "output": "not json",
        "result": {
            "data": {
                "hunks": [{"file":"fallback.rs","lines":[{"text":"-old"}]}]
            }
        }
    });
    let previews = parse_git_diff_previews("{}", &fallback).expect("structured fallback");
    assert_eq!(previews[0].path, "fallback.rs");
    assert_eq!(previews[0].deletions, 1);
}

#[test]
fn parse_git_diff_previews_rejects_malformed_envelopes_and_empty_results() {
    assert!(parse_git_diff_previews("not json", &json!({"output":"{\"hunks\":[]}"})).is_none());
    assert!(parse_git_diff_previews("{}", &json!({"output":"not json"})).is_none());
    assert!(parse_git_diff_previews("{}", &json!({"output":"{\"hunks\":{}}"})).is_none());
    assert!(parse_git_diff_previews("{}", &json!({"output":"{\"hunks\":[]}"})).is_none());
}

#[test]
fn parse_git_diff_previews_filters_metadata_and_malformed_lines() {
    let value = json!({
        "result": {
            "data": {
                "hunks": [{
                    "file": "src/lib.rs",
                    "lines": [
                        {"text":"diff --git a/src/lib.rs b/src/lib.rs"},
                        {"text":"index 111..222 100644"},
                        {"text":"new file mode 100644"},
                        {"text":"deleted file mode 100644"},
                        {"text":"similarity index 90%"},
                        {"text":"rename from src/old.rs"},
                        {"text":"rename to src/lib.rs"},
                        {"text":"\\ No newline at end of file"},
                        {"text":"@@ -1 +1 @@"},
                        {"text":"--- a/src/lib.rs"},
                        {"text":"+++ b/src/lib.rs"},
                        {"text":" unchanged"},
                        {"text":"-old"},
                        {"text":"+new"},
                        {"text":42},
                        {}
                    ]
                }]
            }
        }
    });

    let previews = parse_git_diff_previews("{}", &value).expect("filtered preview");

    assert_eq!(previews.len(), 1);
    assert_eq!(previews[0].before, "unchanged\nold");
    assert_eq!(previews[0].after, "unchanged\nnew");
    assert_eq!(previews[0].additions, 1);
    assert_eq!(previews[0].deletions, 1);
    assert!(!previews[0].cached);
}

#[test]
fn parse_git_diff_previews_trims_paths_groups_hunks_and_inserts_gaps() {
    let value = json!({
        "result": {
            "data": {
                "hunks": [
                    {"file":"  src/main.rs  ","lines":[{"text":" first"}]},
                    {
                        "file":"src/main.rs",
                        "header":"@@ -2 +2 @@",
                        "lines":[{"text":" second"}]
                    }
                ]
            }
        }
    });

    let previews = parse_git_diff_previews("{}", &value).expect("grouped preview");

    assert_eq!(previews.len(), 1);
    assert_eq!(previews[0].path, "src/main.rs");
    assert_eq!(previews[0].before, "first\n\nsecond");
    assert_eq!(previews[0].after, "first\n\nsecond");
}

#[test]
fn parse_git_diff_previews_discards_invalid_and_metadata_only_hunks() {
    let value = json!({
        "result": {
            "data": {
                "hunks": [
                    {"lines":[{"text":"+missing path"}]},
                    {"file":"   ","lines":[{"text":"+blank path"}]},
                    {"file":"missing-lines.rs"},
                    {"file":"wrong-lines.rs","lines":{}},
                    {"file":"metadata.rs","lines":[{"text":"@@ -1 +1 @@"}]},
                    {"file":"kept.rs","lines":[{"text":"+kept"}]}
                ]
            }
        }
    });

    let previews = parse_git_diff_previews("{}", &value).expect("one meaningful preview");

    assert_eq!(previews.len(), 1);
    assert_eq!(previews[0].path, "kept.rs");
    assert_eq!(previews[0].after, "kept");
}

#[test]
fn parse_git_diff_previews_retains_empty_changed_lines_by_change_counts() {
    let value = json!({
        "result": {
            "data": {
                "hunks": [{
                    "file": "blank-lines.rs",
                    "lines": [
                        {"text":"-"},
                        {"text":"+"}
                    ]
                }]
            }
        }
    });

    let previews = parse_git_diff_previews("{}", &value).expect("empty changed lines preview");

    assert_eq!(previews.len(), 1);
    assert_eq!(previews[0].path, "blank-lines.rs");
    assert_eq!(previews[0].before, "");
    assert_eq!(previews[0].after, "");
    assert_eq!(previews[0].additions, 1);
    assert_eq!(previews[0].deletions, 1);
}

#[test]
fn git_diff_title_combines_counts_and_cached_marker() {
    assert_eq!(git_diff_title(&preview("src/lib.rs", 0, 0, false)), "src/lib.rs");
    assert_eq!(git_diff_title(&preview("src/lib.rs", 2, 3, false)), "src/lib.rs  +2-3");
    assert_eq!(git_diff_title(&preview("src/lib.rs", 0, 0, true)), "src/lib.rs  cached");
    assert_eq!(git_diff_title(&preview("src/lib.rs", 2, 3, true)), "src/lib.rs  +2-3  cached");
}

#[test]
fn git_diff_styles_cover_dark_light_and_state_variants() {
    let dark_normal = tool_card_style(&Theme::Dark, false, false, false);
    let dark_hovered = tool_card_style(&Theme::Dark, false, true, false);
    let dark_expanded = tool_card_style(&Theme::Dark, false, false, true);
    let dark_error = tool_card_style(&Theme::Dark, true, false, false);
    let light_normal = tool_card_style(&Theme::Light, false, false, false);
    let light_hovered = tool_card_style(&Theme::Light, false, true, false);
    let light_error = tool_card_style(&Theme::Light, true, false, false);

    assert!(dark_normal.background.is_none());
    assert_ne!(dark_normal.background, dark_hovered.background);
    assert_eq!(dark_hovered.background, dark_expanded.background);
    assert_ne!(dark_normal.background, dark_error.background);
    assert_ne!(light_normal.background, light_hovered.background);
    assert_ne!(light_normal.background, light_error.background);
    assert_ne!(dark_hovered.background, light_hovered.background);
    assert_eq!(dark_normal.border.width, 0.0);
    assert_eq!(dark_error.border.width, 0.0);
    assert_eq!(dark_normal.border.radius.top_left, 14.0);

    let dark_content = tool_content_style(&Theme::Dark, false);
    let dark_error_content = tool_content_style(&Theme::Dark, true);
    let light_content = tool_content_style(&Theme::Light, false);
    let light_error_content = tool_content_style(&Theme::Light, true);
    assert_ne!(dark_content.background, dark_error_content.background);
    assert_ne!(dark_content.background, light_content.background);
    assert_ne!(light_content.background, light_error_content.background);
    assert_eq!(dark_content.border.width, 0.0);
    assert_eq!(dark_content.border.radius.top_left, 12.0);
}

#[test]
fn git_diff_button_style_covers_active_hovered_and_pressed_states() {
    let active = tool_item_button_style(&Theme::Dark, iced::widget::button::Status::Active);
    let hovered = tool_item_button_style(&Theme::Dark, iced::widget::button::Status::Hovered);
    let pressed = tool_item_button_style(&Theme::Dark, iced::widget::button::Status::Pressed);
    let light_active = tool_item_button_style(&Theme::Light, iced::widget::button::Status::Active);
    let light_hovered =
        tool_item_button_style(&Theme::Light, iced::widget::button::Status::Hovered);
    let light_pressed =
        tool_item_button_style(&Theme::Light, iced::widget::button::Status::Pressed);

    assert_ne!(active.background, hovered.background);
    assert_ne!(hovered.background, pressed.background);
    assert_eq!(active.background, light_active.background);
    assert_ne!(light_active.background, light_hovered.background);
    assert_ne!(light_hovered.background, light_pressed.background);
    assert_ne!(hovered.background, light_hovered.background);
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.radius.top_left, 11.0);
    assert_eq!(active.text_color, Theme::Dark.palette().text);
}

#[test]
fn secondary_text_uses_theme_specific_palette_and_alpha() {
    assert_eq!(
        secondary_text(&Theme::Dark, 0.25, 0.75),
        Theme::Dark.palette().text.scale_alpha(0.25)
    );
    assert_eq!(
        secondary_text(&Theme::Light, 0.25, 0.75),
        Theme::Light.extended_palette().secondary.base.text.scale_alpha(0.75)
    );
}

#[test]
fn git_diff_body_rejects_empty_and_builds_collapsed_and_expanded_lists() {
    let app = test_app();
    assert!(git_diff_body(&app, &[], false).is_none());

    let previews = vec![
        preview("one.rs", 1, 0, false),
        preview("two.rs", 0, 1, true),
        preview("three.rs", 1, 1, false),
        preview("four.rs", 2, 2, false),
    ];
    keep_element(git_diff_body(&app, &previews, false).expect("collapsed diff body"));
    keep_element(git_diff_body(&app, &previews, true).expect("expanded diff body"));
}

#[test]
fn tool_git_diff_view_rejects_non_diff_or_empty_completed_output() {
    let app = test_app();

    assert!(tool_git_diff_view(&app, 0, 0, "tool git_diff").is_none());
    assert!(tool_git_diff_view(&app, 0, 0, "git_diff\n{}").is_none());
    assert!(tool_git_diff_view(&app, 0, 0, "tool git_diff\nnot json").is_none());
    assert!(tool_git_diff_view(&app, 0, 0, "tool read\n{}").is_none());
    assert!(
        tool_git_diff_view(
            &app,
            0,
            0,
            r#"tool git_diff
{"input":"{}","status":"completed"}"#
        )
        .is_none()
    );
}

#[test]
fn tool_git_diff_view_accepts_git_operations_diff_output() {
    let app = test_app();
    let payload = json!({
        "input": "{\"operation\":\"diff\"}",
        "status": "completed",
        "output": "{\"hunks\":[{\"file\":\"src/lib.rs\",\"lines\":[{\"text\":\"+new\"}]}]}"
    });
    let visible = format!("tool git_operations\n{payload}");

    keep_element(tool_git_diff_view(&app, 2, 3, &visible).expect("git operations diff view"));
}

#[test]
fn tool_git_diff_view_builds_hovered_multi_file_card() {
    let mut app = test_app();
    app.chat_tool_hovered_idx = Some((5_u64 << 32) | 6);
    let payload = json!({
        "input": "{}",
        "status": "completed",
        "result": {
            "data": {
                "hunks": [
                    {"file":"src/lib.rs","lines":[{"text":"+new lib"}]},
                    {"file":"src/main.rs","lines":[{"text":"-old main"}]}
                ]
            }
        }
    });
    let visible = format!("tool git_diff\n{payload}");

    keep_element(tool_git_diff_view(&app, 5, 6, &visible).expect("hovered multi-file diff"));
}

#[test]
fn tool_git_diff_view_builds_success_running_error_and_denied_cards() {
    let app = test_app();

    let success = tool_git_diff_view(&app, 1, 1, r#"tool git_diff
{"input":"{}","status":"completed","result":{"data":{"hunks":[{"file":"src/main.rs","lines":[{"text":"+new"}]}]}}}"#)
    .expect("success git diff view");
    keep_element(success);

    let running = tool_git_diff_view(
        &app,
        1,
        2,
        r#"tool git_diff
{"input":"{}","status":"running"}"#,
    )
    .expect("running git diff view");
    keep_element(running);

    let error = tool_git_diff_view(
        &app,
        1,
        3,
        r#"tool git_diff
{"input":"{}","status":"error","error":"diff failed"}"#,
    )
    .expect("error git diff view");
    keep_element(error);

    let denied = tool_git_diff_view(
        &app,
        1,
        4,
        r#"tool git_diff
{"input":"{}","status":"denied","error":"permission denied"}"#,
    )
    .expect("denied git diff view");
    keep_element(denied);
}
