// 验证应用状态满足跨异步任务共享所需的 Send 与 Sync 约束。
use super::*;

#[test]
fn app_state_type_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AppState>();
}
