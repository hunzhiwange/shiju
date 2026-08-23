//! 测试 PostgreSQL 任务存储的状态转换、筛选与元数据更新。

use ::postgres::types::ToSql;

use super::{ensure_single_update, push_filter};
use crate::agent_scheduler::domain::TaskId;

#[test]
fn push_filter_appends_value_and_uses_next_parameter_index() {
    let mut query = "SELECT id FROM scheduler_tasks WHERE platform_id = $1".to_owned();
    let mut values: Vec<Box<dyn ToSql + Sync>> = vec![Box::new("platform-a".to_owned())];

    push_filter(&mut query, &mut values, "status", "running".to_owned());
    push_filter(&mut query, &mut values, "assigned_to", "worker-a".to_owned());

    assert_eq!(
        query,
        "SELECT id FROM scheduler_tasks WHERE platform_id = $1 AND status = $2 AND assigned_to = $3"
    );
    assert_eq!(values.len(), 3);
}

#[test]
fn ensure_single_update_accepts_exactly_one_changed_row() {
    let task_id = TaskId::new("task-1").expect("valid task id");

    assert!(ensure_single_update(1, &task_id, "status transition").is_ok());
}

#[test]
fn ensure_single_update_rejects_non_single_row_counts_with_context() {
    let task_id = TaskId::new("task-1").expect("valid task id");

    for changed in [0, 2] {
        let error = ensure_single_update(changed, &task_id, "status transition")
            .expect_err("non-single update must fail");

        assert_eq!(
            error.to_string(),
            format!("PostgreSQL task status transition affected {changed} rows: task-1")
        );
    }
}
