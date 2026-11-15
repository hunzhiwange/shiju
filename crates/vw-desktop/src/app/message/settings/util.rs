//! 提供系统设置消息处理共享的小型工具函数。

use model_types::Info;

pub async fn load_gateway_providers() -> Result<std::collections::HashMap<String, Info>, String> {
    desktop_settings_providers::provider_tasks::load_gateway_providers(
        crate::app::config::gateway_client()?.endpoint().clone(),
    ).await
}

pub use desktop_settings_providers::settings_util::{parse_comma_or_newline_list, is_provider_connected, sync_save};

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
/// 处理 `wasm_save_task` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub fn wasm_save_task(
    tag: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + 'static,
) -> iced::Task<crate::app::Message> {
    crate::app::config::spawn_gateway_task(tag, future)
}
