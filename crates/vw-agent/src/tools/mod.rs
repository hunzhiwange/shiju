//! 代理工具子系统 - 提供代理可调用的能力接口
//!
//! 本模块实现了在代理循环（agentic loop）过程中暴露给大语言模型（LLM）的工具执行面。
//! 每个工具都需要实现 [`traits`] 模块中定义的 [`Tool`] trait，该 trait 要求实现：
//! - 工具名称（`name`）
//! - 工具描述（`description`）
//! - JSON 参数 schema（`parameters_schema`）
//! - 异步执行方法（`execute`），返回结构化的 [`ToolResult`]
//!
//! # 工具注册集
//!
//! 模块提供两个主要的工具注册函数：
//! - [`default_tools`]：默认工具集，包含 shell、文件读写等基础工具
//! - [`all_tools`]：完整工具集，包含记忆、浏览器、定时任务、HTTP、委托代理、可选集成等
//!
//! # 安全策略
//!
//! 安全策略通过 [`SecurityPolicy`](crate::app::agent::security::SecurityPolicy) 在构造时注入，
//! 用于控制工具的访问权限和执行边界。
//!
//! # 扩展指南
//!
//! 添加新工具的步骤：
//! 1. 在新的子模块中实现 [`Tool`] trait
//! 2. 在 [`all_tools_with_runtime`] 函数中注册该工具
//! 3. 参见 `AGENTS.md` 第 7.3 节获取完整的变更手册

pub mod agent_tool;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
pub mod apply_patch;
pub mod batch;
#[cfg(not(target_arch = "wasm32"))]
pub mod browser;
pub mod browser_open;
#[cfg(not(target_arch = "wasm32"))]
pub mod codesearch;
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod content_search;
mod context;
pub mod cron_add;
pub mod cron_list;
pub mod cron_remove;
pub mod cron_run;
pub mod cron_runs;
pub mod cron_update;
mod decision;
pub mod delegate;
pub(crate) mod delegated_tools;
#[cfg(not(target_arch = "wasm32"))]
pub mod enter_worktree;
mod executor;
#[cfg(not(target_arch = "wasm32"))]
pub mod exit_worktree;
#[cfg(not(target_arch = "wasm32"))]
pub mod external_directory;
#[cfg(not(target_arch = "wasm32"))]
pub mod file_edit;
#[cfg(not(target_arch = "wasm32"))]
pub mod file_read;
#[cfg(not(target_arch = "wasm32"))]
pub mod file_write;
#[cfg(not(target_arch = "wasm32"))]
pub mod git_operations;
#[cfg(not(target_arch = "wasm32"))]
pub mod glob;
#[cfg(not(target_arch = "wasm32"))]
pub mod glob_search;
#[cfg(not(target_arch = "wasm32"))]
pub mod grep;
mod hooks;
mod http_headers;
pub mod http_request;
pub mod image_asset_read;
pub mod image_info;
#[cfg(not(target_arch = "wasm32"))]
pub mod ls;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp;
pub mod mcp_common;
pub mod mcp_inspect;
pub mod mcp_status;
pub mod mcp_tool;
pub mod memory_forget;
pub mod memory_recall;
pub mod memory_store;
#[cfg(not(target_arch = "wasm32"))]
pub mod pdf_read;
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
pub mod powershell;
#[cfg(not(target_arch = "wasm32"))]
pub mod process;
#[cfg(not(target_arch = "wasm32"))]
pub mod question;
mod read_state;
pub mod registry;
pub mod remote_trigger;
#[cfg(not(target_arch = "wasm32"))]
pub mod rustcodegraph;
pub mod schedule;
mod scheduler;
pub mod schema;
#[cfg(not(target_arch = "wasm32"))]
pub mod screenshot;
#[cfg(not(target_arch = "wasm32"))]
pub mod shell;
pub mod skill;
pub mod sleep;
pub mod todo;
pub mod tool_search;
mod toolset;
pub mod traits;
pub mod truncation;
pub mod url_validation;
pub mod wasm_module;
pub mod web_fetch;
pub mod web_search_tool;
pub mod websearch;

