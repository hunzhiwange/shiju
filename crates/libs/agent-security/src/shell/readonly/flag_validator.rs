//! readonly 命令白名单的参数标志匹配逻辑。
//!
//! 该模块集中处理安全/不安全标志的匹配规则，覆盖 `--flag=value` 和短标志聚合等
//! 常见 shell 写法，避免 readonly 检查主体重复实现这些边界细节。

use super::allowlist::CommandAllowlistEntry;

/// 判断实参是否匹配白名单条目中声明的安全标志。
///
/// 参数：
/// - `arg`：当前命令实参。
/// - `entry`：当前命令对应的 readonly 白名单条目。
///
/// 返回值：命中任一安全标志时返回 `true`。
/// 错误处理：该函数不返回错误；未知或无法匹配的标志返回 `false`。
pub fn is_safe_flag(arg: &str, entry: &CommandAllowlistEntry) -> bool {
    entry.safe_flags.iter().any(|flag| matches_flag(arg, flag))
}

/// 判断实参是否匹配白名单条目中声明的不安全标志。
///
/// 参数：
/// - `arg`：当前命令实参。
/// - `entry`：当前命令对应的 readonly 白名单条目。
///
/// 返回值：命中任一不安全标志时返回 `true`。
/// 错误处理：该函数不返回错误；未知或无法匹配的标志返回 `false`。
pub fn is_unsafe_flag(arg: &str, entry: &CommandAllowlistEntry) -> bool {
    entry.unsafe_flags.iter().any(|flag| matches_flag(arg, flag))
}

fn matches_flag(arg: &str, flag: &str) -> bool {
    arg == flag
        || arg.strip_prefix(flag).is_some_and(|suffix| {
            // 长标志只接受 `--flag=value`，避免把 `--foo` 误判为 `--foobar`。
            // 短标志允许非空后缀，用于覆盖 `-abc` 这类聚合写法。
            suffix.starts_with('=') || (!flag.starts_with("--") && !suffix.is_empty())
        })
}
#[cfg(test)]
#[path = "flag_validator_tests.rs"]
mod flag_validator_tests;
