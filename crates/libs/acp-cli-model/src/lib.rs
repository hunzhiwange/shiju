//! CLI 参数、启动计划、公共命令模型与版本解析。

pub mod cli;
pub mod cli_core;
pub mod cli_public;
pub mod version;

pub use cli_core::*;
pub use cli_public::*;

use acp_types::*;
use acp_config::ResolvedAcpxConfig;
use acp_agent_registry::list_built_in_agents;
use acp_access_policy::codex_compat::is_codex_invocation;
use acp_owner_process::QUEUE_OWNER_PROCESS_MARKER;
