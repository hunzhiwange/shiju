//! 验证消息文本估高、分段、会话控制解析与长内容回退策略。
//! fixture 包含 UTF-8、中文标点、空段和超限文本，用于锁定字符与字节边界的既有语义。

use super::text::{
    SessionControlSelection, estimate_message_height_rough, estimate_text_height,
    message_editor_body, message_text_body, session_control_selection_card,
    should_prefer_plain_think_body, should_segment_text_block, split_session_control_selection,
    split_text_segments, think_text_body,
};
use crate::app::App;
use iced::Color;
use iced::widget::text_editor;

#[test]
fn estimate_text_height_grows_with_lines() {
    assert!(estimate_text_height("one\ntwo\nthree") > estimate_text_height("one"));
}

#[test]
fn estimate_text_height_uses_exact_wrapping_and_empty_minimum() {
    assert_eq!(estimate_text_height(""), 24.0);
    assert_eq!(estimate_text_height(&"x".repeat(72)), 24.0);
    assert_eq!(estimate_text_height(&"x".repeat(73)), 48.0);
    assert_eq!(estimate_text_height("one\ntwo\nthree"), 72.0);
}

#[test]
fn estimate_text_height_wraps_utf8_content_by_stored_byte_length() {
    assert_eq!("界".len(), 3);
    assert_eq!(estimate_text_height(&"界".repeat(24)), 24.0);
    assert_eq!(estimate_text_height(&"界".repeat(25)), 48.0);
}

#[test]
fn should_segment_text_block_only_for_large_text() {
    assert!(!should_segment_text_block("short"));
    assert!(should_segment_text_block(&"line\n".repeat(120)));
}

#[test]
fn should_segment_text_block_respects_char_and_line_boundaries() {
    assert!(!should_segment_text_block(&"x".repeat(4_000)));
    assert!(should_segment_text_block(&"x".repeat(4_001)));
    assert!(!should_segment_text_block(&vec!["x"; 96].join("\n")));
    assert!(should_segment_text_block(&vec!["x"; 97].join("\n")));
}

#[test]
fn split_text_segments_preserves_content_while_respecting_line_and_size_limits() {
    assert!(split_text_segments("").is_empty());

    let lines = (0..99).map(|i| format!("line-{i}")).collect::<Vec<_>>().join("\n");
    let line_segments = split_text_segments(&lines);
    assert_eq!(line_segments.len(), 2);
    assert_eq!(line_segments[0].lines().count(), 96);
    assert_eq!(line_segments[1].lines().count(), 3);

    let long_line = "x".repeat(3_999);
    let sized_segments = split_text_segments(&format!("{long_line}\ntail"));
    assert_eq!(sized_segments, vec![long_line, "tail".to_string()]);
}

#[test]
fn plain_think_body_is_preferred_for_streaming_or_thinking() {
    assert!(should_prefer_plain_think_body(true, true));
    assert!(should_prefer_plain_think_body(false, true));
    assert!(should_prefer_plain_think_body(true, false));
    assert!(!should_prefer_plain_think_body(false, false));
}

#[test]
fn estimate_message_height_rough_has_minimum() {
    assert!(estimate_message_height_rough("") > 0.0);
}

#[test]
fn split_session_control_selection_extracts_context_card_data() {
    let raw = "修一下这里\n\n<session_control_selection>\n用户在会话控制中选中了以下上下文。选中不代表已经执行；请在本轮需要时再主动使用。\n工具：file_write, ls\n技能：writing-plans, shiju\n识矩功能：skill-4\n</session_control_selection>";

    let (text, selection) = split_session_control_selection(raw);
    let selection = selection.expect("selection context should parse");

    assert_eq!(text, "修一下这里");
    assert_eq!(selection.tools, vec!["file_write", "ls"]);
    assert_eq!(selection.skills, vec!["writing-plans", "shiju"]);
    assert_eq!(selection.shiju_skills, vec!["skill-4"]);
}

#[test]
fn split_session_control_selection_supports_ascii_colons_and_chinese_commas() {
    let raw = "正文\n<session_control_selection>\n工具: read，grep\n技能: rust, testing\n</session_control_selection>\n";

    let (text, selection) = split_session_control_selection(raw);
    let selection = selection.expect("selection context should parse");

    assert_eq!(text, "正文");
    assert_eq!(selection.tools, vec!["read", "grep"]);
    assert_eq!(selection.skills, vec!["rust", "testing"]);
    assert!(selection.shiju_skills.is_empty());
}

