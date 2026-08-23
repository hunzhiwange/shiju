//! 结构化流式输出识别与转录拼接的测试。
//! 确认思考或工具标记经过空白归一化后仍能被识别，纯文本草稿则不伪造转录项。
//! 只有结构化草稿才以助手身份追加，并正确返回剩余草稿状态。

use super::{TranscriptEntry, TranscriptRole};
use super::{build_streaming_transcript_view, structured_stream::should_render_structured_stream};

#[test]
fn detects_structured_stream_markers_after_trimming() {
    assert!(should_render_structured_stream("  <think>\nreasoning"));
    assert!(should_render_structured_stream("plain\ntool read"));
    assert!(!should_render_structured_stream("   \n  "));
    assert!(!should_render_structured_stream("assistant reply"));
}

#[test]
fn preserves_plain_draft_without_adding_transcript_entry() {
    let transcript = vec![TranscriptEntry::new(TranscriptRole::User, "hello")];

    let (view, remaining) = build_streaming_transcript_view(&transcript, "plain text", false);

    assert_eq!(view.len(), 1);
    assert_eq!(remaining, "plain text");
    assert!(matches!(view[0].role, TranscriptRole::User));
}

#[test]
fn appends_assistant_entry_for_structured_stream() {
    let transcript = vec![TranscriptEntry::new(TranscriptRole::User, "hello")];

    let (view, remaining) =
        build_streaming_transcript_view(&transcript, "<think>\nreasoning", true);

    assert_eq!(view.len(), 2);
    assert!(remaining.is_empty());
    assert!(matches!(view[1].role, TranscriptRole::Assistant));
    assert_eq!(view[1].text, "<think>\nreasoning");
}
