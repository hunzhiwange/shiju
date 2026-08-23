//! 验证工作树请求的可选控制默认值及协议枚举名称。

use crate::worktree::{CreateWorktreeRequest, DeleteWorktreeRequest, ResetMode, WorktreeStatus};
use serde_json::json;

#[test]
fn worktree_requests_default_optional_controls() {
    let create: CreateWorktreeRequest = serde_json::from_value(json!({
        "name": "feature",
        "branch": "feature"
    }))
    .expect("valid create");
    assert_eq!(create.from_ref, None);
    assert!(!create.checkout);

    let delete: DeleteWorktreeRequest = serde_json::from_value(json!({})).expect("valid delete");
    assert!(!delete.force);

    assert_eq!(serde_json::to_value(ResetMode::Hard).expect("serialize"), json!("hard"));
    assert_eq!(serde_json::to_value(WorktreeStatus::Ready).expect("serialize"), json!("ready"));
}
