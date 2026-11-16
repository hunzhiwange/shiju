//! # 代理循环测试套件
//!
//! 本模块包含代理核心循环的完整测试集，覆盖 `Agent.turn()` 的全生命周期。
//!
//! ## 设计目标
//!
//! - 使用模拟的 provider 和 tool 隔离外部依赖
//! - 覆盖代理工具循环必须处理的所有边界情况
//! - 确保代理行为在各种输入条件下的一致性和可预测性
//!
//! ## 子模块说明
//!
//! - `helpers` — 测试辅助工具与通用 fixture 定义
//! - `loop_history` — 历史记录管理与检索相关测试
//! - `loop_memory` — 内存/上下文记忆系统集成测试
//! - `loop_text` — 文本响应处理与流式输出测试
//! - `loop_tools` — 工具调用循环逻辑测试（含嵌套调用场景）
//! - `run_single` — 单轮代理执行的端到端测试
//! 消息序列化测试位于 model-types 的集成测试中。
//! - `tool_dispatcher` — 工具分发与执行路由测试

mod helpers;
mod loop_history;
mod loop_memory;
mod loop_text;
mod loop_tools;
mod run_single;
// 消息序列化断言由 model-types/tests/conversation_serialization.rs 注册。
mod tool_dispatcher;
