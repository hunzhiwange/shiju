//! 工作流运行预览测试，覆盖敏感字段过滤、数值格式化与状态别名。

use super::*;
use serde_json::json;

#[test]
fn run_preview_layers_actions_and_copy_controls_build_all_header_branches() {
    let mut running = run_preview(WorkflowRunStatus::Running);
    running.nodes.push(run_node("start", "Start", Some("running")));
    running.text_output = "  line one\nline two  ".to_string();

    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &running,
        false,
    );
    assert_eq!(run_preview_run_id_label(&running), RUN_PREVIEW_PENDING_RUN_ID_LABEL);
    assert!(matches!(
        saved_app_run_preview_action(crate::app::i18n::I18n::new(Default::default()), &running),
        Some(("取消运行", WorkflowMessage::CancelSavedAppRun))
    ));
    assert!(matches!(
        saved_app_run_preview_close_message(),
        WorkflowMessage::CloseSavedAppRunPreview
    ));

    let mut succeeded = run_preview(WorkflowRunStatus::Succeeded);
    succeeded.run_id = Some("run-1".to_string());
    succeeded.is_finished = true;
    succeeded.text_output = "done".to_string();
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &succeeded,
        false,
    );
    assert_eq!(run_preview_run_id_label(&succeeded), "run-1");
    assert!(matches!(
        saved_app_run_preview_action(crate::app::i18n::I18n::new(Default::default()), &succeeded),
        Some(("重新执行", WorkflowMessage::RetrySavedAppRunPreview))
    ));

    let mut failed = run_preview(WorkflowRunStatus::Failed);
    failed.is_finished = true;
    failed.error = Some(state::WorkflowRunErrorPreview {
        status: 500,
        code: "node_error".to_string(),
        message: "boom".to_string(),
    });
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &failed,
        true,
    );
    assert!(matches!(
        saved_app_run_preview_action(crate::app::i18n::I18n::new(Default::default()), &failed),
        Some(("重新执行", WorkflowMessage::RetrySavedAppRunPreview))
    ));

    let mut canceled = run_preview(WorkflowRunStatus::Canceled);
    canceled.is_finished = true;
    let _ = build_saved_app_run_preview_layer(
        crate::app::i18n::I18n::new(Default::default()),
        &canceled,
        false,
    );
    assert!(
        saved_app_run_preview_action(crate::app::i18n::I18n::new(Default::default()), &canceled)
            .is_none()
    );

    let detail = state::WorkflowRunDetailDialog {
        title_source: None,
        title: "节点详情".to_string(),
        content: text_editor::Content::with_text("full detail"),
    };
    let _ =
        build_saved_app_run_detail_layer(crate::app::i18n::I18n::new(Default::default()), &detail);
    let _ = run_preview_section(
        "标题",
        Some(run_preview_detail_button(
            crate::app::i18n::I18n::new(Default::default()),
            Message::WorkflowTool(WorkflowMessage::CloseSavedAppRunDetail),
        )),
        text("内容").into(),
    );
    let _ = run_preview_copy_button(
        crate::app::i18n::I18n::new(Default::default()),
        false,
        Message::None,
    );
    let _ = run_preview_copy_button(
        crate::app::i18n::I18n::new(Default::default()),
        true,
        Message::None,
    );
}

#[test]
fn sensitive_run_preview_keys_are_filtered_case_insensitively() {
    for key in [
        "__vw_internal",
        "access_token",
        "ClientSecret",
        "PASSWORD",
        "api_key",
        "apiKey",
        "Authorization",
        "cookie",
    ] {
        assert!(is_sensitive_run_preview_key(key), "{key} should be sensitive");
    }

    assert!(!is_sensitive_run_preview_key("usage_count"));
}

