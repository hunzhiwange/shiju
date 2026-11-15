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
pub use crate::models::is_provider_connected;

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
