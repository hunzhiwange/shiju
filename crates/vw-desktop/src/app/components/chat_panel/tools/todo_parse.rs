//! 从本次调用的结果补全增量待办参数，避免历史卡片依赖当前会话状态。

use serde_json::Value;

use super::tool_parse::{tool_input, tool_output_text, tool_result_data, tool_status};

pub(crate) fn todowrite_display_todos(value: &Value) -> Vec<Value> {
    let input = serde_json::from_str::<Value>(tool_input(value)).unwrap_or_default();
    let mut todos = input.get("todos").and_then(Value::as_array).cloned().unwrap_or_default();
    if !matches!(tool_status(value), "completed" | "success") {
        return todos;
    }

    let snapshot = tool_result_data(value)
        .and_then(|data| data.get("newTodos"))
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            tool_output_text(value)
                .and_then(|output| serde_json::from_str::<Vec<Value>>(&output).ok())
        })
        .unwrap_or_default();

    for todo in &mut todos {
        let content = todo.get("content").and_then(Value::as_str);
        // 与存储层一致：先按正文匹配，再按 ID 匹配；数字 ID 会被存储层转为字符串。
        let matched = content
            .and_then(|content| {
                snapshot.iter().find(|item| {
                    item.get("content").and_then(Value::as_str) == Some(content.trim())
                })
            })
            .or_else(|| {
                let id = todo_id(todo.get("id")?)?;
                snapshot
                    .iter()
                    .find(|item| item.get("id").and_then(todo_id).as_deref() == Some(id.as_str()))
            });
        if let Some(matched) = matched
            && let Some(todo) = todo.as_object_mut()
        {
            for field in ["content", "status"] {
                if !todo.contains_key(field)
                    && let Some(value) = matched.get(field)
                {
                    todo.insert(field.to_string(), value.clone());
                }
            }
        }
    }
    todos
}

fn todo_id(value: &Value) -> Option<String> {
    match value {
        Value::String(id) => Some(id.trim().to_string()),
        Value::Number(id) => Some(id.to_string()),
        _ => None,
    }
}