#[test]
fn run_preview_values_format_timestamps_arrays_objects_and_truncated_text() {
    assert!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "created_at",
            &json!(0)
        )
        .expect("epoch seconds should format")
        .contains('/')
    );
    assert!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "finished_at_ms",
            &json!("1000")
        )
        .expect("epoch millis string should format")
        .contains(':')
    );
    assert_eq!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "created_at",
            &json!(-1)
        ),
        None
    );
    assert_eq!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "created_at_ms",
            &json!(1.0e30)
        ),
        None
    );
    assert_eq!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "duration_ms",
            &json!(250)
        ),
        Some("250 毫秒".to_string())
    );
    assert_eq!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "duration",
            &json!(1.5)
        ),
        Some("1.5 秒".to_string())
    );

    let compact = workflow_run_preview_value(
        crate::app::i18n::I18n::new(Default::default()),
        "text",
        &json!("  hello\n\nworld  "),
    )
    .expect("non-empty string should compact");
    assert_eq!(compact, "hello world");

    let truncated = workflow_run_preview_value(
        crate::app::i18n::I18n::new(Default::default()),
        "text",
        &json!("x".repeat(140)),
    )
    .expect("long string should truncate");
    assert!(truncated.ends_with("..."));
    assert_eq!(truncated.chars().count(), RUN_PREVIEW_NODE_MAX_VALUE_CHARS + 3);

    assert_eq!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "flag",
            &json!(true)
        ),
        Some("true".to_string())
    );
    assert_eq!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "items",
            &json!(["one", "two", "three", "four"])
        ),
        Some("one, two, three 等，共 4 项".to_string())
    );
    assert_eq!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "items",
            &json!([null, null])
        ),
        Some("2 项".to_string())
    );
    let object_summary = workflow_run_preview_value(
        crate::app::i18n::I18n::new(Default::default()),
        "payload",
        &json!({"access_token": "secret", "visible": "ok", "count": 2, "extra": "more"}),
    )
    .expect("non-sensitive object fields should summarize");
    assert!(object_summary.contains("visible=ok"));
    assert!(object_summary.contains("count=2"));
    assert!(object_summary.contains("extra=more"));
    assert!(object_summary.ends_with(" 等"));
    assert_eq!(
        workflow_run_preview_value(
            crate::app::i18n::I18n::new(Default::default()),
            "payload",
            &json!({"output": "preferred"})
        ),
        Some("preferred".to_string())
    );
}

#[test]
fn run_preview_duration_rejects_negative_and_non_numeric_values() {
    assert_eq!(
        format_run_preview_duration(
            crate::app::i18n::I18n::new(Default::default()),
            "elapsed_ms",
            &json!(-1)
        ),
        None
    );
    assert_eq!(
        format_run_preview_duration(
            crate::app::i18n::I18n::new(Default::default()),
            "elapsed_ms",
            &json!("invalid")
        ),
        None
    );
    assert_eq!(
        format_run_preview_duration(
            crate::app::i18n::I18n::new(Default::default()),
            "elapsed_ms",
            &json!(999.6)
        ),
        Some("1000 毫秒".to_string())
    );
    assert_eq!(
        format_run_preview_duration(
            crate::app::i18n::I18n::new(Default::default()),
            "elapsed_time",
            &json!(1.25)
        ),
        Some("1.25 秒".to_string())
    );
}

#[test]
fn run_preview_node_data_limits_fields_and_hides_runtime_keys() {
    let node = WorkflowRunNodePreview {
        id: "agent".to_string(),
        title: "Agent".to_string(),
        status: Some("succeeded".to_string()),
        data: json!({
            "id": "skip-id",
            "node_id": "skip-node",
            "title": "skip-title",
            "status": "skip-status",
            "node_type": "llm",
            "index": 7,
            "predecessor_node_id": "start",
            "started_at_ms": 0,
            "elapsed_ms": 1000,
            "inputs": {"question": "rust", "token": "hidden"},
            "process_data": {"answer": "working"},
            "outputs": {"text": "done"},
            "apiKey": "hidden",
            "custom_a": "a",
            "custom_b": "b"
        }),
    };

    let items = workflow_run_node_data_preview_items(
        crate::app::i18n::I18n::new(Default::default()),
        &node,
    );
    assert_eq!(items.len(), RUN_PREVIEW_NODE_MAX_FIELDS);
    assert_eq!(items[0], ("类型:".to_string(), "llm".to_string()));
    assert_eq!(items[1], ("序号:".to_string(), "7".to_string()));
    assert!(items.iter().any(|(label, value)| label == "上游:" && value == "start"));
    assert!(items.iter().any(|(label, value)| label == "输入:" && value.contains("question=rust")));
    assert!(items.iter().any(|(label, value)| label == "过程:" && value == "working"));
    assert!(items.iter().any(|(label, value)| label == "输出:" && value == "done"));
    assert!(!items.iter().any(|(_, value)| value.contains("hidden")));

    let scalar_node = WorkflowRunNodePreview {
        id: "scalar".to_string(),
        title: "Scalar".to_string(),
        status: None,
        data: json!(42),
    };
    assert_eq!(
        workflow_run_node_data_preview_items(
            crate::app::i18n::I18n::new(Default::default()),
            &scalar_node
        ),
        vec![("data".to_string(), "42".to_string())]
    );
}

