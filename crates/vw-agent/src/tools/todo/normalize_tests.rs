//! 验证待办项归一化会清理字段、去重并稳定分配有效标识。

use super::*;

fn todo(id: &str, content: &str, status: &str, priority: &str) -> Todo {
    Todo {
        id: id.to_string(),
        content: content.to_string(),
        status: status.to_string(),
        priority: priority.to_string(),
    }
}

#[test]
fn normalize_trims_deduplicates_and_allocates_ids() {
    let todos = normalize::normalize_todos(vec![
        todo("1", " task ", "pending", ""),
        todo("1", "task", "completed", "high"),
        todo("", "new", " pending ", " medium "),
    ]);

    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0].content, "task");
    assert_eq!(todos[0].status, "completed");
    assert_eq!(todos[0].priority, "high");
    assert_eq!(todos[1].id, "2");
}

#[test]
fn normalize_reassigns_zero_and_non_numeric_ids_without_downgrading_duplicates() {
    let todos = normalize::normalize_todos(vec![
        todo("1", "stable", "completed", "high"),
        todo("9", "stable", "pending", "low"),
        todo("0", "zero", "pending", "medium"),
        todo("external", "text", "in_progress", "low"),
    ]);

    assert_eq!(todos.len(), 3);
    assert_eq!(todos[0].status, "completed");
    assert_eq!(todos[0].priority, "high");
    assert_eq!(todos[1].id, "2");
    assert_eq!(todos[2].id, "3");
}
