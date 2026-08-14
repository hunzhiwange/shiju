//! 会话领域模块入口，集中声明会话运行、提示词、状态同步和 UI 持久化相关子模块。

/// 声明委托能力的纯策略解析器。
pub(crate) mod capability_policy;
#[cfg(test)]
#[path = "capability_policy_tests.rs"]
mod capability_policy_tests;
/// 声明持久化子会话执行子模块。
pub mod child_agent;
#[cfg(test)]
#[path = "child_agent_tests.rs"]
mod child_agent_tests;
/// 声明 compaction 子模块，保持当前领域的职责拆分清晰。
pub mod compaction;
/// 声明临时模型上下文装配器，避免 AGENTS 进入可见聊天记录。
pub mod context_injection;
/// 声明 instruction 子模块，保持当前领域的职责拆分清晰。
pub mod instruction;
/// 声明 instruction discovery 子模块，集中确定 AGENTS 文件的来源与优先级。
pub mod instruction_discovery;
/// 声明 llm 子模块，保持当前领域的职责拆分清晰。
pub mod llm;
/// 声明 message 子模块，保持当前领域的职责拆分清晰。
pub mod message;
/// 声明 processor 子模块，保持当前领域的职责拆分清晰。
pub mod processor;
/// 声明 prompt 子模块，保持当前领域的职责拆分清晰。
pub mod prompt;
/// 声明 retry 子模块，保持当前领域的职责拆分清晰。
pub mod retry;
/// 声明 revert 子模块，保持当前领域的职责拆分清晰。
pub mod revert;
/// 声明可供协议适配器和 Gateway 复用的会话运行时边界。
pub mod runtime;
/// 声明 session 子模块，保持当前领域的职责拆分清晰。
pub mod session;
/// 声明 status 子模块，保持当前领域的职责拆分清晰。
pub mod status;
/// 声明 store 子模块，提供原生全局数据库会话持久化。
#[cfg(not(target_arch = "wasm32"))]
pub mod store;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "store_tests.rs"]
mod store_tests;
/// 声明 summary 子模块，保持当前领域的职责拆分清晰。
pub mod summary;
/// 声明 system 子模块，保持当前领域的职责拆分清晰。
pub mod system;
/// 声明 title 子模块，保持当前领域的职责拆分清晰。
pub mod title;
/// 声明 todo 子模块，保持当前领域的职责拆分清晰。
pub mod todo;
/// 声明 ui_config 子模块，保持当前领域的职责拆分清晰。
pub mod ui_config;
/// 声明 ui_store 子模块，保持当前领域的职责拆分清晰。
pub mod ui_store;
/// 声明 ui_types 子模块，保持当前领域的职责拆分清晰。
pub mod ui_types;
