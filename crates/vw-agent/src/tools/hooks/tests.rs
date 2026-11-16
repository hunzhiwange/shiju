//! 验证工具调用前后钩子的透传、取消映射与权限决策兼容行为。
//! fixture 刻意返回取消结果，用于确认钩子拒绝不会被当作普通执行错误。

use super::*;
use crate::app::agent::hooks::HookHandler;
use async_trait::async_trait;
use serde_json::json;

struct CancellingHook;

#[async_trait]
impl HookHandler for CancellingHook {
    fn name(&self) -> &str {
        "cancelling-tool-hook"
    }

    async fn before_tool_call(&self, _name: String, _args: Value) -> HookResult<(String, Value)> {
        HookResult::Cancel("blocked by test hook".to_string())
    }
}

#[tokio::test]
async fn pre_tool_hook_maps_cancellation_to_denied_error() {
    let mut runner = crate::hooks::new_hook_runner();
    runner.register(Box::new(CancellingHook));

    let error = PreToolHook::run(Some(&runner), "shell".to_string(), json!({"command": "pwd"}))
        .await
        .unwrap_err();

    assert_eq!(error.message(), "Cancelled by hook: blocked by test hook");
    assert!(error.permission_request().is_none());
}
