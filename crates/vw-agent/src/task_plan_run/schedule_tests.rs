//! 验证任务计划调度的依赖判断、就绪选择与顺序稳定性。
//! 批次按升序稳定分组，并在调度前拒绝自依赖、前向依赖和缺失批次。

use super::{PlanTaskEntry, build_batch_stages, parse_tasks_json};

fn entry(
    id: &str,
    title: &str,
    file: &str,
    batch: u32,
    depends_on_batches: Vec<u32>,
) -> PlanTaskEntry {
    PlanTaskEntry {
        id: id.to_string(),
        title: title.to_string(),
        file: file.to_string(),
        batch,
        depends_on_batches,
        status: "未开始".to_string(),
    }
}

#[test]
fn build_batch_stages_groups_tasks_by_ascending_batch() {
    let doc = parse_tasks_json(
        r#"{
            "plan_dir": ".plans/demo",
            "tasks": [
                {
                    "id": "1",
                    "title": "a",
                    "file": "1-a.md",
                    "batch": 1,
                    "depends_on_batches": [],
                    "status": "未开始"
                },
                {
                    "id": "2",
                    "title": "b",
                    "file": "2-b.md",
                    "batch": 1,
                    "depends_on_batches": [],
                    "status": "未开始"
                },
                {
                    "id": "3",
                    "title": "c",
                    "file": "3-c.md",
                    "batch": 2,
                    "depends_on_batches": [1],
                    "status": "未开始"
                },
                {
                    "id": "4",
                    "title": "d",
                    "file": "4-d.md",
                    "batch": 3,
                    "depends_on_batches": [1, 2],
                    "status": "未开始"
                }
            ]
        }"#,
    )
    .expect("valid tasks.json");

    let stages = build_batch_stages(&doc.tasks).expect("topology should be valid");
    assert_eq!(stages, vec![vec![0, 1], vec![2], vec![3]]);
}

#[test]
fn build_batch_stages_rejects_self_dependency() {
    let tasks = vec![entry("1", "self", "1-self.md", 1, vec![1])];
    let err = build_batch_stages(&tasks).expect_err("self dependency must fail");
    assert!(err.contains("自依赖") || err.contains("非更早批次"), "unexpected error: {err}");
}

#[test]
// 前向依赖在执行前直接拒绝，调度器不会等待一个尚未合法形成的未来批次。
fn build_batch_stages_rejects_forward_dependency() {
    let tasks = vec![entry("1", "a", "1-a.md", 1, vec![2]), entry("2", "b", "2-b.md", 2, vec![])];
    let err = build_batch_stages(&tasks).expect_err("forward dependency must fail");
    assert!(err.contains("非更早批次") || err.contains("前向"), "unexpected error: {err}");
}

#[test]
fn build_batch_stages_rejects_missing_dependency_batch() {
    let tasks = vec![entry("1", "a", "1-a.md", 2, vec![1])];
    let err = build_batch_stages(&tasks).expect_err("missing dependency batch must fail");
    assert!(err.contains("不存在"), "unexpected error: {err}");
}

#[test]
fn build_batch_stages_rejects_empty_tasks() {
    let err = build_batch_stages(&[]).expect_err("empty tasks must fail");
    assert!(err.contains("为空") || err.contains("空"), "unexpected error: {err}");
}
