//! Workflow 应用列表视图测试模块，覆盖列表状态、文案回退、时间格式化与样式分支。

use super::*;
use crate::apps::workflow::state::{
    WorkflowRunErrorPreview, WorkflowRunNodePreview, WorkflowRunPreview, WorkflowRunStatus,
    WorkflowSavedAppSummary,
};
use iced::widget::button;
use iced::{Background, Theme};

#[test]
fn saved_app_title_uses_fallback_for_blank_name() {
    assert_eq!(saved_app_title("  "), "未命名应用");
}

#[test]
fn saved_app_title_trims_non_blank_name() {
    assert_eq!(saved_app_title("  客服助手  "), "客服助手");
}

#[test]
fn saved_app_description_uses_fallback_for_blank_description() {
    assert_eq!(saved_app_description("  "), "暂无描述");
}

#[test]
fn saved_app_description_trims_non_blank_description() {
    assert_eq!(saved_app_description("  自动回复客户问题  "), "自动回复客户问题");
}

#[test]
fn format_saved_app_time_rejects_invalid_timestamp() {
    assert_eq!(format_saved_app_time(u64::MAX), "--");
}

#[test]
fn format_saved_app_time_rejects_out_of_range_i64_timestamp() {
    assert_eq!(format_saved_app_time(i64::MAX as u64), "--");
}

#[test]
fn format_saved_app_time_formats_valid_timestamp() {
    let formatted = format_saved_app_time(0);

    assert_ne!(formatted, "--");
    assert_eq!(formatted.len(), "1970/01/01 08:00".len());
}

#[test]
fn saved_app_matches_search_query() {
    let app = WorkflowSavedAppSummary {
        name: "DHB 数据洞察".to_string(),
        description: "数据智能分析".to_string(),
        ..saved_app_summary()
    };

    assert!(saved_app_matches_query(&app, "dhb"));
    assert!(saved_app_matches_query(&app, "智能"));
    assert!(saved_app_matches_query(&app, " UUID "));
    assert!(saved_app_matches_query(&app, "  "));
    assert!(!saved_app_matches_query(&app, "客服"));
}

#[test]
fn saved_apps_view_builds_loading_error_empty_and_filtered_states() {
    let mut state = WorkflowState { saved_apps_loading: true, ..WorkflowState::default() };
    let _ = build_saved_apps_view(&state);

    state.saved_apps_loading = false;
    state.saved_apps_error = Some("数据库不可用".to_string());
    let _ = build_saved_apps_view(&state);

    state.saved_apps_error = None;
    let _ = build_saved_apps_view(&state);

    state.saved_apps = vec![saved_app_summary()];
    let _ = build_saved_apps_view(&state);

    state.saved_app_search_query = "无匹配".to_string();
    let _ = build_saved_apps_view(&state);
}

#[test]
fn saved_apps_header_and_empty_state_parts_build() {
    let state = WorkflowState::default();

    let _header = build_saved_apps_header(&state);
    let _search = build_search_input(&state);
    let _refresh = saved_apps_header_button("刷新", WorkflowMessage::LoadSavedApps);
    let _create_card = build_create_app_card();
    let _create_button =
        create_action_button(Icon::FileEarmarkPlus, "创建空白应用", WorkflowMessage::OpenFile);
    let _create_row = create_action_row(Icon::FolderOpen, "导入 DSL 文件");
    let _no_result = build_no_result_card();
    let _notice = build_saved_apps_notice("暂无应用");
    let _error = build_saved_apps_error("数据库不可用");
}

#[test]
fn saved_app_card_builds_action_and_progress_states() {
    let state = WorkflowState {
        saved_apps: vec![saved_app_summary()],
        opening_saved_app_uuid: Some("uuid".to_string()),
        deleting_saved_app_uuid: Some("uuid".to_string()),
        saved_app_actions_menu_uuid: Some("uuid".to_string()),
        copied_saved_app_uuid: Some("uuid".to_string()),
        ..WorkflowState::default()
    };

    let _ = build_saved_app_card(&state, &state.saved_apps[0]);
    let _ = build_saved_app_actions_menu("uuid".to_string(), true, true, true, true);
    assert!(matches!(
        saved_app_run_action_message("uuid".to_string(), false),
        Some(WorkflowMessage::OpenSavedAppRunPrompt(uuid)) if uuid == "uuid"
    ));
    assert!(saved_app_run_action_message("uuid".to_string(), true).is_none());
    let _ = build_saved_app_action_item(Icon::ArrowRepeat, "运行中", None, false);
    let _ = build_saved_app_action_item(Icon::Trash, "删除中", None, true);
    let _ = saved_app_uuid_row("uuid", true);
    let _ = saved_app_uuid_row("uuid", false);
}

