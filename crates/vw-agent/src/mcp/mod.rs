//! MCP 服务组合：协议处理由 agent-mcp 提供，宿主保留工具工厂接线。
pub struct AgentToolServer;
impl AgentToolServer {
    pub fn new(context: tools::ToolRuntimeContext) -> agent_mcp::AgentToolServer { agent_mcp::AgentToolServer::new(NativeRuntime(context)) }
}
struct NativeRuntime(tools::ToolRuntimeContext);
use agent_mcp::McpToolRuntime;
use crate::tools;

impl McpToolRuntime for NativeRuntime {
    fn spec_dtos(&self) -> Vec<vw_api_tools::ToolSpecDto> {
        tools::registry::spec_dtos(None)
    }

    fn execute_tool_call(&self, name: &str, input: &str)
        -> Result<tools::ToolCallResult, tools::ToolCallError>
    {
        tools::execute_tool_call(name, input, &self.0)
    }
}

#[cfg(test)]
mod tests;
