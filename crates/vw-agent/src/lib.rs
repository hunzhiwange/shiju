//! # Agent 模块
//!
//! 本模块是 VibeWindow 代理系统的核心库，提供自主代理的完整运行时能力。
//!
//! ## 模块职责
//!
//! - **编排与执行**：代理的核心调度、任务分类、研究与提示构建
//! - **通道集成**：多平台消息通道（Telegram、Slack、Discord、WhatsApp 等）
//! - **模型调用**：Provider trait 及各类 LLM/AI 模型适配器
//! - **工具执行**：Shell、文件、检索、计划任务等工具面
//! - **记忆系统**：多后端记忆层（Markdown、SQLite、Postgres、Vector 等）
//! - **安全策略**：访问控制、隔离、泄漏检测、紧急停止
//! - **运行时适配**：Native、Docker、WASM 等运行时环境
//! - **观测性**：日志与观测后端聚合
//! - **调度任务**：Cron 表达式驱动的定时任务存储与执行
//! - **技能生态**：技能的加载、发现、审计与执行
//!
//! ## 扩展点
//!
//! 本模块采用 trait 驱动架构，核心扩展点包括：
//! - [`providers::Provider`] — 模型提供者接口
//! - [`channels::Channel`] — 通道接口
//! - [`tools::Tool`] — 工具接口
//! - [`memory::Memory`] — 记忆存储接口
//! - [`observability::Observer`] — 观测接口
//! - [`runtime::RuntimeAdapter`] — 运行时适配接口
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use vibe_window::Config;
//!
//! // 加载配置
//! let config = Config::load().expect("Failed to load config");
//!
//! // 配置可通过 CLI 子命令进行管理
//! ```

#![warn(clippy::all, clippy::pedantic)]
#![allow(
    // 当前阶段优先保证工作区 Clippy 收敛，保留后续按专题治理风格告警的空间
    clippy::all,
    // vw-agent 历史代码量较大，暂不将 pedantic 告警作为阻塞项
    clippy::pedantic,
    // 允许使用 clone 方法进行赋值，简化代码
    clippy::assigning_clones,
    // 允许在 if 表达式中将 bool 转换为 int，常见于条件计数
    clippy::bool_to_int_with_if,
    // 允许区分大小写的文件扩展名比较，保持跨平台一致性
    clippy::case_sensitive_file_extension_comparisons,
    // 允许可能溢出的类型转换，由业务逻辑保证安全性
    clippy::cast_possible_wrap,
    // 允许文档中的 Markdown 语法，支持更丰富的文档格式
    clippy::doc_markdown,
    // 允许文档中包含带引号的示例文本，避免被误判为链接
    clippy::doc_link_with_quotes,
    // 允许使用 Default 后重新赋值字段，提高可读性
    clippy::field_reassign_with_default,
    // 允许浮点数比较，在阈值比较场景中常见
    clippy::float_cmp,
    // 允许追加格式化字符串时保持当前写法，避免大规模无行为改动
    clippy::format_push_string,
    // 允许隐式 clone，减少显式 .clone() 调用
    clippy::implicit_clone,
    // 允许在语句后声明项，保持代码组织灵活性
    clippy::items_after_statements,
    // 允许多层 if 保持局部流程显式
    clippy::collapsible_if,
    // 允许使用 map().unwrap_or() 模式，语义清晰
    clippy::map_unwrap_or,
    // 允许文档列表延续格式，避免为说明性文档做大规模重排
    clippy::doc_lazy_continuation,
    // 允许通过索引遍历以保持双向区间计算更直接
    clippy::needless_range_loop,
    // 允许先 trim 再 split，保持输入清洗步骤显式
    clippy::trim_split_whitespace,
    // 允许相同 match 分支体，减少提炼出的中间状态分支
    clippy::match_same_arms,
    // 允许保留 repeat.take 形式，避免为 lint 改写成熟逻辑
    clippy::manual_repeat_n,
    // 允许潜在截断转换，由上层时间范围约束保证安全
    clippy::cast_possible_truncation,
    // 允许显式生命周期，便于树遍历代码阅读
    clippy::needless_lifetimes,
    // 允许结构体包含多个布尔位，贴合当前状态建模
    clippy::struct_excessive_bools,
    // 允许 map(identity) 形式，便于链式表达保持一致
    clippy::map_identity,
    // 允许显式 return，便于控制流与早返回一致
    clippy::needless_return,
    // 允许布尔非运算保持原判定方向
    clippy::nonminimal_bool,
    // 允许 filter_map 写法保持链式结构统一
    clippy::unnecessary_filter_map,
    // 允许借用模式保持解构形状清晰
    clippy::needless_borrowed_reference,
    // 允许手动 let-else 模式，兼容旧版 Rust
    clippy::manual_let_else,
    // 允许函数缺少错误文档，减少文档负担
    clippy::missing_errors_doc,
    // 允许函数缺少 panic 文档，减少文档负担
    clippy::missing_panics_doc,
    // 允许模块名重复，反映领域模型
    clippy::module_name_repetitions,
    // 允许测试模块与目录按同名组织
    clippy::module_inception,
    // 允许不标记 must_use，由调用方决定是否使用返回值
    clippy::must_use_candidate,
    // 允许无 Default 实现的 new 方法，某些类型不适合默认构造
    clippy::new_without_default,
    // 允许按值传递参数，某些场景下所有权转移更清晰
    clippy::needless_pass_by_value,
    // 允许原始字符串字面量中不必要的 #，保持一致性
    clippy::needless_raw_string_hashes,
    // 允许方法调用的冗余闭包，提高可读性
    clippy::redundant_closure_for_method_calls,
    // 允许返回 Self 但不标记 must_use
    clippy::return_self_not_must_use,
    // 允许相似的命名，反映领域概念
    clippy::similar_names,
    // 允许单分支 match-else，某些场景更清晰
    clippy::single_match_else,
    // 允许结构体字段名包含类型名，提高可读性
    clippy::struct_field_names,
    // 允许函数行数较多，核心逻辑可能需要较长实现
    clippy::too_many_lines,
    // 允许保留异步 API 形状，即使当前实现没有 await
    clippy::unused_async,
    // 允许较大的 Future，避免为通过 lint 引入额外 boxing
    clippy::large_futures,
    // 允许非内联格式参数，兼容旧格式风格
    clippy::uninlined_format_args,
    // 允许不必要的类型转换，某些场景提供额外保障
    clippy::unnecessary_cast,
    // 允许不必要的惰性求值，保持代码一致性
    clippy::unnecessary_lazy_evaluations,
    // 允许不必要的字面量边界，减少噪音
    clippy::unnecessary_literal_bound,
    // 允许不必要的 map_or，某些场景更清晰
    clippy::unnecessary_map_or,
    // 允许未使用的 self，trait 实现可能暂时不需要
    clippy::unused_self,
    // 允许精度损失的类型转换，由业务逻辑保证安全性
    clippy::cast_precision_loss,
    // 允许不必要的包装，某些场景提供一致性
    clippy::unnecessary_wraps,
    // 允许死代码警告，开发过程中可能暂时未使用
    dead_code
)]
#![cfg_attr(test, allow(unused_imports))]
#![recursion_limit = "256"]