#[test]
fn split_session_control_selection_preserves_text_without_marker() {
    let raw = "保留原始正文，包括前后空格 ";

    assert_eq!(split_session_control_selection(raw), (raw.to_string(), None));
}

#[test]
fn split_session_control_selection_filters_empty_items_and_ignores_unknown_lines() {
    let raw = "正文\n<session_control_selection>\n未知：ignored\n工具： read, , write，，\n技能： , testing,，\n</session_control_selection>";

    let (text, selection) = split_session_control_selection(raw);

    assert_eq!(text, "正文");
    assert_eq!(
        selection,
        Some(SessionControlSelection {
            tools: vec!["read".to_string(), "write".to_string()],
            skills: vec!["testing".to_string()],
            shiju_skills: Vec::new(),
        })
    );
}

#[test]
fn split_session_control_selection_ignores_incomplete_or_empty_blocks() {
    assert_eq!(split_session_control_selection("a <session_control_selection> 工具: read").1, None);

    let (text, selection) = split_session_control_selection(
        "a <session_control_selection>\n说明\n</session_control_selection> b",
    );
    assert_eq!(text, "a  b");
    assert_eq!(selection, None);
}

#[test]
fn estimate_message_height_accounts_for_tools_and_thinks_but_clamps() {
    let plain = estimate_message_height_rough("hello");
    let rich = estimate_message_height_rough("<think>x</think>\ntool bash\n{}\nhello");
    let huge = estimate_message_height_rough(&"line\n".repeat(10_000));

    assert!(rich > plain);
    assert_eq!(huge, 720.0);
}

#[test]
fn estimate_message_height_rough_counts_wrapping_and_each_bonus() {
    assert_eq!(estimate_message_height_rough("hello"), 78.0);
    assert_eq!(estimate_message_height_rough(&"x".repeat(97)), 100.0);
    assert_eq!(estimate_message_height_rough("<think>x"), 106.0);
    assert_eq!(estimate_message_height_rough("tool bash"), 114.0);
    assert_eq!(estimate_message_height_rough("<think>x tool bash"), 142.0);
}

#[test]
fn message_editor_body_requires_matching_editor_content() {
    let mut app = App::new().0;

    assert!(message_editor_body(&app, 0, Color::WHITE).is_none());

    app.chat_message_editors.push(text_editor::Content::with_text("message"));
    assert!(message_editor_body(&app, 0, Color::WHITE).is_some());
    assert!(message_editor_body(&app, 1, Color::WHITE).is_none());
}

#[test]
fn think_text_body_constructs_editor_and_plain_text_fallbacks() {
    let mut app = App::new().0;
    let msg_idx = 2;
    let think_idx = 3;
    let scroll_key = ((msg_idx as u64) << 32) | think_idx as u64;
    app.chat_think_editors
        .insert(scroll_key, text_editor::Content::with_text("selectable think text"));

    let _editor =
        think_text_body(&app, msg_idx, think_idx, "selectable think text".to_string(), false);
    let _preferred_plain =
        think_text_body(&app, msg_idx, think_idx, "selectable think text".to_string(), true);
    let _missing_editor =
        think_text_body(&app, msg_idx, think_idx + 1, "fallback".to_string(), false);
}

#[test]
fn think_text_body_constructs_preview_and_segmented_fallbacks() {
    let mut app = App::new().0;
    let preview = "x".repeat(2_001);
    app.chat_think_editors.insert(0, text_editor::Content::with_text(&preview));

    let _preview = think_text_body(&app, 0, 0, preview, false);
    let _segmented = think_text_body(&app, 1, 0, "line\n".repeat(120), true);
}

#[test]
fn text_and_session_cards_construct_for_segmented_and_empty_inputs() {
    let _ = message_text_body("line\n".repeat(120), false);
    let _ = message_text_body(String::new(), true);
    let _ = session_control_selection_card(SessionControlSelection {
        tools: vec!["read".to_string()],
        skills: vec!["testing".to_string()],
        shiju_skills: vec!["skill-4".to_string()],
    });
    let _ = session_control_selection_card(SessionControlSelection {
        tools: Vec::new(),
        skills: Vec::new(),
        shiju_skills: Vec::new(),
    });
}
