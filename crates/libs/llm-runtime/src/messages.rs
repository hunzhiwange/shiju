use serde_json::Value;
pub fn has_tool_calls(messages: &[Value]) -> bool {
    for msg in messages {
        // 检查是否存在 tool_calls 数组且非空
        if msg.get("tool_calls").and_then(Value::as_array).is_some_and(|a| !a.is_empty()) {
            return true;
        }

        // 检查是否为 tool 响应消息（角色为 tool 且包含 tool_call_id）
        if msg.get("role").and_then(Value::as_str) == Some("tool")
            && msg.get("tool_call_id").and_then(Value::as_str).is_some()
        {
            return true;
        }

        // 检查消息内容是否为数组格式，并查找工具调用相关的类型
        let Some(content) = msg.get("content") else { continue };
        let Some(arr) = content.as_array() else { continue };

        for part in arr {
            let Some(ty) = part.get("type").and_then(Value::as_str) else { continue };
            // 检查是否为 tool-call 或 tool-result 类型
            if ty == "tool-call" || ty == "tool-result" {
                return true;
            }
        }
    }
    false
}