// ============================================================================
// 模块声明
// ============================================================================

mod cli_commands;
#[cfg(test)]
#[path = "cli_commands_tests.rs"]
mod cli_commands_tests;

// 核心编排与执行
pub mod acp;
pub mod agent;

// 应用兼容层（为拆分期保留旧路径）
pub(crate) mod app;

// 内部模块（仅在当前 crate 内可见）
pub mod approval;
pub mod auth;

// 事件总线
pub mod bus;

// 通道集成
pub mod channels;

// 配置管理
pub mod config;

// 命令定义
pub mod command;

// 定时任务
pub mod cron;

// 守护进程管理
pub mod daemon;

// 诊断工具
pub mod doctor;

// 环境变量工具
pub mod env;

// 文件系统工具
pub mod file;

// 运行时标志
pub mod flag;

// 文本格式化（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod format;

// API 网关（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod gateway;

// 目标引擎
pub mod goals;

// 全局状态
pub mod global;

// 全局 SQLite 连接基础（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod global_database;

// 健康检查
pub(crate) mod health;

// 心跳机制
pub(crate) mod heartbeat;

// 扩展钩子
pub mod hooks;

// 身份管理
pub mod identity;

// ID 生成
pub mod id;

// 安装管理
pub mod installation;

// 第三方集成
pub mod integrations;

// 记忆系统
pub mod memory;

// 知识库运行时
#[cfg(not(target_arch = "wasm32"))]
pub mod knowledge;

// MCP 协议（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod mcp;

// 配置迁移
pub(crate) mod migration;

// 多模态支持
pub(crate) mod multimodal;

// 观测性
pub mod observability;

// 补丁工具
pub mod patch;

// 权限系统
pub mod permission;

// 插件包清单解析与静态审计
pub mod plugins;

// 基于 git2 的只读仓库发现
pub(crate) mod git_repository;

// 项目上下文
pub mod project;

// Provider 抽象兼容层
pub mod provider;

// 模型提供者
pub mod providers;

// PTY（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod pty;

// 交互提问
pub mod question;

// 运行时适配
pub mod runtime;

// 调度器
pub mod scheduler;

// Agent Scheduler（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub mod agent_scheduler;

// 安全策略
pub mod security;

// 服务管理
pub mod service;

/// SHIJU upstream integrations.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod shiju;

// 会话系统
pub mod session;

// Shell 执行
pub mod shell;

// 技能核心
pub mod skill;

// Skillforge
pub mod skillforge;

// 技能生态
pub mod skills;

// 快照系统
pub mod snapshot;

// 存储层
pub mod storage;

// 任务池 worktree pool
pub mod task_worktree_pool;

// 任务生命周期 workflow 领域服务
pub mod task_lifecycle;

// 聊天工作流执行（tasks.json 调度）
pub mod task_plan_run;

// 测试能力
pub mod test_capabilities;

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Mutex, OnceLock};

    pub(crate) fn global_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    pub(crate) fn global_runtime_proxy_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    pub(crate) fn global_async_env_lock() -> &'static tokio::sync::Mutex<()> {
        static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        &LOCK
    }
}

// 工具执行
pub mod tools;

// 隧道服务（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod tunnel;

// 更新管理
pub mod update;

// 通用工具
pub mod util;

// 工作树管理
pub mod worktree;

// Dify Workflow 执行
#[cfg(not(target_arch = "wasm32"))]
pub mod workflow;

// ============================================================================
// 公共类型重导出
// ============================================================================

pub use cli_commands::{
    ChannelCommands, CronCommands, IntegrationCommands, MemoryCommands, MigrateCommands,
    PluginCommands, ServiceCommands, SkillCommands,
};
/// 重导出配置类型，作为公共 API 的一部分
pub use config::Config;
pub use health::initialize_runtime_build_id;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
include!("agent_scheduler_policy_legacy_review_exact_tests.rs");
