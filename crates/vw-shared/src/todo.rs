//! 待办事项数据结构
//!
//! 定义 `Todo` 结构体及其自定义 serde 反序列化辅助函数。
//! 本模块为 vw-shared 层公共类型，供 vw-agent（工具执行）和 vw-desktop（UI 渲染）复用。
//!
//! # 兼容性说明
//!
//! 历史数据中的待办 `id` 字段可能同时出现字符串与数字格式，因此本模块提供了
//! 宽松的反序列化辅助函数，保证旧数据与新数据可以共存读取。

use serde::{Deserialize, Serialize};

/// 旧版待办结构别名，当前与 [`Todo`] 保持一致。
pub type LegacyTodo = Todo;

/// 单条待办事项的共享表示。
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Todo {
    pub content: String,
    #[serde(default = "default_todo_status")]
    pub status: String,
    #[serde(default = "default_todo_priority")]
    pub priority: String,
    #[serde(deserialize_with = "de_string_or_number")]
    pub id: String,
}

/// 将字符串或数字格式的字段统一反序列化为字符串。
pub fn de_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        _ => Err(serde::de::Error::custom("期望字符串或数字")),
    }
}

/// 将可空的字符串或数字字段统一反序列化为可选字符串。
pub fn de_opt_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => Ok(Some(s)),
        serde_json::Value::Number(n) => Ok(Some(n.to_string())),
        _ => Err(serde::de::Error::custom("期望字符串或数字")),
    }
}

/// 返回待办状态字段的默认值。
pub fn default_todo_status() -> String {
    "pending".to_string()
}

/// 返回待办优先级字段的默认值。
pub fn default_todo_priority() -> String {
    "medium".to_string()
}

#[cfg(test)]
#[path = "todo_tests.rs"]
mod todo_tests;
