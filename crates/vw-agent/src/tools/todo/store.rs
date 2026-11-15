use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::MutexGuard;
use vw_shared::todo::{default_todo_priority, default_todo_status};

use super::Todo;
use super::normalize::normalize_todos;
use super::schema::{TodoInput, TodoPatch, WriteArgs};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::project::instance;

/// 全局待办事项存储
///
/// 使用互斥锁保护的哈希表，键为会话标识，值为该会话的待办事项列表。
static TODOS: LazyLock<Mutex<HashMap<String, Vec<Todo>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) fn recover_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            let guard = poisoned.into_inner();
            mutex.clear_poison();
            guard
        }
    }
}

fn storage_key(session: &str, scope: Option<&str>) -> String {
    match scope.map(str::trim).filter(|value| !value.is_empty()) {
        Some(scope) => format!("{scope}\0{session}"),
        None => session.to_string(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn scoped_project_id() -> Option<String> {
    instance::project().map(|info| info.id)
}

#[cfg(target_arch = "wasm32")]
fn scoped_project_id() -> Option<String> {
    None
}

fn load_todos(session: &str, scope: Option<&str>) -> Vec<Todo> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return crate::session::ui_store::load_session_todos_scoped(session, scope)
            .into_iter()
            .map(|todo| Todo {
                content: todo.content,
                status: todo.status,
                priority: todo.priority,
                id: todo.id,
            })
            .collect();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let Some(path) = ProjectDirs::from("dev", "VibeWindow", "vibe-window")
            .map(|d| d.data_local_dir().join("todos").join(format!("{}.json", session)))
        else {
            return vec![];
        };
        let Ok(content) = std::fs::read_to_string(path) else {
            return vec![];
        };
        serde_json::from_str::<Vec<Todo>>(&content).unwrap_or_default()
    }
}

fn save_todos(session: &str, todos: &[Todo], scope: Option<&str>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mapped = todos
            .iter()
            .map(|todo| crate::session::ui_types::SessionTodoItem {
                content: todo.content.clone(),
                status: todo.status.clone(),
                priority: todo.priority.clone(),
                id: todo.id.clone(),
            })
            .collect::<Vec<_>>();
        let _ = crate::session::ui_store::save_session_todos_scoped(session, &mapped, scope);
    }

    #[cfg(target_arch = "wasm32")]
    {
        let Some(path) = ProjectDirs::from("dev", "VibeWindow", "vibe-window")
            .map(|d| d.data_local_dir().join("todos").join(format!("{}.json", session)))
        else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let content = serde_json::to_string(todos).unwrap_or_else(|_| "[]".to_string());
        let _ = std::fs::write(path, content);
    }
}

/// 供 UI 层使用的待办事项读取接口
pub(super) fn read_for_ui(session: &str) -> Vec<Todo> {
    read_for_ui_scoped(session, scoped_project_id().as_deref())
}

fn read_for_ui_scoped(session: &str, scope: Option<&str>) -> Vec<Todo> {
    let key = storage_key(session, scope);
    let mut lock = recover_lock(&TODOS);
    if !lock.contains_key(&key) {
        lock.insert(key.clone(), normalize_todos(load_todos(session, scope)));
    }
    lock.get(&key).cloned().unwrap_or_default()
}

pub(super) fn read_for_tool(session: &str) -> anyhow::Result<Vec<Todo>> {
    read_for_tool_scoped(session, scoped_project_id().as_deref())
}

pub(super) fn read_for_tool_scoped(
    session: &str,
    scope: Option<&str>,
) -> anyhow::Result<Vec<Todo>> {
    let key = storage_key(session, scope);
    let mut lock = recover_lock(&TODOS);
    if !lock.contains_key(&key) {
        lock.insert(key.clone(), normalize_todos(load_todos(session, scope)));
    }

    let todos = lock.get(&key).cloned().unwrap_or_default();
    let normalized = normalize_todos(todos);
    lock.insert(key, normalized.clone());
    save_todos(session, &normalized, scope);
    Ok(normalized)
}

/// 核心写入逻辑：更新或替换待办事项列表
pub(super) fn write_todos(session: &str, args: serde_json::Value) -> anyhow::Result<String> {
    write_todos_scoped(session, args, scoped_project_id().as_deref())
}

pub(super) fn write_todos_scoped(
    session: &str,
    args: serde_json::Value,
    scope: Option<&str>,
) -> anyhow::Result<String> {
    let args: WriteArgs = serde_json::from_value(args)?;
    let key = storage_key(session, scope);

    let mut lock = recover_lock(&TODOS);
    let mut current = lock.get(&key).cloned().unwrap_or_else(|| load_todos(session, scope));
    current = normalize_todos(current);

    let next = if args.merge {
        let patches = args
            .todos
            .iter()
            .cloned()
            .map(|value| {
                serde_json::from_value::<TodoPatch>(value).map_err(|e| anyhow!(e.to_string()))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        for patch in patches {
            let patch_content = patch.content.as_deref().map(|s| s.trim().to_string());
            let patch_id = patch.id.as_deref().map(|s| s.trim().to_string());

            let mut target_idx = patch_content
                .as_deref()
                .and_then(|content| current.iter().position(|todo| todo.content == content));
            if target_idx.is_none() {
                target_idx = patch_id
                    .as_deref()
                    .and_then(|id| current.iter().position(|todo| todo.id == id));
            }

            if let Some(idx) = target_idx {
                if let Some(content) = patch_content.clone() {
                    current[idx].content = content;
                }
                if let Some(status) = patch.status {
                    current[idx].status = status.trim().to_string();
                }
                if let Some(priority) = patch.priority {
                    current[idx].priority = priority.trim().to_string();
                }
                continue;
            }

            let Some(content) = patch_content else { continue };
            let status = patch.status.unwrap_or_else(default_todo_status);
            let priority = patch.priority.unwrap_or_else(default_todo_priority);
            let mut next_todos =
                vec![Todo { id: patch_id.unwrap_or_default(), content, status, priority }];
            next_todos.extend(current);
            current = normalize_todos(next_todos);
        }

        normalize_todos(current)
    } else {
        let parsed = args
            .todos
            .iter()
            .cloned()
            .map(|value| {
                let todo_input = serde_json::from_value::<TodoInput>(value)
                    .map_err(|e| anyhow!(e.to_string()))?;
                Ok::<Todo, anyhow::Error>(Todo {
                    id: todo_input.id.unwrap_or_default(),
                    content: todo_input.content,
                    status: todo_input.status,
                    priority: todo_input.priority,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        normalize_todos(parsed)
    };

    lock.insert(key, next.clone());
    save_todos(session, &next, scope);
    serde_json::to_string_pretty(&next).map_err(|e| anyhow!(e))
}
