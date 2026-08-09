//! 提供系统设置消息处理共享的小型工具函数。

use vw_shared::provider::types::Info;

pub async fn load_gateway_providers() -> Result<std::collections::HashMap<String, Info>, String> {
    let response = crate::app::config::gateway_client()?.provider_list(None).await?;
    Ok(response
        .all
        .into_iter()
        .map(|provider| (provider.id.clone(), provider))
        .collect())
}

pub fn parse_comma_or_newline_list(input: &str) -> Vec<String> {
    input
        .split([',', '\n'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// 处理 `is_provider_connected` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub fn is_provider_connected(provider: &vw_shared::provider::types::Info) -> bool {
    let has_key = provider.key.as_deref().is_some_and(|key| !key.trim().is_empty());

    has_key
        || matches!(
            provider.source,
            vw_shared::provider::types::ProviderSource::Api
                | vw_shared::provider::types::ProviderSource::Env
        )
}

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

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
/// 处理 `sync_save` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回 `Err` 时保留原始错误文本，交由上层设置页展示。
pub fn sync_save(persist: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    persist()
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
/// 处理 `sync_save` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回 `Err` 时保留原始错误文本，交由上层设置页展示。
pub fn sync_save(_persist: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    Ok(())
}
#[cfg(test)]
#[path = "util_tests.rs"]
mod util_tests;
