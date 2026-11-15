//! Workflow 应用列表视图测试模块，覆盖列表状态、文案回退、时间格式化与样式分支。

use super::*;
use crate::apps::workflow::state::{
    WorkflowRunErrorPreview, WorkflowRunNodePreview, WorkflowRunPreview, WorkflowRunStatus,
    WorkflowSavedAppSummary,
};
use iced::widget::button;
use iced::{Background, Theme};

#[test]
fn workflow_run_preview_builds_running_success_and_error_states() {
    let mut running = run_preview(WorkflowRunStatus::Running);
    running.nodes.push(run_node("start", "开始", Some("running")));
    running.text_output = "正在分析输入".to_string();
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &running,
        false,
    );
    assert!(matches!(
        saved_app_run_preview_action(crate::app::i18n::I18n::new(Default::default()), &running),
        Some(("取消运行", WorkflowMessage::CancelSavedAppRun))
    ));
    assert_eq!(
        workflow_run_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            WorkflowRunStatus::Running
        ),
        "运行中"
    );
    assert_eq!(
        workflow_run_node_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            Some("running")
        ),
        "运行中"
    );

    let mut succeeded = run_preview(WorkflowRunStatus::Succeeded);
    succeeded.run_id = Some("run-1".to_string());
    succeeded.is_finished = true;
    succeeded.nodes.push(run_node("answer", "回答", Some("succeeded")));
    succeeded.text_output = "完成输出".to_string();
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &succeeded,
        false,
    );
    assert!(matches!(
        saved_app_run_preview_action(crate::app::i18n::I18n::new(Default::default()), &succeeded),
        Some(("重新执行", WorkflowMessage::RetrySavedAppRunPreview))
    ));
    assert_eq!(
        workflow_run_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            WorkflowRunStatus::Succeeded
        ),
        "成功"
    );
    assert_eq!(
        workflow_run_node_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            Some("succeeded")
        ),
        "成功"
    );

    let mut failed = run_preview(WorkflowRunStatus::Failed);
    failed.is_finished = true;
    failed.nodes.push(run_node("agent", "Agent", Some("failed")));
    failed.error = Some(WorkflowRunErrorPreview {
        status: 500,
        code: "workflow_run_error".to_string(),
        message: "节点执行失败".to_string(),
    });
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &failed,
        false,
    );
    assert!(matches!(
        saved_app_run_preview_action(crate::app::i18n::I18n::new(Default::default()), &failed),
        Some(("重新执行", WorkflowMessage::RetrySavedAppRunPreview))
    ));
    assert_eq!(
        workflow_run_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            WorkflowRunStatus::Failed
        ),
        "失败"
    );
    assert_eq!(
        workflow_run_node_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            Some("failed")
        ),
        "失败"
    );
    assert!(matches!(
        saved_app_run_preview_close_message(),
        WorkflowMessage::CloseSavedAppRunPreview
    ));

    let mut canceled = run_preview(WorkflowRunStatus::Canceled);
    canceled.is_finished = true;
    canceled.nodes.push(run_node("cancel", "取消节点", Some("canceled")));
    // Unknown status falls back to the warning kind, waiting (no status) stays muted,
    // and an empty-detail node must still build a stable row.
    canceled.nodes.push(run_node("weird", "未知节点", Some("mystery")));
    canceled.nodes.push(WorkflowRunNodePreview {
        id: "pending".to_string(),
        title: "等待节点".to_string(),
        status: None,
        data: serde_json::json!({}),
    });
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &canceled,
        false,
    );
    assert!(
        saved_app_run_preview_action(crate::app::i18n::I18n::new(Default::default()), &canceled)
            .is_none()
    );

    // Node status labels cover every timeline state.
    assert_eq!(
        workflow_run_node_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            Some("canceled")
        ),
        "已取消"
    );
    assert_eq!(
        workflow_run_node_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            Some("mystery")
        ),
        "未知"
    );
    assert_eq!(
        workflow_run_node_status_label(crate::app::i18n::I18n::new(Default::default()), None),
        "等待中"
    );
    assert_eq!(
        workflow_run_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            WorkflowRunStatus::Canceled
        ),
        "已取消"
    );

    // An empty-detail node produces no detail rows but still has a stable placeholder.
    let empty_node = WorkflowRunNodePreview {
        id: "pending".to_string(),
        title: "等待节点".to_string(),
        status: None,
        data: serde_json::json!({}),
    };
    assert!(
        workflow_run_node_data_preview_items(
            crate::app::i18n::I18n::new(Default::default()),
            &empty_node
        )
        .is_empty()
    );

    // A freshly started run with no run id and no nodes must still build: the summary
    // falls back to a placeholder run id and the timeline shows the waiting placeholder.
    let pending = run_preview(WorkflowRunStatus::Running);
    assert!(pending.nodes.is_empty());
    assert!(pending.run_id.is_none());
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &pending,
        false,
    );
    assert_eq!(run_preview_run_id_label(&pending), RUN_PREVIEW_PENDING_RUN_ID_LABEL);
    assert_eq!(run_preview_run_id_label(&succeeded), succeeded.run_id.as_deref().unwrap());

    // A multi-node run with rich detail data builds a stable, multi-row timeline.
    let mut multi = run_preview(WorkflowRunStatus::Running);
    multi.run_id = Some("run-multi".to_string());
    multi.nodes.push(run_node("start", "开始", Some("succeeded")));
    multi.nodes.push(run_node("llm", "大模型", Some("running")));
    multi.nodes.push(run_node("answer", "回答", None));
    multi.text_output = "多节点输出".to_string();
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &multi,
        false,
    );
    assert_eq!(run_preview_run_id_label(&multi), "run-multi");

    // Every node status kind resolves to a chip style with a visible color in both themes,
    // and a distinct accent color so dark mode stays readable.
    let kinds = [
        WorkflowPreviewStatusKind::Running,
        WorkflowPreviewStatusKind::Succeeded,
        WorkflowPreviewStatusKind::Failed,
        WorkflowPreviewStatusKind::Canceled,
        WorkflowPreviewStatusKind::Waiting,
        WorkflowPreviewStatusKind::Unknown,
    ];
    for theme in [Theme::Light, Theme::Dark] {
        assert!(workflow_run_preview_node_style(&theme).background.is_some());
        assert!(workflow_run_preview_output_style(&theme).background.is_some());
        for kind in kinds {
            let chip = workflow_run_preview_chip_style(&theme, kind);
            assert!(chip.background.is_some());
            assert!(chip.text_color.is_some());
        }
    }
}

