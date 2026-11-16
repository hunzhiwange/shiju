//! 压缩状态内容投影；会话选择、请求生命周期及存储由宿主验证。
use crate::message::chat::stream::stream_events::{settle_compaction, upsert_compaction_status};
use crate::test_messages::message;
use session_ui_types::ChatRole;

const STARTED: &str = "正在压缩会话";
const COMPLETED: &str = "压缩完成";

#[test]
fn compaction_status_started_then_completed_updates_one_message() {
    let mut chat = Vec::new();
    let mut ids = Vec::new();
    let index = upsert_compaction_status(&mut chat, &mut ids, false, None, STARTED, COMPLETED);
    assert_eq!(chat.len(), 1);
    assert_eq!(chat[0].role, ChatRole::System);
    assert_eq!(chat[0].content, STARTED);

    let completed_index =
        upsert_compaction_status(&mut chat, &mut ids, true, Some(index), STARTED, COMPLETED);
    assert_eq!(completed_index, index);
    assert_eq!(chat.len(), 1);
    assert_eq!(chat[0].content, COMPLETED);
    assert_eq!(ids, vec![None]);
}

#[test]
fn compaction_error_and_cancel_settle_active_and_background_status() {
    // 前后台都向同一个领域入口提供消息切片；路由矩阵仍留在宿主。
    for content in ["会话压缩已取消", "会话压缩失败"] {
        let mut chat = vec![message(ChatRole::System, STARTED)];
        let mut pending = Some(0);
        assert_eq!(settle_compaction(&mut chat, &mut pending, STARTED, content), Some(0));
        assert!(chat.iter().all(|message| message.content != STARTED));
        assert!(chat.iter().any(|message| message.content == content));
        assert_eq!(pending, None);
    }
}
