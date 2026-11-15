//! 消息处理助手模块
//!
//! 本模块提供通道管理器中处理消息的核心辅助函数，负责消息的日志记录、
//! 钩子处理、安全防护、错误通知以及历史记录管理等功能。
//!
//! # 主要功能
//!
//! - **入站消息处理**：记录消息日志、应用入站钩子、执行语义安全检测
//! - **自动保存**：将符合条件的消息自动存储到记忆系统
//! - **历史管理**：查询和加载对话历史记录
//! - **错误通知**：向用户报告 provider 初始化失败等异常情况
//!
//! # 安全特性
//!
//! 模块集成了语义防护（Semantic Guard）机制，用于检测和拦截潜在的
//! 提示注入攻击，保护系统免受恶意输入的影响。

use super::*;
pub(crate) use channel_runtime::manager::message_helpers::*;
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/message_helpers_tests.rs"]
mod message_helpers_tests;