#[test]
fn workflow_run_output_text_handles_empty_long_and_padded_output() {
    // Empty / whitespace-only output collapses to the empty state (None).
    assert!(workflow_run_output_text("").is_none());
    assert!(workflow_run_output_text("   \n\t  ").is_none());

    // Non-empty output trims outer whitespace but keeps inner content verbatim.
    assert_eq!(
        workflow_run_output_text("  第一行\n  第二行  "),
        Some("第一行\n  第二行".to_string())
    );

    // Long output is preserved in full so the report stays scrollable, not truncated.
    let long = "字".repeat(4096);
    let rendered = workflow_run_output_text(&long).expect("long output should render");
    assert_eq!(rendered.chars().count(), 4096);
}

#[test]
fn workflow_run_output_and_error_areas_build_for_empty_long_and_failed_states() {
    // Empty output still builds a stable, lightweight empty-state container.
    let _ = workflow_run_output(crate::app::i18n::I18n::new(Default::default()), "");

    // Long multi-line output builds without panicking.
    let long_output = (0..200).map(|i| format!("行 {i}")).collect::<Vec<_>>().join("\n");
    let _ = workflow_run_output(crate::app::i18n::I18n::new(Default::default()), &long_output);

    // A failed preview with a multi-line, long error message builds a stable error report
    // in both themes via its dedicated styles.
    let mut failed = run_preview(WorkflowRunStatus::Failed);
    failed.run_id = Some("run-err".to_string());
    failed.is_finished = true;
    failed.nodes.push(run_node("agent", "Agent", Some("failed")));
    failed.error = Some(WorkflowRunErrorPreview {
        status: 500,
        code: "workflow_run_error".to_string(),
        message: format!("节点执行失败\n详细堆栈:\n{}", "栈帧 ".repeat(64)),
    });
    let _ = workflow_run_error(crate::app::i18n::I18n::new(Default::default()), &failed);

    for theme in [Theme::Light, Theme::Dark] {
        assert!(workflow_run_preview_output_style(&theme).background.is_some());
        assert!(workflow_run_preview_output_empty_style(&theme).background.is_some());
        assert!(workflow_run_preview_error_style(&theme).background.is_some());
    }
}

#[test]
fn workflow_run_debug_text_redacts_internal_runtime_inputs() {
    let mut failed = run_preview(WorkflowRunStatus::Failed);
    failed.run_id = Some("run-1".to_string());
    failed.is_finished = true;
    failed.nodes.push(WorkflowRunNodePreview {
        id: "start".to_string(),
        title: "Start".to_string(),
        status: Some("failed".to_string()),
        data: serde_json::json!({
            "node_id": "start",
            "title": "Start",
            "status": "failed",
            "inputs": {
                "topic": "rust",
                "__vw_full_access": true,
                "__vw_root": "/Users/secret/project"
            }
        }),
    });
    failed.error = Some(WorkflowRunErrorPreview {
        status: 0,
        code: "transport_error".to_string(),
        message: "连接失败".to_string(),
    });

    let debug_text = failed.debug_text();
    assert!(debug_text.contains("topic"));
    assert!(debug_text.contains("rust"));
    assert!(!debug_text.contains("__vw_full_access"));
    assert!(!debug_text.contains("__vw_root"));
    assert!(!debug_text.contains("/Users/secret/project"));
}

