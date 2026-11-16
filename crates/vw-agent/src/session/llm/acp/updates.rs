//! ACP 输出消息到 VibeWindow 流式事件的转换工具。
//!
//! ACP 代理会以 JSON-RPC session/update 消息上报文本、推理、工具和 token 使用量。
//! 本模块只做结构解析和事件映射。远端工具更新通过只读生命周期事件呈现。

pub(crate) use llm_runtime::acp::updates::{extract_delta_from_acp_message, extract_reasoning_delta_from_acp_message, extract_tool_call_from_acp_message};
