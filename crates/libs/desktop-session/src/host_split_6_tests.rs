//! 工具流文本投影；宿主保留请求路由、定时刷新和消息 ID 接线。
use crate::message::chat::stream::{
    stream_events::{
        append_stream_delta_to_tail, prepare_buffered_tool_line, queue_delta,
        upsert_workflow_tool_to_tail,
    },
    tail::StreamTailState,
};
use crate::test_messages::message;
use session_ui_types::ChatRole;

#[test]
fn buffered_tool_event_starts_on_its_own_line() {
    let mut chat = vec![message(ChatRole::User, "question")];
    let mut ids = vec![None];
    let mut tail = StreamTailState::default();
    append_stream_delta_to_tail(&mut chat, &mut ids, "Checking".into(), 1, &mut tail);
    let mut pending = String::new();
    let mut scheduled = false;
    let _ = queue_delta(&tail, &mut pending, &mut scheduled, true, " the repo.".into());
    prepare_buffered_tool_line(&mut pending);
    let _ = queue_delta(&tail, &mut pending, &mut scheduled, true, "tool execute\n{}\n".into());
    let _ = queue_delta(&tail, &mut pending, &mut scheduled, true, "Done.".into());
    append_stream_delta_to_tail(&mut chat, &mut ids, pending, 2, &mut tail);

    assert_eq!(chat.last().unwrap().content, "Checking the repo.\ntool execute\n{}\nDone.");
}

#[test]
fn active_workflow_update_appends_tool_block_and_preserves_message_id() {
    let mut chat = vec![message(ChatRole::Assistant, "answer")];
    let mut ids = vec![Some("assistant-id".to_string())];
    let changed_idx = upsert_workflow_tool_to_tail(
        &mut chat,
        &mut ids,
        "tool workflow_node\n{\"tool_call_id\":\"node-1\",\"status\":\"running\"}\n".into(),
    );

    assert_eq!(changed_idx, 0);
    assert_eq!(chat.len(), 1);
    assert!(chat[0].content.starts_with("answer\ntool workflow_node\n"));
    assert_eq!(ids, vec![Some("assistant-id".to_string())]);
}
