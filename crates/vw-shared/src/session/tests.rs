//! 网关客户端测试模块，覆盖端点拼接、SSE 分帧和流式事件归一化行为。

use super::ui_store::{load_session_scoped, save_session_scoped};
use super::ui_types::{ChatMessage, ChatRole, ChatSession};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(not(target_arch = "wasm32"))]
fn test_data_dir(test_name: &str) -> PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    std::env::temp_dir().join(format!("vw-shared-{test_name}-{}-{unique}", std::process::id()))
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn save_and_load_session_preserves_tool_role() {
    let data_dir = test_data_dir("tool-role");
    std::fs::create_dir_all(&data_dir).expect("should create temp data dir");

    let session = ChatSession {
        id: "session-tool".to_string(),
        title: "tool role".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::Tool,
            content: "tool shell\npwd\n/Users/demo\n".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some("msg-tool".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };

    let saved = save_session_scoped(&data_dir, &session, None);
    assert!(saved.is_some(), "session should be persisted");

    let loaded = load_session_scoped(&data_dir, &session.id, None).expect("session should load");
    assert_eq!(loaded.messages.len(), 1);
    assert_eq!(loaded.messages[0].role, ChatRole::Tool);
    assert_eq!(loaded.messages[0].content, session.messages[0].content);
    assert_eq!(loaded.message_ids, session.message_ids);

    let _ = std::fs::remove_dir_all(&data_dir);
}