pub use agent_tool::AgentTool;
#[cfg(not(target_arch = "wasm32"))]
pub use apply_patch::ApplyPatchTool;
pub use batch::BatchTool;
#[cfg(not(target_arch = "wasm32"))]
pub use browser::{BrowserTool, ComputerUseConfig};
pub use browser_open::BrowserOpenTool;
#[cfg(not(target_arch = "wasm32"))]
pub use codesearch::CodeSearchTool;
pub use config::ConfigTool;
#[cfg(not(target_arch = "wasm32"))]
pub use content_search::ContentSearchTool;
pub use context::{ToolUseContext, current_tool_use_context};
pub use cron_add::CronAddTool;
pub use cron_list::CronListTool;
pub use cron_remove::CronRemoveTool;
pub use cron_run::CronRunTool;
pub use cron_runs::CronRunsTool;
pub use cron_update::CronUpdateTool;
#[cfg(not(target_arch = "wasm32"))]
pub use enter_worktree::EnterWorktreeTool;
pub(crate) use executor::{ToolResultHistoryEntry, build_tool_result_history_messages};
#[cfg(not(target_arch = "wasm32"))]
pub use exit_worktree::ExitWorktreeTool;
#[cfg(not(target_arch = "wasm32"))]
pub use file_edit::FileEditTool;
#[cfg(not(target_arch = "wasm32"))]
pub use file_read::FileReadTool;
#[cfg(not(target_arch = "wasm32"))]
pub use file_write::FileWriteTool;
#[cfg(not(target_arch = "wasm32"))]
pub use git_operations::GitOperationsTool;
#[cfg(not(target_arch = "wasm32"))]
pub use glob::GlobTool;
#[cfg(not(target_arch = "wasm32"))]
pub use glob_search::GlobSearchTool;
#[cfg(not(target_arch = "wasm32"))]
pub use grep::GrepTool;
pub use http_request::HttpRequestTool;
pub use image_asset_read::ImageAssetReadTool;
pub use image_info::ImageInfoTool;
#[cfg(not(target_arch = "wasm32"))]
pub use ls::LsTool;
#[cfg(not(target_arch = "wasm32"))]
pub use lsp::LspTool;
pub use mcp_status::McpStatusTool;
pub use memory_forget::MemoryForgetTool;
pub use memory_recall::MemoryRecallTool;
pub use memory_store::MemoryStoreTool;
#[cfg(not(target_arch = "wasm32"))]
pub use pdf_read::PdfReadTool;
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
pub use powershell::PowerShellTool;
#[cfg(not(target_arch = "wasm32"))]
pub use process::ProcessTool;
#[cfg(not(target_arch = "wasm32"))]
pub use question::QuestionTool;
pub use read_state::{FileReadStateCache, FileReadStateEntry, FileSnapshot};
pub use remote_trigger::RemoteTriggerTool;
#[cfg(not(target_arch = "wasm32"))]
pub use rustcodegraph::RustCodeGraphTool;
pub use schedule::ScheduleTool;
#[allow(unused_imports)]
pub use schema::{CleaningStrategy, SchemaCleanr};
#[cfg(not(target_arch = "wasm32"))]
pub use screenshot::ScreenshotTool;
#[cfg(not(target_arch = "wasm32"))]
pub use shell::ShellTool;
pub use skill::SkillTool;
pub use sleep::SleepTool;
pub use todo::{Todo, TodoReadTool, TodoWriteTool};
pub use tool_search::ToolSearchTool;
pub use traits::Tool;
#[allow(unused_imports)]
pub use traits::{
    ToolCallResult, ToolCallTelemetry, ToolContextUpdate, ToolExtraMessage, ToolRenderHint,
    ToolResult, ToolSpec,
};

