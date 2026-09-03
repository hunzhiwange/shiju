//! 会话短标识生成与默认标题识别测试。
//!
//! 覆盖 Base62 映射、随机源失败时的时间戳回退和标题格式边界。

#[test]
fn generated_slug_has_expected_shape() {
    let slug = super::create_slug();

    assert_eq!(slug.len(), 8);
    assert!(slug.bytes().all(|b| b.is_ascii_alphanumeric()));
}

#[test]
fn slug_from_random_bytes_maps_each_byte_to_base62_character() {
    let slug = super::slug_from_random_result(Ok([0, 1, 9, 10, 35, 36, 61, 62]));

    assert_eq!(slug, "019AZaz0");
}

#[test]
fn slug_from_random_error_falls_back_to_hex_timestamp() {
    let slug = super::slug_from_random_result(Err(()));

    assert!(!slug.is_empty());
    assert!(slug.bytes().all(|b| b.is_ascii_hexdigit()));
}

#[test]
fn default_title_matches_only_generated_titles() {
    assert!(super::is_default_title("New session - 2026-05-24T12:34:56.789Z"));
    assert!(super::is_default_title("Child session - 2026-05-24T12:34:56.789Z"));
    assert!(!super::is_default_title("New session - draft"));
    assert!(!super::is_default_title("New session - 2026-05-24T12:34:56Z"));
}

#[test]
fn task_session_source_marker_round_trip_preserves_content() {
    let content = "  第一行\n<task_session_source> 是用户正文\n最后一行  ";
    let encoded = super::encode_task_session_source_marker("session-123", content)
        .expect("non-empty source id should encode");

    let source = super::parse_task_session_source_marker(&encoded)
        .expect("encoded marker should parse");
    assert_eq!(source.source_session_id, "session-123");
    assert_eq!(source.content, content);
    assert_eq!(super::strip_task_session_source_marker(&encoded), content);
}

#[test]
fn task_session_source_marker_rejects_malformed_blocks() {
    let malformed = [
        "<task_session_source>\nsource_session_id: session-123\n由 ChatGPT 从另一项任务发起\n正文",
        "<task_session_source>\nsource_session_id: \n由 ChatGPT 从另一项任务发起\n</task_session_source>\n正文",
        "前缀<task_session_source>\nsource_session_id: session-123\n由 ChatGPT 从另一项任务发起\n</task_session_source>\n正文",
    ];

    for raw in malformed {
        assert_eq!(super::parse_task_session_source_marker(raw), None);
        assert_eq!(super::strip_task_session_source_marker(raw), raw);
    }
    assert_eq!(super::encode_task_session_source_marker("", "正文"), None);
}
