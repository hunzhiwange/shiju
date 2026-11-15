//! # MCP（模型上下文协议）服务模块
//!
//! 本模块实现了 MCP（Model Context Protocol）服务器，用于将 VibeWindow 代理工具
//! 通过标准化协议暴露给外部客户端。
//!
//! ## 主要功能
//!
//! - 将代理工具注册为 MCP 工具供客户端发现和调用
//! - 提供基于标准输入/输出的传输层支持
//! - 处理工具列表查询和工具调用请求
//!
//! ## 架构说明
//!
//! 模块核心是 [`AgentToolServer`] 结构体，它实现了 `rmcp` 库的 `ServerHandler` trait，
//! 提供以下能力：
//! - 服务器信息查询（`get_info`）
//! - 工具列表查询（`list_tools`）
//! - 工具调用执行（`call_tool`）

use tool_context::ToolCallError;
use tool_types::ToolCallResult as AgentToolCallResult;
use vw_api_tools::ToolSpecDto;

/// 宿主提供真实工具注册表和带权限上下文的同步执行入口。
pub trait McpToolRuntime: Send + Sync + 'static {
    fn spec_dtos(&self) -> Vec<ToolSpecDto>;
    fn execute_tool_call(&self, name: &str, input: &str)
        -> Result<AgentToolCallResult, ToolCallError>;
}
use rmcp::ErrorData as McpError;
use rmcp::ServiceExt;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, Implementation, ListToolsResult,
    PaginatedRequestParam, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::io::stdio;
use serde_json::{Map, Value};
use std::sync::Arc;

/// MCP 工具服务器
///
/// 该结构体是将 VibeWindow 代理工具暴露给 MCP 客户端的核心服务。
/// 它持有工具执行所需的上下文，并实现了 MCP 协议的服务器端处理逻辑。
///
#[derive(Clone)]
pub struct AgentToolServer {
    /// 工具执行上下文，使用 `Arc` 实现共享所有权
    /// 以支持在多个并发请求中安全地访问上下文
    ctx: Arc<dyn McpToolRuntime>,
}

impl AgentToolServer {
    /// 创建新的 MCP 工具服务器实例
    ///
    /// # 参数
    ///
    /// * `ctx` - 工具执行上下文，包含工具运行所需的配置和状态信息
    ///
    /// # 返回值
    ///
    /// 返回一个新的 `AgentToolServer` 实例
    ///
    /// # 示例
    ///
    pub fn new(ctx: impl McpToolRuntime) -> Self {
        Self { ctx: Arc::new(ctx) }
    }

    /// 通过标准输入/输出启动 MCP 服务
    ///
    /// 该方法使用标准输入（stdin）和标准输出（stdout）作为传输层，
    /// 启动 MCP 服务器并等待客户端连接。这是最常见的 MCP 服务部署方式，
    /// 允许与支持 stdio 通信的各种客户端集成。
    ///
    /// # 返回值
    ///
    /// - `Ok(())` - 服务正常结束（包括客户端正常关闭连接）
    /// - `Err(...)` - 服务启动或运行过程中发生错误
    ///
    /// # 错误处理
    ///
    /// 方法会特别处理连接关闭的情况：
    /// - 当错误消息包含 "connectionclosed" 或 "connection closed" 时，
    ///   视为正常结束并返回 `Ok(())`
    /// - 其他错误将正常返回 `Err`
    ///
    /// # 示例
    ///
    pub async fn serve_stdio(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 创建标准输入/输出传输层
        let transport = stdio();

        // 尝试启动服务，处理可能的连接关闭错误
        let server = match self.serve(transport).await {
            Ok(server) => server,
            Err(e) => {
                // 将错误消息转换为小写以便不区分大小写匹配
                let msg = e.to_string().to_lowercase();
                // 连接关闭视为正常结束，不作为错误处理
                if msg.contains("connectionclosed") || msg.contains("connection closed") {
                    return Ok(());
                }
                return Err(e.into());
            }
        };

        // 等待服务完成，同样处理连接关闭的情况
        match server.waiting().await {
            Ok(_) => {}
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                // 连接关闭视为正常结束，不作为错误处理
                if msg.contains("connectionclosed") || msg.contains("connection closed") {
                    return Ok(());
                }
                return Err(e.into());
            }
        }
        Ok(())
    }
}

