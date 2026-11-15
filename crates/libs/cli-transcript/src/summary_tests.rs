//! 转录摘要到终端行列表的组装测试。
//! 覆盖空转录默认提示、流式草稿、进度与错误项过滤，以及可见项之间的留白。
//! 空助手消息必须保留带前缀的占位行，避免对话结构在视图中丢失。

use super::transcript_to_lines;
use super::{TranscriptEntry, TranscriptRole};
use std::collections::BTreeSet;

fn line_text(line: &ratatui::text::Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect::<String>()
}

#[test]
fn renders_default_line_and_draft_for_empty_transcript() {
    let (lines, think_map) = transcript_to_lines(&[], false, false, &BTreeSet::new(), "draft");

    assert_eq!(lines.len(), 2);
    assert_eq!(line_text(&lines[0]), "Ready. Type /help for commands.");
    assert_eq!(line_text(&lines[1]), "draft");
    assert_eq!(think_map, vec![None, None]);
}

#[test]
fn skips_progress_and_error_entries() {
    let transcript = vec![
        TranscriptEntry::new(TranscriptRole::Progress, "loading"),
        TranscriptEntry::new(TranscriptRole::Error, "boom"),
        TranscriptEntry::new(TranscriptRole::User, "hello"),
    ];

    let (lines, think_map) = transcript_to_lines(&transcript, false, false, &BTreeSet::new(), "");

    assert_eq!(lines.len(), 1);
    assert_eq!(line_text(&lines[0]), "> hello");
    assert_eq!(think_map, vec![None]);
}

#[test]
fn inserts_blank_line_between_visible_entries() {
    let transcript = vec![
        TranscriptEntry::new(TranscriptRole::System, "boot"),
        TranscriptEntry::new(TranscriptRole::User, "hello"),
    ];

    let (lines, _) = transcript_to_lines(&transcript, false, false, &BTreeSet::new(), "");

    assert_eq!(line_text(&lines[0]), "boot");
    assert_eq!(line_text(&lines[1]), "");
    assert_eq!(line_text(&lines[2]), "> hello");
}

#[test]
fn renders_prefixed_placeholder_for_empty_assistant_entry() {
    let transcript = vec![TranscriptEntry::new(TranscriptRole::Assistant, "")];

    let (lines, think_map) = transcript_to_lines(&transcript, false, false, &BTreeSet::new(), "");

    assert_eq!(lines.len(), 1);
    assert_eq!(line_text(&lines[0]), "● ");
    assert_eq!(think_map, vec![None]);
}
