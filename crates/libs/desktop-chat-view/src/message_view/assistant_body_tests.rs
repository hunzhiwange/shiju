use super::ParsedChatBlock;
use super::assistant_body::deduped_tool_last_indices;
use super::permission_card::should_highlight_pending_permission_tool;

#[test]
fn pending_permission_highlight_matches_active_request_or_direct_target() {
    assert!(should_highlight_pending_permission_tool(None, None, true));
    assert!(should_highlight_pending_permission_tool(Some("req-1"), Some("req-1"), false));
    assert!(!should_highlight_pending_permission_tool(Some("req-1"), Some("req-2"), false));
    assert!(!should_highlight_pending_permission_tool(Some("req-1"), None, false));
    assert!(!should_highlight_pending_permission_tool(None, None, false));
}

#[test]
fn deduped_tool_last_indices_ignores_invalid_and_explore_blocks() {
    let blocks = vec![
        ParsedChatBlock::Text { content: "before".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool read\n{\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"completed\"}"
                .to_string(),
        },
        ParsedChatBlock::Tool { raw: "not a tool".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool bash\n{\"input\":\"echo first\",\"status\":\"completed\"}".to_string(),
        },
        ParsedChatBlock::Tool {
            raw: "tool shell\n{\"input\":\"echo first\",\"status\":\"completed\"}".to_string(),
        },
    ];

    let last = deduped_tool_last_indices(&blocks);

    assert_eq!(last.len(), 1);
    assert_eq!(last.get("bash:echo first"), Some(&4));
}
