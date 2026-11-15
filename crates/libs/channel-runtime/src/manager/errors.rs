//! 通道错误处理辅助模块
//!
//! 本模块提供与通道运行时相关的错误检测与判断辅助函数。
//! 主要用于识别特定类型的错误（如上下文窗口溢出、工具迭代限制等）
//! 以及检查工具在非 CLI 环境下的排除状态。

use super::*;

/// 检查指定工具是否在非 CLI 环境下被排除
///
/// 某些工具在非命令行交互环境下可能不适合使用（例如需要用户交互的工具），
/// 该函数通过查询通道运行时上下文中的排除列表来判断给定工具是否被排除。
///
/// # 参数
///
/// * `ctx` - 通道运行时上下文，包含被排除工具的列表
/// * `tool_name` - 待检查的工具名称
///
/// # 返回值
///
/// 如果该工具在排除列表中则返回 `true`，否则返回 `false`
///
/// # 线程安全
///
/// 该函数通过互斥锁访问共享的排除列表，即使锁被污染（例如前次访问时发生 panic）
/// 也能安全地恢复并继续执行。
pub fn is_non_cli_tool_excluded(ctx: &ChannelRuntimeContext, tool_name: &str) -> bool {
    // 获取互斥锁，若锁被污染则恢复其内部值继续使用
    ctx.non_cli_excluded_tools
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        // 检查排除列表中是否存在与目标工具名匹配的条目
        .any(|excluded| excluded == tool_name)
}

/// 判断错误是否为上下文窗口溢出类型
///
/// 不同的大语言模型提供商对上下文长度超限的报错信息各不相同，
/// 该函数通过匹配常见的错误提示文本来识别此类错误。
///
/// # 参数
///
/// * `err` - 待检查的错误引用
///
/// # 返回值
///
/// 如果错误信息包含已知的上下文窗口溢出提示则返回 `true`，否则返回 `false`
///
/// # 匹配模式
///
/// 当前支持识别以下错误提示（不区分大小写）：
/// - "exceeds the context window"
/// - "context window of this model"
/// - "maximum context length"
/// - "context length exceeded"
/// - "too many tokens"
/// - "token limit exceeded"
/// - "prompt is too long"
/// - "input is too long"
pub fn is_context_window_overflow_error(err: &anyhow::Error) -> bool {
    // 将错误信息转换为小写以便进行不区分大小写的匹配
    let lower = err.to_string().to_lowercase();
    // 遍历已知的上下文溢出错误提示，检查是否存在匹配
    [
        "exceeds the context window",
        "context window of this model",
        "maximum context length",
        "context length exceeded",
        "too many tokens",
        "token limit exceeded",
        "prompt is too long",
        "input is too long",
    ]
    .iter()
    .any(|hint| lower.contains(hint))
}

/// 判断错误是否为工具迭代次数限制错误
///
/// 代理在执行任务时会调用各种工具，为防止无限循环，
/// 系统对单次任务中工具调用的迭代次数设置了上限。
/// 该函数委托给代理循环模块中的相应实现来完成判断。
///
/// # 参数
///
/// * `err` - 待检查的错误引用
///
/// # 返回值
///
/// 如果该错误为工具迭代限制错误则返回 `true`，否则返回 `false`
pub fn is_tool_iteration_limit_error(err: &anyhow::Error) -> bool {
    agent_loop::core::errors::is_tool_iteration_limit_error(err)
}

#[cfg(test)]
#[path = "errors_tests.rs"]
mod errors_tests;
