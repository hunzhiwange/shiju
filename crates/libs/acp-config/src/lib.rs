//! ACP 全局与项目配置及 MCP 服务器字段解析。

pub mod config;
pub mod mcp_servers;

pub use config::*;
pub use mcp_servers::*;

#[cfg(test)]
static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