fn mcp_tools_from_registry(ctx: &dyn McpToolRuntime) -> Vec<Tool> {
    ctx.spec_dtos()
        .into_iter()
        .map(|s| {
            let schema = match s.input_schema {
                Value::Object(m) => m,
                _ => Map::new(),
            };
            Tool::new(s.id.0, s.description, Arc::new(schema))
        })
        .collect()
}

fn serialize_tool_arguments(arguments: Option<Map<String, Value>>) -> String {
    let args = arguments.unwrap_or_default();
    serde_json::to_string(&Value::Object(args)).unwrap_or_else(|_| "{}".to_string())
}

fn call_tool_with_context(
    name: &str,
    arguments: Option<Map<String, Value>>,
    ctx: &dyn McpToolRuntime,
) -> CallToolResult {
    let input = serialize_tool_arguments(arguments);
    mcp_call_tool_result(ctx.execute_tool_call(name, &input))
}

fn mcp_call_tool_result(
    result: Result<AgentToolCallResult, ToolCallError>,
) -> CallToolResult {
    match result {
        Ok(r) => CallToolResult::success(vec![Content::text(r.model_text())]),
        Err(e) => CallToolResult::error(vec![Content::text(format!("{:?}", e))]),
    }
}

/// MCP 服务器处理器实现
///
/// 为 `AgentToolServer` 实现 `rmcp::ServerHandler` trait，
/// 提供 MCP 协议所需的服务器端处理能力。
impl ServerHandler for AgentToolServer {
    /// 获取服务器信息
    ///
    /// 返回 MCP 协议要求的服务器元数据，包括：
    /// - 服务器名称（"vibe-window"）
    /// - 版本号（来自 Cargo.toml）
    /// - 使用说明
    ///
    /// # 返回值
    ///
    /// 返回包含服务器基本信息的 `ServerInfo` 结构体
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "vibe-window".to_string(),
                title: None,
                // 使用编译时环境变量获取包版本
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            // 提供简短的服务说明
            instructions: Some("Expose vibe-window agent tools over MCP.".to_string()),
            ..Default::default()
        }
    }

    /// 列出可用工具
    ///
    /// 响应客户端的工具列表查询请求，返回所有已注册的代理工具信息。
    /// 每个工具包含名称、描述和参数 schema。
    ///
    /// # 参数
    ///
    /// * `_request` - 分页请求参数（当前未使用）
    /// * `_context` - 请求上下文（当前未使用）
    ///
    /// # 返回值
    ///
    /// 返回包含工具列表的异步 Future，成功时返回 `ListToolsResult`
    ///
    /// # 实现细节
    ///
    /// 1. 调用宿主的 `McpToolRuntime::spec_dtos` 获取默认对外工具规范
    /// 2. 将每个工具规范转换为 MCP 工具格式
    /// 3. 处理参数 schema，确保其为有效的 JSON 对象
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            // 返回工具列表结果
            Ok(ListToolsResult { tools: mcp_tools_from_registry(self.ctx.as_ref()), ..Default::default() })
        }
    }

    /// 执行工具调用
    ///
    /// 响应客户端的工具调用请求，执行指定的代理工具并返回结果。
    ///
    /// # 参数
    ///
    /// * `request` - 工具调用请求，包含工具名称和参数
    /// * `_context` - 请求上下文（当前未使用）
    ///
    /// # 返回值
    ///
    /// 返回包含执行结果的异步 Future，成功时返回 `CallToolResult`：
    /// - 工具执行成功：返回包含输出文本的成功结果
    /// - 工具执行失败：返回包含错误信息的错误结果
    ///
    /// # 实现细节
    ///
    /// 1. 从请求中提取工具名称和参数
    /// 2. 将参数转换为 JSON 字符串格式
    /// 3. 调用工具执行引擎运行工具
    /// 4. 根据执行结果构造返回内容
    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let name = request.name;
            let arguments = request.arguments;
            Ok(call_tool_with_context(name.as_ref(), arguments, self.ctx.as_ref()))
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
