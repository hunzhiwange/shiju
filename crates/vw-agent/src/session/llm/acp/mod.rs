//! ACP 会话适配层的聚合模块。
//!
//! 该模块把 VibeWindow 内部的 LLM 流式请求转换为 Agent Client Protocol
//! (ACP) 客户端调用，并集中暴露配置解析、历史重放、会话缓存和更新转换等窄接口。
//! 这里不直接承载传输细节，而是把具体职责拆到相邻子模块中，便于请求流程保持清晰。

#[allow(unused_imports)]
pub(crate) use config::{
    build_acp_command_line, lookup_acp_command, normalize_acp_agent_config, parse_acp_options,
};
#[allow(unused_imports)]
pub(crate) use replay::{build_replay_prompt, parse_replay_strategy};
#[allow(unused_imports)]
pub(crate) use request::do_stream_request_acp;
#[allow(unused_imports)]
pub(crate) use session::{acp_session_name, missing_session_error};
#[allow(unused_imports)]
pub(crate) use updates::{
    extract_delta_from_acp_message, extract_reasoning_delta_from_acp_message,
    extract_tool_call_from_acp_message,
};




use super::types::{Error, StreamEvent};

mod approval;
mod config;
pub(crate) mod controls;
mod fork_replay;
pub(super) mod lifecycle;
mod model_source;
// ACP recovery is owned and registered by llm-runtime::acp.
mod replay;
pub(super) mod request;
mod session;
mod skill_mounts;
mod updates;

#[cfg(test)]
use llm_runtime::acp::cache::{ACP_CLIENT_CACHE};

/// ACP 客户端缓存命中结果。
///
/// `key` 用于在一次性请求结束后安全移除对应缓存项，`client` 是实际共享客户端。
#[cfg(test)]
use llm_runtime::acp::session::CachedAcpClientEntry;

/// 从运行时选项解析出的 ACP 会话参数。
///
/// 字段保持为 `Option`，让调用方能区分“用户未配置”和“明确配置为空”的场景。
pub(super) type ParsedAcpOptions = llm_runtime::acp::options::ParsedAcpOptions<skill_mounts::MountedSkills>;
pub(super) use llm_runtime::acp::options::AcpReplayStrategy;

pub(crate) mod source_selection;
