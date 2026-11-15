//! 验证 CLI 转录渲染的回归行为。
//! 测试覆盖消息片段、工具卡片和文本边界，确保终端输出保持可读。

use super::{
    ACCENT_CYAN, ACCENT_RED, SUCCESS, TEXT_SUBTLE, ThinkBlockMeta, TranscriptEntry, TranscriptRole,
    WARNING, build_streaming_transcript_view, default_empty_transcript_line, think_block_expanded,
    transcript_prefix_style, transcript_to_lines, wrap_trim_disabled,
};
use ratatui::style::{Modifier, Style};
use std::collections::BTreeSet;

fn first_think_meta(think_map: &[Option<ThinkBlockMeta>]) -> ThinkBlockMeta {
    think_map.iter().find_map(|meta| *meta).expect("expected at least one think block")
}

#[test]
fn think_block_expansion_respects_global_default_and_per_block_override() {
    let open = ThinkBlockMeta { id: 7, open: true };
    let closed = ThinkBlockMeta { id: 7, open: false };
    let no_overrides = BTreeSet::new();
    let overrides = BTreeSet::from([7]);
    let unrelated_override = BTreeSet::from([8]);

    assert!(think_block_expanded(open, false, &no_overrides));
    assert!(!think_block_expanded(closed, false, &no_overrides));
    assert!(!think_block_expanded(open, false, &overrides));
    assert!(think_block_expanded(closed, false, &overrides));
    assert!(think_block_expanded(open, false, &unrelated_override));
    assert!(think_block_expanded(closed, true, &overrides));
}

#[test]
fn transcript_entry_new_preserves_role_and_converts_text() {
    let entry = TranscriptEntry::new(TranscriptRole::System, String::from("booting"));

    assert!(matches!(entry.role, TranscriptRole::System));
    assert_eq!(entry.text, "booting");
}

#[test]
fn wrap_configuration_preserves_surrounding_whitespace() {
    assert!(!wrap_trim_disabled().trim);
}

#[test]
fn transcript_roles_and_empty_prompt_use_expected_prefix_styles() {
    let cases = [
        (TranscriptRole::System, "", TEXT_SUBTLE),
        (TranscriptRole::User, "> ", SUCCESS),
        (TranscriptRole::Assistant, "● ", ACCENT_CYAN),
        (TranscriptRole::Progress, "", WARNING),
        (TranscriptRole::Error, "", ACCENT_RED),
    ];

    for (role, expected_prefix, expected_color) in cases {
        let (prefix, color, style) = transcript_prefix_style(role);

        assert_eq!(prefix, expected_prefix);
        assert_eq!(color, expected_color);
        assert_eq!(style, Style::default().fg(expected_color).add_modifier(Modifier::BOLD));
    }

    let empty_line = default_empty_transcript_line();
    assert_eq!(empty_line.spans.len(), 1);
    assert_eq!(empty_line.spans[0].content.as_ref(), "Ready. Type /help for commands.");
    assert_eq!(empty_line.spans[0].style, Style::default().fg(TEXT_SUBTLE));
}

#[test]
fn open_think_block_defaults_expanded_and_can_collapse() {
    let transcript = vec![TranscriptEntry::new(TranscriptRole::Assistant, "<think>\nalpha")];

    let (expanded_lines, think_map) =
        transcript_to_lines(&transcript, false, false, &BTreeSet::new(), "");
    assert_eq!(expanded_lines.len(), 3);

    let think_meta = first_think_meta(&think_map);
    let overrides = BTreeSet::from([think_meta.id]);
    let (collapsed_lines, _) = transcript_to_lines(&transcript, false, false, &overrides, "");
    assert_eq!(collapsed_lines.len(), 2);
}

#[test]
fn closed_think_block_defaults_collapsed_and_can_expand() {
    let transcript =
        vec![TranscriptEntry::new(TranscriptRole::Assistant, "<think>\nalpha\n</think>")];

    let (collapsed_lines, think_map) =
        transcript_to_lines(&transcript, false, false, &BTreeSet::new(), "");
    assert_eq!(collapsed_lines.len(), 2);

    let think_meta = first_think_meta(&think_map);
    let overrides = BTreeSet::from([think_meta.id]);
    let (expanded_lines, _) = transcript_to_lines(&transcript, false, false, &overrides, "");
    assert_eq!(expanded_lines.len(), 3);
}

#[test]
fn streaming_think_block_id_stays_stable_across_rebuilds() {
    let transcript = vec![TranscriptEntry::new(TranscriptRole::User, "hi")];
    let draft = "<think>\nalpha";

    let (streaming_view_a, remaining_a) =
        build_streaming_transcript_view(&transcript, draft, false);
    assert_eq!(remaining_a, "");
    let (_, think_map_a) =
        transcript_to_lines(&streaming_view_a, false, false, &BTreeSet::new(), "");
    let think_id_a = first_think_meta(&think_map_a).id;

    let (streaming_view_b, remaining_b) =
        build_streaming_transcript_view(&transcript, draft, false);
    assert_eq!(remaining_b, "");
    let (_, think_map_b) =
        transcript_to_lines(&streaming_view_b, false, false, &BTreeSet::new(), "");
    let think_id_b = first_think_meta(&think_map_b).id;

    assert_eq!(think_id_a, think_id_b);
}
