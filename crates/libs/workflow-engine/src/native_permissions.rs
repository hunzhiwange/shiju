//! 原生工作流工具的会话能力范围检查。

use serde_json::Value;

pub fn check_allowed(name: &str, allowed: Option<&Value>) -> Result<(), String> {
    if let Some(allowed) = allowed {
        let allowed = allowed.as_array().ok_or("allowed_tools 必须是数组")?;
        if !allowed.iter().any(|value| value.as_str() == Some(name)) {
            return Err(format!("工作流工具不在当前会话能力范围内: {name}"));
        }
    }
    Ok(())
}
