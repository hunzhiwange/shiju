//! Agent 主循环模块
//!
//! 本模块是 VibeWindow 代理的核心执行引擎，负责协调代理与 AI 提供商之间的交互循环。
//! 它实现了工具调用循环、消息处理、历史管理等核心功能。
//!
//! # 模块结构
//!
//! - [`approval`] - 非CLI环境下的审批流程和上下文管理
//! - [`context`] - 消息上下文构建和管理
//! - [`core`] - 核心工具循环逻辑、历史构建和迭代控制
//! - [`cron`] - 定时任务注入和调度
//! - [`execution`] - 工具执行引擎（并行/顺序执行策略）
//! - [`history`] - 对话历史管理、压缩和裁剪
//! - [`instructions`] - 工具和Shell策略指令构建
//! - [`parsing`] - 工具调用解析（支持多种格式）
//! - [`progress`] - 草稿进度跟踪和状态管理
//! - [`query_engine`] - 会话级查询引擎，负责组装运行时并维护多轮历史
//! - [`runner`] - 消息处理的主入口点
//! - [`utils`] - 通用工具函数（如凭据清理）
//!
//! # 主要功能
//!
//! - 执行工具调用循环，支持迭代限制和取消机制
//! - 管理对话历史，包括自动压缩和裁剪
//! - 支持多种工具调用解析格式（OpenAI、GLM、Perl风格）
//! - 提供并行和顺序两种工具执行策略
//! - 集成定时任务调度功能
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::agent::loop_::run_tool_call_loop;
//!
//! // 运行工具调用循环
//! let result = run_tool_call_loop(config, provider, channel).await?;
//! ```

/// 非CLI环境下的审批流程和上下文管理
pub mod approval;

/// 消息上下文构建和管理
pub mod context;

/// 核心工具循环逻辑、历史构建和迭代控制
pub mod core;

/// 定时任务注入和调度
pub mod cron;

/// 工具执行引擎（并行/顺序执行策略）
pub mod execution;

/// 对话历史管理、压缩和裁剪
pub mod history;

/// 工具和Shell策略指令构建
pub mod instructions;

/// 工具调用解析（支持多种格式）
pub mod parsing;

/// 草稿进度跟踪和状态管理
pub mod progress;

/// 会话级查询引擎
pub mod query_engine;

/// 消息处理的主入口点
pub mod runner;

/// 通用工具函数
pub mod utils;

/// 单元测试和集成测试
#[cfg(test)]
mod tests;
#[cfg(test)]
mod utils_tests;

// ============================================================================
// 公共API重导出
// ============================================================================

/// 处理单条消息的主函数
///
/// 这是处理代理消息的公共API入口点
pub use runner::process_message;

// ============================================================================
// 内部辅助项（供测试和crate内部使用）
// ============================================================================

// ============================================================================
// 测试专用导出
// ============================================================================

/// 工具格式转换函数（仅供测试使用）
///
/// 将工具定义转换为OpenAI兼容格式
#[cfg(test)]
pub(crate) use core::tools_to_openai_format;

/// 核心常量和函数（仅供测试使用）
///
/// 包括：
/// - `DEFAULT_MAX_HISTORY_MESSAGES` - 默认最大历史消息数
/// - `DEFAULT_MAX_ITERATIONS` - 默认最大工具迭代次数
/// - `autosave_memory_key` - 自动保存记忆的键
/// - `build_native_assistant_history` - 构建原生助手历史
/// - `build_native_assistant_history_from_parsed_calls` - 从解析的调用构建历史
/// - `looks_like_unverified_action_completion_without_tool_call` - 检测未验证的操作完成
#[cfg(test)]
pub(crate) use core::{
    DEFAULT_MAX_HISTORY_MESSAGES, DEFAULT_MAX_ITERATIONS, autosave_memory_key,
    build_native_assistant_history, build_native_assistant_history_from_parsed_calls,
    looks_like_unverified_action_completion_without_tool_call,
};
// ============================================================================
// 核心循环功能导出
// ============================================================================