#[test]
fn workflow_run_node_preview_formats_key_value_details() {
    let node = WorkflowRunNodePreview {
        id: "agent".to_string(),
        title: "Agent".to_string(),
        status: Some("succeeded".to_string()),
        data: serde_json::json!({
            "id": "agent-finished",
            "node_id": "agent",
            "node_type": "llm",
            "title": "Agent",
            "status": "succeeded",
            "index": 2,
            "created_at": 0,
            "elapsed_time": 1.25,
            "outputs": {
                "text": "  完成节点输出  ",
                "usage": {
                    "total_tokens": 18
                }
            },
            "__vw_full_access": true,
            "token": "secret-token"
        }),
    };

    let items = workflow_run_node_data_preview_items(
        crate::app::i18n::I18n::new(Default::default()),
        &node,
    );
    assert_eq!(items[0], ("类型:".to_string(), "llm".to_string()));
    assert_eq!(items[1], ("序号:".to_string(), "2".to_string()));
    assert_eq!(items[2].0, "开始时间:");
    assert_eq!(items[2].1.len(), "1970/01/01 08:00:00".len());
    assert_eq!(items[3], ("耗时:".to_string(), "1.25 秒".to_string()));
    assert_eq!(items[4], ("输出:".to_string(), "完成节点输出".to_string()));
    assert!(!items.iter().any(|(_, value)| value.contains("secret-token")));
}

#[test]
fn workflow_run_node_preview_formats_millisecond_and_hides_empty_fields() {
    let node = WorkflowRunNodePreview {
        id: "agent".to_string(),
        title: "Agent".to_string(),
        status: Some("succeeded".to_string()),
        data: serde_json::json!({
            "node_type": "tool",
            "index": 1,
            "created_at_ms": 0,
            "duration_ms": 250,
            "error": "   ",
            "inputs": {},
            "outputs": null,
            "process_data": "ready",
            "secret_key": "should-be-hidden",
        }),
    };

    let items = workflow_run_node_data_preview_items(
        crate::app::i18n::I18n::new(Default::default()),
        &node,
    );

    // Type and index keep their business priority order.
    assert_eq!(items[0], ("类型:".to_string(), "tool".to_string()));
    assert_eq!(items[1], ("序号:".to_string(), "1".to_string()));

    // Millisecond-form timestamp is rendered as a wall-clock string, not a raw number.
    let started = items
        .iter()
        .find(|(label, _)| label == "开始时间:")
        .expect("created_at_ms should produce a 开始时间 row");
    assert_eq!(started.1.len(), "1970/01/01 08:00:00".len());
    assert!(started.1.contains('/') && started.1.contains(':'));

    // Millisecond-form duration is rendered with a 毫秒 unit.
    let elapsed = items
        .iter()
        .find(|(label, _)| label == "耗时:")
        .expect("duration_ms should produce a 耗时 row");
    assert_eq!(elapsed.1, "250 毫秒".to_string());

    // Whitespace-only error, empty inputs object and null outputs are hidden.
    assert!(!items.iter().any(|(label, _)| label == "错误:"));
    assert!(!items.iter().any(|(label, _)| label == "输入:"));
    assert!(!items.iter().any(|(label, _)| label == "输出:"));

    // Priority "过程" field is still surfaced; sensitive field is filtered out entirely.
    assert!(items.iter().any(|(label, value)| label == "过程:" && value == "ready"));
    assert!(!items.iter().any(|(_, value)| value.contains("should-be-hidden")));
}

fn run_preview(status: WorkflowRunStatus) -> WorkflowRunPreview {
    WorkflowRunPreview {
        uuid: "uuid".to_string(),
        run_id: None,
        status,
        nodes: Vec::new(),
        text_output: String::new(),
        error: None,
        is_finished: false,
    }
}

fn run_node(id: &str, title: &str, status: Option<&str>) -> WorkflowRunNodePreview {
    WorkflowRunNodePreview {
        id: id.to_string(),
        title: title.to_string(),
        status: status.map(ToOwned::to_owned),
        data: serde_json::json!({
            "node_id": id,
            "title": title,
            "status": status,
        }),
    }
}
