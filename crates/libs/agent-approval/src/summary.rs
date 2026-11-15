/// 生成工具参数的简短可读摘要。
pub(super) fn summarize_args(args: &serde_json::Value) -> String {
    match args {
        serde_json::Value::Object(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(key, value)| {
                    let value = match value {
                        serde_json::Value::String(text) => truncate_for_summary(text, 80),
                        other => {
                            let rendered = other.to_string();
                            truncate_for_summary(&rendered, 80)
                        }
                    };
                    format!("{key}: {value}")
                })
                .collect();
            parts.join(", ")
        }
        other => {
            let rendered = other.to_string();
            truncate_for_summary(&rendered, 120)
        }
    }
}

fn truncate_for_summary(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() { format!("{truncated}…") } else { input.to_string() }
}