/// 核心工具循环函数和类型
///
/// 包括：
/// - `ToolLoopCancelled` - 工具循环取消错误类型
/// - `agent_turn` - 执行单个代理轮次
/// - `is_tool_iteration_limit_error` - 检查是否为迭代限制错误
/// - `is_tool_loop_cancelled` - 检查循环是否被取消
/// - `run_tool_call_loop` - 运行工具调用循环（基本版本）
/// - `run_tool_call_loop_with_non_cli_approval_context` - 带非CLI审批上下文的循环
/// - `run_tool_call_loop_with_reply_target` - 带回复目标的循环
pub(crate) use core::{
    ToolLoopCancelled, is_tool_iteration_limit_error, is_tool_loop_cancelled, run_tool_call_loop,
};

/// 指令构建函数
///
/// 包括：
/// - `build_shell_policy_instructions` - 构建Shell策略指令
/// - `build_tool_instructions` - 从工具列表构建指令
/// - `build_tool_instructions_from_specs` - 从规格说明构建指令
pub(crate) use instructions::{
    build_shell_policy_instructions, build_tool_instructions_from_specs,
};

/// 解析函数（仅供测试使用）
///
/// 包括：
/// - `ParsedToolCall` - 解析后的工具调用结构体
/// - `parse_tool_calls` - 核心工具调用解析入口
/// - `parse_structured_tool_calls` - 结构化工具调用解析
/// - `detect_tool_call_parse_issue` - 工具调用格式问题检测
/// - `parse_glm_style_tool_calls` - GLM 风格工具调用解析
/// - `parse_glm_shortened_body` - GLM 简化主体解析
/// - `parse_perl_style_tool_calls` - Perl 风格工具调用解析
/// - `parse_tool_calls_from_json_value` - 从 JSON 值解析工具调用
/// - `parse_tool_call_value` - 单个工具调用值解析
/// - `parse_arguments_value` - 参数值解析
/// - `default_param_for_tool` - 工具默认参数查询
#[cfg(test)]
pub(crate) use parsing::{
    ParsedToolCall, default_param_for_tool, detect_tool_call_parse_issue, parse_arguments_value,
    parse_glm_shortened_body, parse_glm_style_tool_calls, parse_perl_style_tool_calls,
    parse_structured_tool_calls, parse_tool_call_value, parse_tool_calls,
    parse_tool_calls_from_json_value,
};

/// 进度跟踪常量
///
/// 包括：
/// - `DRAFT_CLEAR_SENTINEL` - 草稿清除标记
/// - `DRAFT_PROGRESS_SENTINEL` - 草稿进度标记
/// - `DRAFT_WS_EVENT_SENTINEL` - WebSocket 私有结构化事件标记
/// - `PROGRESS_MIN_INTERVAL_MS` - 最小进度更新间隔（毫秒）
pub(crate) use progress::{DRAFT_CLEAR_SENTINEL, DRAFT_PROGRESS_SENTINEL, DRAFT_WS_EVENT_SENTINEL};

/// 凭据清理工具函数
///
/// 用于从日志和输出中清理敏感凭据信息
pub(crate) use utils::scrub_credentials;

// ============================================================================
// 遗留导入（用于使用 super::* 的测试）
// ============================================================================

/// 历史管理函数（仅供测试使用）
#[cfg(test)]
pub(crate) use history::{apply_compaction_summary, build_compaction_transcript, trim_history};

/// 上下文构建函数（仅供测试使用）
#[cfg(test)]
pub(crate) use context::build_context;

/// 工具执行策略函数（仅供测试使用）
#[cfg(test)]
pub(crate) use execution::should_execute_tools_in_parallel;

/// 非 CLI 审批上下文（供 crate 内部复用）
pub(crate) use approval::NonCliApprovalContext;

/// 带 非 CLI 审批上下文的工具循环函数（仅供测试使用）
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use core::run_tool_call_loop_with_non_cli_approval_context;

pub use parsing::extract_json_values;
