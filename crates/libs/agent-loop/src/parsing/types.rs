/// 解析后的工具调用结构体
///
/// 表示从 LLM 响应中提取出的单个工具调用，
/// 包含工具名称、参数以及可选的工具调用标识符。
#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    /// 工具名称，如 "bash"、"file_read" 等。
    /// 兼容别名在解析阶段可能会收敛为内部 canonical 名称。
    pub name: String,
    /// 工具参数，以 JSON 值形式存储
    pub arguments: serde_json::Value,
    /// 工具调用的唯一标识符（可选）
    /// 某些 Provider（如 OpenAI）会为每个工具调用分配唯一 ID
    pub tool_call_id: Option<String>,
}
