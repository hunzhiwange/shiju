//! 会话待办服务，负责更新、读取并广播单个会话的待办列表。

use crate::app::agent::bus;
use crate::app::agent::project::instance;
use crate::app::agent::storage;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use vw_api_types::session::GatewaySessionTodoItem as Info;
use vw_api_types::todo::{TodoPriority, TodoStatus};

/// 声明 event 子模块，保持当前领域的职责拆分清晰。
pub mod event {
    use crate::app::agent::bus;

    /// UPDATED 定义当前模块共享的常量值。
    pub const UPDATED: bus::Definition = bus::Definition { r#type: "todo.updated" };
}

/// Error 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug)]
pub enum Error {
    Storage(storage::Error),
    Json(serde_json::Error),
    Other(anyhow::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Storage(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
            Error::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<storage::Error> for Error {
    fn from(value: storage::Error) -> Self {
        Error::Storage(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json(value)
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error::Other(value)
    }
}

/// UpdateInput 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInput {
    /// session_id 字段由调用方显式提供或读取，避免隐藏默认行为。
    #[serde(rename = "sessionID")]
    pub session_id: String,
    /// todos 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub todos: Vec<Info>,
}

fn todo_status_to_str(status: &TodoStatus) -> &'static str {
    match status {
        TodoStatus::Pending => "pending",
        TodoStatus::InProgress => "in_progress",
        TodoStatus::Completed => "completed",
        TodoStatus::Cancelled => "cancelled",
    }
}

fn todo_priority_to_str(priority: &TodoPriority) -> &'static str {
    match priority {
        TodoPriority::Low => "low",
        TodoPriority::Medium => "medium",
        TodoPriority::High => "high",
    }
}

fn todo_status_from_str(status: &str) -> TodoStatus {
    match status {
        "completed" => TodoStatus::Completed,
        "in_progress" => TodoStatus::InProgress,
        "cancelled" => TodoStatus::Cancelled,
        _ => TodoStatus::Pending,
    }
}

fn todo_priority_from_str(priority: &str) -> TodoPriority {
    match priority {
        "low" => TodoPriority::Low,
        "high" => TodoPriority::High,
        _ => TodoPriority::Medium,
    }
}

fn instance_directory_opt() -> Option<String> {
    let d = instance::directory();
    if d.is_empty() { None } else { Some(d) }
}

#[cfg(not(target_arch = "wasm32"))]
fn todo_storage_session_id(session_id: &str) -> String {
    instance::project()
        .map(|project| format!("{}:{session_id}", project.id))
        .unwrap_or_else(|| session_id.to_string())
}

/// 执行 update 操作，并返回调用方需要的结果。
pub async fn update(input: UpdateInput) -> Result<(), Error> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let storage_session_id = todo_storage_session_id(&input.session_id);
        let tool_todos = input
            .todos
            .iter()
            .map(|todo| crate::app::agent::tools::todo::Todo {
                content: todo.content.clone(),
                status: todo_status_to_str(&todo.status).to_string(),
                priority: todo_priority_to_str(&todo.priority).to_string(),
                id: todo.id.clone(),
            })
            .collect::<Vec<_>>();
        crate::app::agent::tools::todo::write_session_todos(&storage_session_id, tool_todos)?;
    }

    #[cfg(target_arch = "wasm32")]
    {
        storage::write(&["todo", &input.session_id], &input.todos).await?;
    }

    let _ = bus::publish(
        event::UPDATED,
        json!({ "sessionID": input.session_id, "todos": input.todos }),
        instance_directory_opt(),
    );
    Ok(())
}

/// 执行 get 操作，并返回调用方需要的结果。
pub async fn get(session_id: &str) -> Vec<Info> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let storage_session_id = todo_storage_session_id(session_id);
        return crate::app::agent::tools::todo::read_session_todos(&storage_session_id)
            .into_iter()
            .map(|todo| Info {
                id: todo.id,
                content: todo.content,
                status: todo_status_from_str(&todo.status),
                priority: todo_priority_from_str(&todo.priority),
            })
            .collect();
    }

    #[cfg(target_arch = "wasm32")]
    {
        storage::read::<Vec<Info>>(&["todo", session_id]).await.unwrap_or_default()
    }
}
#[cfg(test)]
#[path = "todo_tests.rs"]
mod todo_tests;