pub(crate) use scheduler::{
    PendingToolCall, ScheduledToolBatch, ScheduledToolBatchMode, schedule_tool_calls,
};
pub use toolset::{
    ExecutedToolCall, ToolCallError, ToolRuntimeContext, all_tools, all_tools_with_runtime,
    default_tools, default_tools_with_runtime, execute_tool_call, execute_tool_from_registry,
    is_binary, tool_specs_for_context,
};
pub(crate) use toolset::{agent_tool_parallel_limit_for_context, execute_agent_tool_call};
pub use wasm_module::WasmModuleTool;
pub use web_fetch::WebFetchTool;
pub use web_search_tool::WebSearchTool;

pub const ASK_USER_QUESTION_TOOL_ID: &str = "AskUserQuestion";
pub const TODO_READ_TOOL_ID: &str = "TodoRead";
pub const TODO_WRITE_TOOL_ID: &str = "TodoWrite";
pub const WEB_FETCH_TOOL_ID: &str = "WebFetch";
pub const WEB_SEARCH_TOOL_ID: &str = "WebSearch";
pub const BROWSER_TOOL_ID: &str = "Browser";
pub const BROWSER_OPEN_TOOL_ID: &str = "BrowserOpen";

pub const QUESTION_TOOL_ALIAS: &str = "question";
pub const TODO_READ_TOOL_ALIAS: &str = "todoread";
pub const TODO_WRITE_TOOL_ALIAS: &str = "todowrite";
pub const WEB_FETCH_TOOL_ALIAS: &str = "web_fetch";
pub const WEB_SEARCH_TOOL_ALIAS: &str = "web_search_tool";
pub const BROWSER_TOOL_ALIAS: &str = "browser";
pub const BROWSER_OPEN_TOOL_ALIAS: &str = "browser_open";
pub const ENTER_WORKTREE_TOOL_ID: &str = "enter_worktree";
pub const EXIT_WORKTREE_TOOL_ID: &str = "exit_worktree";
pub const TOOL_SEARCH_TOOL_ID: &str = "tool_search";

pub const ENTER_WORKTREE_TOOL_ALIASES: [&str; 1] = ["EnterWorktree"];
pub const EXIT_WORKTREE_TOOL_ALIASES: [&str; 1] = ["ExitWorktree"];
pub const TOOL_SEARCH_TOOL_ALIASES: [&str; 1] = ["ToolSearch"];

pub fn is_question_tool_id(id: &str) -> bool {
    matches!(id, ASK_USER_QUESTION_TOOL_ID | QUESTION_TOOL_ALIAS)
}

pub fn is_todo_read_tool_id(id: &str) -> bool {
    matches!(id, TODO_READ_TOOL_ID | TODO_READ_TOOL_ALIAS)
}

pub fn is_todo_write_tool_id(id: &str) -> bool {
    matches!(id, TODO_WRITE_TOOL_ID | TODO_WRITE_TOOL_ALIAS)
}

pub fn is_web_fetch_tool_id(id: &str) -> bool {
    matches!(id, WEB_FETCH_TOOL_ID | WEB_FETCH_TOOL_ALIAS | "webfetch")
}

pub fn is_web_search_tool_id(id: &str) -> bool {
    matches!(id, WEB_SEARCH_TOOL_ID | WEB_SEARCH_TOOL_ALIAS | "websearch" | "web_search")
}

pub fn is_browser_tool_id(id: &str) -> bool {
    matches!(id, BROWSER_TOOL_ID | BROWSER_TOOL_ALIAS)
}

pub fn is_browser_open_tool_id(id: &str) -> bool {
    matches!(id, BROWSER_OPEN_TOOL_ID | BROWSER_OPEN_TOOL_ALIAS)
}

#[cfg(test)]
use crate::app::agent::config::DelegateAgentConfig;
#[cfg(test)]
use crate::app::agent::memory::Memory;
#[cfg(test)]
use crate::app::agent::runtime::RuntimeAdapter;
#[cfg(test)]
use crate::app::agent::security::SecurityPolicy;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
mod mcp_tool_tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod coverage_tests;

#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;
#[cfg(test)]
pub(crate) use delegate::DelegateTool;
