//! 待办状态补丁的领域测试。

use todo_types as todo;

fn item(id: &str, content: &str, status: &str) -> todo::Todo {
    todo::Todo {
        id: id.to_string(),
        content: content.to_string(),
        status: status.to_string(),
        priority: "medium".to_string(),
    }
}

#[test]
fn build_todo_status_patches_updates_all_or_only_first_needed_item() {
    let items = vec![
        item("1", "completed", "completed"),
        item("2", "active", "in_progress"),
        item("3", "waiting", "pending"),
    ];

    let all = super::build_todo_status_patches(&items, "completed", false);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0], serde_json::json!({ "id": "2", "status": "completed" }));
    assert_eq!(all[1], serde_json::json!({ "id": "3", "status": "completed" }));

    let first = super::build_todo_status_patches(&items, "in_progress", true);
    assert!(first.is_empty());
}

#[test]
fn build_todo_status_patches_skips_completed_and_target_status_items() {
    let items = vec![
        item("1", "done", "completed"),
        item("2", "already", "pending"),
        item("3", "later", "in_progress"),
    ];

    let patches = super::build_todo_status_patches(&items, "pending", false);
    assert_eq!(patches, vec![serde_json::json!({ "id": "3", "status": "pending" })]);

    let only_first_stops_when_first_incomplete_already_has_target_status =
        super::build_todo_status_patches(&items, "pending", true);
    assert!(only_first_stops_when_first_incomplete_already_has_target_status.is_empty());

    let completed = vec![item("1", "done", "completed")];
    assert!(super::build_todo_status_patches(&completed, "completed", false).is_empty());
}

#[test]
fn build_todo_status_patches_returns_empty_for_empty_input() {
    assert!(super::build_todo_status_patches(&[], "completed", false).is_empty());
    assert!(super::build_todo_status_patches(&[], "in_progress", true).is_empty());
}

#[test]
fn build_todo_status_patches_only_updates_first_eligible_item() {
    let items = vec![
        item("1", "done", "completed"),
        item("2", "first", "pending"),
        item("3", "second", "pending"),
    ];

    let patches = super::build_todo_status_patches(&items, "in_progress", true);

    assert_eq!(patches, vec![serde_json::json!({ "id": "2", "status": "in_progress" })]);
}

/// 测试：仅将第一个未完成的 Todo 项状态更新为 in_progress
///
/// # 验证场景
/// - 列表包含 3 个 Todo 项：pending、in_progress、pending
/// - 调用 `build_todo_status_patches` 时传入 `only_first = true`
/// - 期望结果：仅生成 1 个补丁，针对第一个 pending 项（id=1）
///
/// # 业务逻辑
/// 当 `only_first = true` 时，函数应该跳过已处于 in_progress 的项，
/// 只将第一个未完成（pending）的项更新为目标状态。
#[test]
fn patches_only_first_incomplete_to_in_progress() {
    // 准备测试数据：3 个 Todo 项，中间一个已是 in_progress
    let items = vec![
        todo::Todo {
            content: "a".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
            id: "1".to_string(),
        },
        todo::Todo {
            content: "b".to_string(),
            status: "in_progress".to_string(),
            priority: "high".to_string(),
            id: "2".to_string(),
        },
        todo::Todo {
            content: "c".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
            id: "3".to_string(),
        },
    ];

    // 调用待测函数：仅更新第一个未完成项
    let patches = super::build_todo_status_patches(&items, "in_progress", true);

    // 断言：应该只生成 1 个补丁
    assert_eq!(patches.len(), 1);
    // 断言：补丁针对第一个 pending 项（id=1）
    assert_eq!(patches[0].get("id").and_then(|v| v.as_str()), Some("1"));
    // 断言：补丁将状态设置为 in_progress
    assert_eq!(patches[0].get("status").and_then(|v| v.as_str()), Some("in_progress"));
}

/// 测试：将所有未完成的 Todo 项状态更新为 completed
///
/// # 验证场景
/// - 列表包含 3 个 Todo 项：completed、in_progress、pending
/// - 调用 `build_todo_status_patches` 时传入 `only_first = false`
/// - 期望结果：生成 2 个补丁，分别针对 in_progress（id=2）和 pending（id=3）
///
/// # 业务逻辑
/// 当 `only_first = false` 时，函数应该将所有未完成（非 completed）的项
/// 都更新为目标状态。
#[test]
fn patches_all_incomplete_to_completed() {
    // 准备测试数据：3 个 Todo 项，第一个已完成
    let items = vec![
        todo::Todo {
            content: "a".to_string(),
            status: "completed".to_string(),
            priority: "high".to_string(),
            id: "1".to_string(),
        },
        todo::Todo {
            content: "b".to_string(),
            status: "in_progress".to_string(),
            priority: "high".to_string(),
            id: "2".to_string(),
        },
        todo::Todo {
            content: "c".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
            id: "3".to_string(),
        },
    ];

    // 调用待测函数：更新所有未完成项
    let patches = super::build_todo_status_patches(&items, "completed", false);

    // 断言：应该生成 2 个补丁
    assert_eq!(patches.len(), 2);
    // 断言：第一个补丁针对 in_progress 项（id=2）
    assert_eq!(patches[0].get("id").and_then(|v| v.as_str()), Some("2"));
    // 断言：第二个补丁针对 pending 项（id=3）
    assert_eq!(patches[1].get("id").and_then(|v| v.as_str()), Some("3"));
}
