//! 消息执行结果模块
//!
//! 该模块定义了 LLM 执行结果的表示方式，以及基于执行结果生成用户反馈的工具函数。
//! 主要用于通道管理器中处理和展示异步 LLM 调用的最终状态。

pub use channel_runtime::manager::message_result::*;
