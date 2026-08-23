//! 验证任务计划运行模型的默认值、序列化与进度契约。
//! 任务编号兼容整数输入，但进入运行模型后统一规范为字符串。

use super::parse_tasks_json;

#[test]
fn parse_tasks_json_normalizes_integer_task_ids_to_strings() {
    let document = parse_tasks_json(
        r#"{
            "plan_dir": ".plans/task-demo",
            "tasks": [
                {
                    "id": 1,
                    "title": "数字任务",
                    "file": "1-数字任务.md",
                    "batch": 1,
                    "depends_on_batches": [],
                    "status": "未开始"
                },
                {
                    "id": "task-2",
                    "title": "字符串任务",
                    "file": "2-字符串任务.md",
                    "batch": 2,
                    "depends_on_batches": [1],
                    "status": "未开始"
                }
            ]
        }"#,
    )
    .expect("integer and string task ids should both parse");

    assert_eq!(document.tasks[0].id, "1");
    assert_eq!(document.tasks[1].id, "task-2");
}
