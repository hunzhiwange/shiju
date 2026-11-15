use std::collections::{HashMap, HashSet};

use super::Todo;

/// 获取状态的排序权重
///
/// 用于对待办事项按状态排序：
/// - completed（已完成）: 3
/// - in_progress（进行中）: 2
/// - pending（待处理）: 1
/// - 其他: 0
fn status_rank(status: &str) -> u8 {
    match status {
        "completed" => 3,
        "in_progress" => 2,
        "pending" => 1,
        _ => 0,
    }
}

/// 解析数字 ID
///
/// 尝试将字符串解析为无符号 64 位整数。
/// 空字符串或 0 返回 None（0 被视为无效 ID）。
fn parse_numeric_id(id: &str) -> Option<u64> {
    let s = id.trim();
    if s.is_empty() {
        return None;
    }
    let n = s.parse::<u64>().ok()?;
    if n == 0 { None } else { Some(n) }
}

/// 分配新的数字 ID
///
/// 从 1 开始寻找第一个未被使用的数字作为新 ID。
fn alloc_numeric_id(used: &mut HashSet<u64>) -> String {
    let mut n = 1u64;
    while used.contains(&n) {
        n = n.saturating_add(1);
    }
    used.insert(n);
    n.to_string()
}

/// 规范化待办事项列表
///
/// 执行以下处理步骤：
/// 1. 去除所有字符串字段的首尾空白
/// 2. 按内容去重：相同内容的待办项合并，保留最高状态
/// 3. 确保 ID 唯一性：重复 ID 重新分配
/// 4. 为缺失 ID 的项自动分配新 ID
pub(super) fn normalize_todos(mut todos: Vec<Todo>) -> Vec<Todo> {
    for todo in &mut todos {
        todo.content = todo.content.trim().to_string();
        todo.status = todo.status.trim().to_string();
        todo.priority = todo.priority.trim().to_string();
        todo.id = todo.id.trim().to_string();
    }

    let mut by_content: Vec<Todo> = Vec::new();
    let mut index_by_content: HashMap<String, usize> = HashMap::new();
    for todo in todos {
        let key = todo.content.clone();
        if let Some(&idx) = index_by_content.get(&key) {
            let existing = &mut by_content[idx];
            if status_rank(&todo.status) > status_rank(&existing.status) {
                existing.status = todo.status;
            }
            if existing.priority.is_empty() && !todo.priority.is_empty() {
                existing.priority = todo.priority;
            }
            continue;
        }
        index_by_content.insert(key, by_content.len());
        by_content.push(todo);
    }

    let mut used: HashSet<u64> = HashSet::new();
    for todo in &mut by_content {
        let Some(n) = parse_numeric_id(&todo.id) else {
            todo.id.clear();
            continue;
        };
        if used.contains(&n) {
            todo.id.clear();
            continue;
        }
        used.insert(n);
        todo.id = n.to_string();
    }

    for todo in &mut by_content {
        if todo.id.trim().is_empty() {
            todo.id = alloc_numeric_id(&mut used);
        }
    }

    by_content
}
