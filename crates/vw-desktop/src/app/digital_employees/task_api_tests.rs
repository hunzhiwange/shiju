use serde_json::json;

use super::Detail;

#[test]
fn task_detail_accepts_both_artifact_size_fields() {
    let artifact = json!({
        "id": "artifact-1",
        "name": "报告.pdf",
        "content_type": "application/pdf",
        "byte_size": 2048,
        "size_bytes": 2048,
    });
    let detail: Detail = serde_json::from_value(json!({
        "task": { "task_id": "task-1", "title": "生成报告", "status": "SUCCEEDED" },
        "steps": [],
        "artifacts": [artifact.clone()],
        "deliverables": {
            "items": [artifact],
            "empty_state": "暂无交付文件",
        },
    }))
    .expect("task detail with both size fields should deserialize");

    assert_eq!(detail.task.title, "生成报告");
    assert_eq!(detail.artifacts[0].byte_size(), Some(2048));
    assert_eq!(detail.deliverables.unwrap().items[0].byte_size(), Some(2048));
}

#[test]
fn task_detail_accepts_legacy_artifact_size_field() {
    let detail: Detail = serde_json::from_value(json!({
        "task": { "task_id": "task-1", "title": "生成报告", "status": "SUCCEEDED" },
        "artifacts": [{ "name": "报告.pdf", "size_bytes": 2048 }],
    }))
    .expect("task detail with only the legacy size field should deserialize");

    assert_eq!(detail.artifacts[0].byte_size(), Some(2048));
}

#[test]
fn task_department_choices_keep_named_iam_paths() {
    for total in [json!(5), json!("5")] {
        let page = serde_json::from_value(json!({
            "list": [
                {"department_id": 101015, "department_path": "0,101015", "department_name": "综合支持"},
                {"department_id": "101016", "department_path": "0,101016", "department_name": "财务部"},
                {"department_path": "0,101015", "department_name": "综合支持"},
                {"department_path": "0,101017"},
                {"department_name": "无路径部门"}
            ],
            "total": total,
        }))
        .expect("the IAM department page accepts numeric and string totals");
        assert_eq!(
            super::department_choices(page),
            [
                ("0,101015".to_owned(), "综合支持".to_owned()),
                ("0,101016".to_owned(), "财务部".to_owned())
            ],
        );
    }
}
