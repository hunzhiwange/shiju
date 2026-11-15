//! 与 provider 解析相关的环境变量读取。
//!
//! 这些环境变量主要用于测试覆盖、自定义模型数据源以及控制运行时行为。

use once_cell::sync::Lazy;
use std::env;

/// 读取环境变量并转换为字符串。
fn env_string(key: &str) -> Option<String> {
    env::var_os(key).map(|v| v.to_string_lossy().to_string())
}

/// 判断环境变量是否表示布尔真值。
fn truthy(key: &str) -> bool {
    let Some(value) = env_string(key) else {
        return false;
    };
    let value = value.to_ascii_lowercase();
    value == "true" || value == "1"
}

/// 自定义 models 文件路径。
pub static VIBEWINDOW_MODELS_PATH: Lazy<Option<String>> =
    Lazy::new(|| env_string("VIBEWINDOW_MODELS_PATH"));

/// 禁止从远端刷新 models 元数据。
pub static VIBEWINDOW_DISABLE_MODELS_FETCH: Lazy<bool> =
    Lazy::new(|| truthy("VIBEWINDOW_DISABLE_MODELS_FETCH"));

/// 自定义 models.dev 源地址。
pub static VIBEWINDOW_MODELS_URL: Lazy<Option<String>> =
    Lazy::new(|| env_string("VIBEWINDOW_MODELS_URL"));

/// 返回当前客户端标识，用于拼接 User-Agent。
///
/// 若未显式指定，则默认视为 `cli`。
pub fn vibewindow_client() -> String {
    env_string("VIBEWINDOW_CLIENT").unwrap_or_else(|| "cli".to_string())
}

#[cfg(test)]
#[path = "flag_tests.rs"]
mod flag_tests;
