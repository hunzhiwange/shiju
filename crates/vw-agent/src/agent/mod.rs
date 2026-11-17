//! # Agent 模块
//!
//! 本模块是 VibeWindow 代理系统的核心编排层，负责代理的定义、加载和管理。
//!
//! ## 主要功能
//!
//! - **代理信息管理**：定义和存储各种代理的配置信息
//! - **状态加载**：从配置文件加载代理状态，包括内置代理和用户自定义代理
//! - **权限控制**：为每个代理配置细粒度的权限规则
//! - **默认代理选择**：智能选择合适的默认代理
//!
//! ## 模块结构
//!
//! - `agent`：核心代理实现，包含 Agent 结构体和 AgentBuilder
//! - `classifier`：消息分类器，用于确定消息类型
//! - `dispatcher`：任务调度器，负责任务分发
//! - `loop_`：主循环处理，包含消息处理和运行逻辑
//! - `memory_loader`：记忆加载器
//! - `prompt`：提示词管理
//!
//! ## 内置代理
//!
//! 本模块预定义了以下内置代理：
//! - `main`：唯一交互执行代理
//! - `researcher`、`coder`、`reviewer`、`explore`：专业委托代理

#[allow(clippy::module_inception)]
pub mod agent;
pub use agent_config::catalog;
#[cfg(test)]
use agent_config::catalog::load_agent_configs;
pub use agent_loop::classifier;
pub use agent_loop::defaults;
pub use agent_loop::dispatcher;
pub mod loop_;
pub use agent_loop::memory_loader;
pub mod prompt;

#[cfg(test)]
#[path = "../../../libs/agent-loop/src/tests/agent_host_suite/mod.rs"]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub use agent::run;
#[allow(unused_imports)]
pub use agent::{Agent, AgentBuilder};
#[allow(unused_imports)]
pub use loop_::process_message;

pub use agent_loop::definitions::{Info, generate_prompt, init, resolve_model_ref};

struct DefinitionHost;

#[async_trait::async_trait]
impl agent_loop::definitions::DefinitionHost for DefinitionHost {
    async fn agent_configs() -> Result<std::collections::HashMap<String, config_agent::DelegateAgentConfig>, String> {
        Ok(crate::config::get_current().await.map_err(|error| error.to_string())?.agents.clone())
    }

    async fn app_config() -> Result<serde_json::Value, String> {
        let tenant = crate::session::ui_store::current_tenant_scope();
        tokio::task::spawn_blocking(move || crate::session::ui_config::load_app_config_result(&tenant))
            .await.map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())
    }

    async fn skill_dirs() -> Vec<String> {
        crate::skill::dirs().await
    }

    fn data_dir() -> std::path::PathBuf {
        crate::global::paths().data.clone()
    }
}

pub async fn get(name: &str) -> Option<config_agent::DelegateAgentConfig> {
    agent_loop::definitions::get::<DefinitionHost>(name).await
}

pub async fn default_agent() -> Result<String, String> {
    agent_loop::definitions::default_agent::<DefinitionHost>().await
}

pub async fn list() -> Vec<Info> {
    agent_loop::definitions::list::<DefinitionHost>().await
}

pub async fn permission_rules(name: &str) -> Option<permission_runtime::next::Ruleset> {
    agent_loop::definitions::permission_rules::<DefinitionHost>(name).await
}

pub(crate) async fn native_permission_ceiling(name: &str) -> Result<permission_runtime::next::Ruleset, String> {
    agent_loop::definitions::native_permission_ceiling::<DefinitionHost>(name).await
}
