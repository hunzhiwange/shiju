//! LLM 会话日志记录模块
//!
//! 提供 LLM 会话专用的日志记录器，用于记录 LLM 交互过程中的各种事件和信息。
//! 日志记录器会自动添加 `service: llm` 标签，便于在日志流中识别和过滤 LLM 相关的日志条目。

use agent_log as log;
use serde_json::{Map, Value};
use std::sync::LazyLock;

/// LLM 会话专用的全局日志记录器
///
/// 这是一个使用 `once_cell::sync::Lazy` 实现的延迟初始化静态变量，
/// 确保在整个应用程序生命周期内只有一个日志记录器实例，并且线程安全。
///
/// 日志记录器会自动为所有日志添加 `service: llm` 标签，用于标识日志来源。
pub static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    // 创建日志记录器，并配置默认元数据
    log::create(Some({
        // 构建元数据映射，添加服务标识标签
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("llm".to_string()));
        m
    }))
});

#[cfg(test)]
#[path = "logging_tests.rs"]
mod logging_tests;
