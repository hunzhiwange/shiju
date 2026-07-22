//! VibeWindow 对外共享的 API 类型定义。
//!
//! 本 crate 集中定义前后端、网关、桌面端与代理运行时之间共享的数据结构，目标是：
//! - 用稳定的 DTO 表达跨模块边界上的输入输出
//! - 将 ID、时间、分页、错误等基础概念统一建模
//! - 为会话、聊天、项目、工具、任务等核心域提供一致的序列化结构
//! - 减少不同 crate 之间重复声明同一份协议类型
//!
//! # 模块概览
//!
//! - [`agent_scheduler`][]: Agent Scheduler 任务、Worker、Team、lease、事件与 outbox DTO
//! - [`apiql`][]: ApiQL 只读列表、分页响应与实体配置 DTO
//! - [`chat`][]: 聊天消息、流式事件与聊天请求体
//! - [`data`][]: AI-DATA 连接、报表、查询与 AI 规划协议
//! - [`session`][]: 会话详情、列表、归档、分叉与网关会话操作
//! - [`project`][]: 项目实体、项目列表与项目级变更记录
//! - [`file`][]: 文件树、文件读写、搜索与文件元信息
//! - [`git`][]: Git 状态、diff、分支切换与提交选择
//! - [`knowledge`][]: 知识库、文档、检索与运行时能力 DTO
//! - [`provider`][]: 模型提供商与模型能力信息
//! - [`settings`][]: 运行时、安全、多代理与浏览器等设置
//! - [`staff`][]: 员工任务流本地 IPC 消息 DTO
//! - [`task`][]: 后台任务、任务事件流与执行请求
//! - [`task_lifecycle`][]: 任务生命周期 workflow 跨网关 DTO
//! - [`todo`][]: 会话待办项与待办更新请求
//! - [`tools`][]: Claude Tools V2 共享规格与结果 DTO
//! - [`workflow`][]: Dify Workflow 执行请求、响应与节点事件 DTO
//! - [`question`][]: 交互式问题与用户回答结构
//! - [`worktree`][]: 工作树创建、删除、切换与重置
//! - [`common`][]: 时间戳、分页、映射等基础通用类型
//! - [`error`][]: 统一 API 错误结构
//! - [`id`][]: 各领域实体的强类型 ID 包装
//!
//! # 设计约束
//!
//! - 类型优先保持朴素、可序列化、易跨语言消费
//! - 默认面向 API 边界，不承载运行时行为
//! - 避免在 DTO 中引入隐式业务逻辑
//! - 通过新类型包装 ID，降低字符串误用风险

pub mod agent_scheduler;
pub mod apiql;
pub mod chat;
pub mod common;
pub mod data;
pub mod error;
pub mod file;
pub mod git;
pub mod id;
pub mod knowledge;
pub mod mcp;
pub mod project;
pub mod provider;
pub mod question;
pub mod session;
pub mod settings;
pub mod staff;
pub mod task;
pub mod task_lifecycle;
pub mod todo;
pub mod tools;
pub mod workflow;
pub mod worktree;

#[cfg(test)]
#[path = "agent_scheduler_tests.rs"]
mod agent_scheduler_tests;
#[cfg(test)]
#[path = "apiql_tests.rs"]
mod apiql_tests;
#[cfg(test)]
#[path = "chat_tests.rs"]
mod chat_tests;
#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;
#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
#[cfg(test)]
#[path = "file_tests.rs"]
mod file_tests;
#[cfg(test)]
#[path = "git_tests.rs"]
mod git_tests;
#[cfg(test)]
#[path = "id_tests.rs"]
mod id_tests;
#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod knowledge_tests;
#[cfg(test)]
#[path = "project_tests.rs"]
mod project_tests;
#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
#[cfg(test)]
#[path = "question_tests.rs"]
mod question_tests;
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
#[cfg(test)]
#[path = "staff_tests.rs"]
mod staff_tests;
#[cfg(test)]
#[path = "task_lifecycle_tests.rs"]
mod task_lifecycle_tests;
#[cfg(test)]
#[path = "task_tests.rs"]
mod task_tests;
#[cfg(test)]
#[path = "todo_tests.rs"]
mod todo_tests;
#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;
#[cfg(test)]
#[path = "workflow_tests.rs"]
mod workflow_tests;
#[cfg(test)]
#[path = "worktree_tests.rs"]
mod worktree_tests;

pub use chat::ReasoningEffortDto;
pub use common::*;
pub use error::*;
pub use id::*;