#[test]
fn workflow_run_preview_builds_running_success_and_error_states() {
    let mut running = run_preview(WorkflowRunStatus::Running);
    running.nodes.push(run_node("start", "开始", Some("running")));
    running.text_output = "正在分析输入".to_string();
    let _ = build_saved_app_run_preview_layer(&running, false);
    assert!(matches!(
        saved_app_run_preview_action(&running),
        Some(("取消运行", WorkflowMessage::CancelSavedAppRun))
    ));
    assert_eq!(workflow_run_status_label(WorkflowRunStatus::Running), "运行中");
    assert_eq!(workflow_run_node_status_label(Some("running")), "运行中");

    let mut succeeded = run_preview(WorkflowRunStatus::Succeeded);
    succeeded.run_id = Some("run-1".to_string());
    succeeded.is_finished = true;
    succeeded.nodes.push(run_node("answer", "回答", Some("succeeded")));
    succeeded.text_output = "完成输出".to_string();
    let _ = build_saved_app_run_preview_layer(&succeeded, false);
    assert!(matches!(
        saved_app_run_preview_action(&succeeded),
        Some(("重新执行", WorkflowMessage::RetrySavedAppRunPreview))
    ));
    assert_eq!(workflow_run_status_label(WorkflowRunStatus::Succeeded), "成功");
    assert_eq!(workflow_run_node_status_label(Some("succeeded")), "成功");

    let mut failed = run_preview(WorkflowRunStatus::Failed);
    failed.is_finished = true;
    failed.nodes.push(run_node("agent", "Agent", Some("failed")));
    failed.error = Some(WorkflowRunErrorPreview {
        status: 500,
        code: "workflow_run_error".to_string(),
        message: "节点执行失败".to_string(),
    });
    let _ = build_saved_app_run_preview_layer(&failed, false);
    assert!(matches!(
        saved_app_run_preview_action(&failed),
        Some(("重新执行", WorkflowMessage::RetrySavedAppRunPreview))
    ));
    assert_eq!(workflow_run_status_label(WorkflowRunStatus::Failed), "失败");
    assert_eq!(workflow_run_node_status_label(Some("failed")), "失败");
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
    let _ = build_saved_app_run_preview_layer(&canceled, false);
    assert!(saved_app_run_preview_action(&canceled).is_none());

    // Node status labels cover every timeline state.
    assert_eq!(workflow_run_node_status_label(Some("canceled")), "已取消");
    assert_eq!(workflow_run_node_status_label(Some("mystery")), "未知");
    assert_eq!(workflow_run_node_status_label(None), "等待中");
    assert_eq!(workflow_run_status_label(WorkflowRunStatus::Canceled), "已取消");

    // An empty-detail node produces no detail rows but still has a stable placeholder.
    let empty_node = WorkflowRunNodePreview {
        id: "pending".to_string(),
        title: "等待节点".to_string(),
        status: None,
        data: serde_json::json!({}),
    };
    assert!(workflow_run_node_data_preview_items(&empty_node).is_empty());

    // A freshly started run with no run id and no nodes must still build: the summary
    // falls back to a placeholder run id and the timeline shows the waiting placeholder.
    let pending = run_preview(WorkflowRunStatus::Running);
    assert!(pending.nodes.is_empty());
    assert!(pending.run_id.is_none());
    let _ = build_saved_app_run_preview_layer(&pending, false);
    assert_eq!(run_preview_run_id_label(&pending), RUN_PREVIEW_PENDING_RUN_ID_LABEL);
    assert_eq!(run_preview_run_id_label(&succeeded), succeeded.run_id.as_deref().unwrap());

    // A multi-node run with rich detail data builds a stable, multi-row timeline.
    let mut multi = run_preview(WorkflowRunStatus::Running);
    multi.run_id = Some("run-multi".to_string());
    multi.nodes.push(run_node("start", "开始", Some("succeeded")));
    multi.nodes.push(run_node("llm", "大模型", Some("running")));
    multi.nodes.push(run_node("answer", "回答", None));
    multi.text_output = "多节点输出".to_string();
    let _ = build_saved_app_run_preview_layer(&multi, false);
    assert_eq!(run_preview_run_id_label(&multi), "run-multi");

    // The scrollbar gutter must stay pinned at the repo-mandated 4px width.
    assert_eq!(RUN_PREVIEW_SCROLLBAR_WIDTH, 4.0);

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
        assert!(workflow_run_preview_card_style(&theme).background.is_some());
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
    let _ = workflow_run_output("");

    // Long multi-line output builds without panicking.
    let long_output = (0..200).map(|i| format!("行 {i}")).collect::<Vec<_>>().join("\n");
    let _ = workflow_run_output(&long_output);

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
    let _ = workflow_run_error(&failed);

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

    let items = workflow_run_node_data_preview_items(&node);
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

    let items = workflow_run_node_data_preview_items(&node);

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

#[test]
fn delete_confirm_dialog_uses_matching_app_or_generic_name() {
    let state = WorkflowState {
        saved_apps: vec![saved_app_summary()],
        confirm_delete_saved_app_uuid: Some("uuid".to_string()),
        ..WorkflowState::default()
    };

    assert!(build_saved_app_delete_confirm_dialog(&state).is_some());

    let missing_state = WorkflowState {
        confirm_delete_saved_app_uuid: Some("missing".to_string()),
        ..WorkflowState::default()
    };

    assert!(build_saved_app_delete_confirm_dialog(&missing_state).is_some());
    assert!(build_saved_app_delete_confirm_dialog(&WorkflowState::default()).is_none());
}

#[test]
fn saved_app_run_prompt_dialog_builds_inputs_and_error_states() {
    let mut state =
        WorkflowState { saved_apps: vec![saved_app_summary()], ..WorkflowState::default() };
    assert!(build_saved_app_run_prompt_dialog(&state).is_none());

    state.open_saved_app_run_prompt("uuid".to_string());
    let _ = build_saved_app_run_prompt_dialog(&state).expect("prompt should build");

    state.finish_saved_app_run_prompt_load(
        "uuid",
        Ok(vec![
            state::WorkflowRunInputDraft {
                name: "query".to_string(),
                label: "Query".to_string(),
                input_type: "paragraph".to_string(),
                required: true,
                value: "hello".to_string(),
                ..Default::default()
            },
            state::WorkflowRunInputDraft {
                name: "enabled".to_string(),
                label: "Enabled".to_string(),
                input_type: "checkbox".to_string(),
                value: "true".to_string(),
                ..Default::default()
            },
        ]),
    );
    let _ = build_saved_app_run_prompt_dialog(&state).expect("prompt with inputs should build");

    state.set_saved_app_run_prompt_error("请输入 query 后再运行".to_string());
    let _ = build_saved_app_run_prompt_dialog(&state).expect("prompt with error should build");
}

#[test]
fn saved_app_run_input_metadata_uses_type_specific_fallbacks() {
    let mut input = state::WorkflowRunInputDraft {
        name: "amount".to_string(),
        input_type: "number".to_string(),
        ..Default::default()
    };

    assert_eq!(saved_app_run_input_label(&input), "amount");
    assert_eq!(saved_app_run_input_placeholder(&input), "请输入数字");

    input.label = "金额".to_string();
    input.input_type = "file-list".to_string();
    assert_eq!(saved_app_run_input_label(&input), "金额");
    assert_eq!(saved_app_run_input_placeholder(&input), "输入文件 URL 或路径，逗号分隔");

    input.input_type = "paragraph".to_string();
    input.hint = "补充说明".to_string();
    assert_eq!(saved_app_run_input_placeholder(&input), "补充说明");

    input.placeholder = "自定义提示".to_string();
    assert_eq!(saved_app_run_input_placeholder(&input), "自定义提示");
}

#[test]
fn saved_app_run_inputs_build_loading_hidden_select_and_text_variants() {
    let mut prompt = state::WorkflowRunPromptDraft {
        uuid: "uuid".to_string(),
        loading: true,
        ..Default::default()
    };
    let _ = build_saved_app_run_inputs(&prompt);

    prompt.loading = false;
    prompt.inputs = vec![state::WorkflowRunInputDraft {
        name: "internal".to_string(),
        hidden: true,
        ..Default::default()
    }];
    let _ = build_saved_app_run_inputs(&prompt);
    assert!(!saved_app_run_prompt_has_query_input(&prompt));

    prompt.inputs = vec![
        state::WorkflowRunInputDraft {
            name: "query".to_string(),
            label: "Query".to_string(),
            input_type: "select".to_string(),
            options: vec!["a".to_string(), "b".to_string()],
            value: "b".to_string(),
            required: true,
            ..Default::default()
        },
        state::WorkflowRunInputDraft {
            name: "enabled".to_string(),
            input_type: "checkbox".to_string(),
            value: "false".to_string(),
            hint: "是否启用".to_string(),
            ..Default::default()
        },
        state::WorkflowRunInputDraft {
            name: "attachment".to_string(),
            input_type: "file".to_string(),
            ..Default::default()
        },
        state::WorkflowRunInputDraft {
            name: "fallback".to_string(),
            input_type: "select".to_string(),
            value: "free text".to_string(),
            ..Default::default()
        },
    ];

    assert!(saved_app_run_prompt_has_query_input(&prompt));
    let _ = build_saved_app_run_inputs(&prompt);
    for (index, input) in prompt.inputs.iter().enumerate() {
        let _ = build_saved_app_run_input_field(index, input);
        let _ = build_saved_app_run_input_control(index, input);
    }
}

#[test]
fn saved_app_styles_cover_light_dark_and_button_statuses() {
    let light = Theme::Light;
    let dark = Theme::Dark;

    assert!(saved_app_actions_menu_style(&light).background.is_some());
    assert!(saved_app_actions_menu_style(&dark).background.is_some());
    assert!(saved_app_card_container_style(&light).background.is_some());
    assert!(saved_app_card_container_style(&dark).background.is_some());
    assert!(saved_app_name_text_style(&dark).color.is_some());
    assert!(workflow_run_prompt_error_style(&light).background.is_some());

    let enabled_hovered = saved_app_action_item_style(&light, button::Status::Hovered, true, true);
    let enabled_pressed = saved_app_action_item_style(&light, button::Status::Pressed, false, true);
    let enabled_active = saved_app_action_item_style(&light, button::Status::Active, false, true);
    let disabled_active = saved_app_action_item_style(&light, button::Status::Active, false, false);
    let disabled_hovered =
        saved_app_action_item_style(&light, button::Status::Hovered, true, false);

    assert!(enabled_hovered.background.is_some());
    assert!(enabled_pressed.background.is_some());
    assert!(enabled_active.background.is_none());
    assert!(disabled_active.background.is_none());
    assert!(disabled_hovered.background.is_none());
    assert!(saved_app_card_button_style(&light, button::Status::Active).background.is_none());
}

#[test]
fn saved_app_tag_and_badge_styles_build_for_dark_theme() {
    let _ = saved_app_robot_badge();
    let _ = saved_app_tag("工作流");

    let card_style = saved_app_card_container_style(&Theme::Dark);
    let Background::Color(background) = card_style.background.expect("card background") else {
        panic!("card background should be a color");
    };

    assert!(background.a > 0.0);
}

fn saved_app_summary() -> WorkflowSavedAppSummary {
    WorkflowSavedAppSummary {
        uuid: "uuid".to_string(),
        name: "应用".to_string(),
        description: String::new(),
        created_at_ms: 0,
        updated_at_ms: 0,
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
        data: serde_json::json!({
            "node_id": id,
            "title": title,
            "status": status,
        }),
    }
}