#[test]
fn run_preview_summaries_hide_sensitive_only_objects_and_describe_null_arrays() {
    let sensitive = json!({"access_token": "secret", "password": "hidden"});
    let object = sensitive.as_object().expect("object fixture");
    assert_eq!(
        summarize_run_preview_object(crate::app::i18n::I18n::new(Default::default()), object),
        None
    );

    assert_eq!(
        summarize_run_preview_array(
            crate::app::i18n::I18n::new(Default::default()),
            &[serde_json::Value::Null, serde_json::Value::Null]
        ),
        Some("2 项".to_string())
    );
}

#[test]
fn run_preview_node_status_aliases_map_to_labels_and_kinds() {
    for alias in ["started", "running"] {
        assert_eq!(
            workflow_run_node_status_label(
                crate::app::i18n::I18n::new(Default::default()),
                Some(alias)
            ),
            "运行中"
        );
        assert_eq!(workflow_run_node_status_kind(Some(alias)), WorkflowPreviewStatusKind::Running);
    }
    for alias in ["success", "finished"] {
        assert_eq!(
            workflow_run_node_status_label(
                crate::app::i18n::I18n::new(Default::default()),
                Some(alias)
            ),
            "成功"
        );
        assert_eq!(
            workflow_run_node_status_kind(Some(alias)),
            WorkflowPreviewStatusKind::Succeeded
        );
    }
    assert_eq!(
        workflow_run_node_status_label(
            crate::app::i18n::I18n::new(Default::default()),
            Some("unexpected")
        ),
        "未知"
    );
    assert_eq!(
        workflow_run_node_status_kind(Some("unexpected")),
        WorkflowPreviewStatusKind::Unknown
    );
    assert_eq!(
        workflow_run_node_status_label(crate::app::i18n::I18n::new(Default::default()), None),
        "等待中"
    );
    assert_eq!(workflow_run_node_status_kind(None), WorkflowPreviewStatusKind::Waiting);
}

#[test]
fn run_preview_styles_keep_visible_backgrounds_and_opaque_accent_text() {
    for theme in [Theme::Light, Theme::Dark] {
        assert!(workflow_run_preview_card_style(&theme).background.is_some());
        assert!(workflow_run_preview_node_style(&theme).background.is_some());
        assert!(workflow_run_preview_output_style(&theme).background.is_some());
        assert!(workflow_run_preview_output_empty_style(&theme).background.is_some());
        assert!(workflow_run_preview_error_style(&theme).background.is_some());
        assert!(
            workflow_run_status_badge_style(&theme, WorkflowRunStatus::Running)
                .background
                .is_some()
        );
        assert!(
            run_preview_copied_button_style(&theme, iced::widget::button::Status::Active)
                .background
                .is_some()
        );

        let editor = workflow_run_detail_editor_style(&theme, text_editor::Status::Active);
        assert_eq!(editor.border.width, 0.0);
        assert_eq!(editor.background, Background::Color(Color::TRANSPARENT));

        for kind in [
            WorkflowPreviewStatusKind::Running,
            WorkflowPreviewStatusKind::Succeeded,
            WorkflowPreviewStatusKind::Failed,
            WorkflowPreviewStatusKind::Canceled,
            WorkflowPreviewStatusKind::Waiting,
            WorkflowPreviewStatusKind::Unknown,
        ] {
            let chip = workflow_run_preview_chip_style(&theme, kind);
            assert!(chip.background.is_some());
            assert_eq!(chip.text_color.expect("chip text color").a, 1.0);
            assert!(workflow_run_status_kind_color(&theme, kind).a > 0.0);
        }

        assert_eq!(workflow_run_status_color(&theme, WorkflowRunStatus::Succeeded).a, 1.0);
        assert_eq!(opaque_color(Color { a: 0.1, ..Color::BLACK }).a, 1.0);
    }
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
        data: json!({
            "node_id": id,
            "title": title,
            "status": status,
        }),
    }
}
