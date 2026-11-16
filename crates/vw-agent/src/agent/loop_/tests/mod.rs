//! # Agent 主循环测试模块
//!
//! 本模块是 Agent 主循环（`loop_`）的测试套件入口，负责组织和集成所有相关的单元测试与集成测试。
//!
//! ## 模块职责
//!
//! - 提供 Agent 主循环逻辑的全面测试覆盖
//! - 验证工具调用解析、协议解析、内存上下文管理、定时任务投递等核心功能
//! - 确保 Agent 在各种边界条件和异常场景下的稳定性和正确性
//!
//! ## 子模块结构
//!
//! - `constants`: 测试常量与工具函数
//! - `cron_delivery`: 定时任务投递逻辑测试
//! - `loop_tools`: 主循环工具相关测试
//! - `memory_context`: 内存上下文管理测试
//! - `parsing_edge_cases`: 解析器边界条件测试
//! - `parsing_extract_json`: JSON 提取逻辑测试
//! - `parsing_glm`: GLM 模型响应解析测试
//! - `parsing_protocols`: 协议解析测试
//! - `parsing_recovery`: 解析器恢复机制测试
//! - `parsing_structured`: 结构化数据解析测试
//! - `parsing_tool_calls`: 工具调用解析测试
//! - `parsing_value`: 值解析测试
//! - `shell_policy`: Shell 策略执行测试
//! - `tool_instructions`: 工具指令生成测试

// 引入父模块（loop_）的所有公共项，包括 AgentLoop、LoopState 等核心类型
use super::*;

// 引入审批管理器和审批响应类型，用于测试需要人工确认的场景
use crate::app::agent::approval::{ApprovalManager, ApprovalResponse};

// 引入聊天消息和工具调用类型，用于构建测试输入
use model_types::{ChatMessage, ToolCall};

// 引入聊天请求和提供者 trait，用于模拟 Provider 交互
use model_types::{ChatRequest, Provider};

// 引入工具 trait，用于测试工具注册和执行
use crate::app::agent::tools::Tool;

// 引入原子引用计数，用于跨线程共享测试资源
use std::sync::Arc;

pub(crate) use super::core::tools_to_openai_format;
pub(crate) use super::cron::maybe_inject_cron_add_delivery;
pub(crate) use super::instructions::{build_shell_policy_instructions, build_tool_instructions};
pub(crate) use super::parsing::{
    ParsedToolCall, default_param_for_tool, detect_tool_call_parse_issue, parse_arguments_value,
    parse_glm_shortened_body, parse_glm_style_tool_calls, parse_perl_style_tool_calls,
    parse_structured_tool_calls, parse_tool_call_value, parse_tool_calls,
    parse_tool_calls_from_json_value,
};

// ============================================================================
// 测试子模块声明
// ============================================================================

// constants 测试已在 agent-loop 注册。
// cron_delivery 测试已在 agent-loop 注册。
#[path = "../../../../../libs/agent-loop/src/tests/loop_suite/loop_tools.rs"]
mod loop_tools;
#[path = "../../../../../libs/agent-loop/src/tests/loop_suite/memory_context.rs"]
mod memory_context;
// parsing_edge_cases 测试已在 agent-loop 注册。
// parsing_extract_json 测试已在 agent-loop 注册。
mod parsing_glm;
mod parsing_protocols;
mod parsing_recovery;
mod parsing_structured;
mod parsing_tool_calls;
mod parsing_value;
mod shell_policy;
mod tool_instructions;
