//! 验证 worktree 就绪与失败事件的线路类型名称保持稳定。
//! 这些字符串属于跨组件协议，重构内部常量时不能随意改名。

use super::event::{FAILED, READY};

#[test]
fn event_types_are_stable() {
    assert_eq!(READY.r#type, "worktree.ready");
    assert_eq!(FAILED.r#type, "worktree.failed");
}
