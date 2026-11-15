//! Host specialization of shared client tools.
pub type ClientTool = tool_services::client_tool::ClientTool<super::context::AgentContextHost, crate::session::runtime::dynamic_calls::ClientToolHost>;
