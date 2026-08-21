use super::{
    PLAN_RUN_CARD_MAX_WIDTH, WorkflowNodeState, chat_summary_text, elapsed_label, metadata_string,
    metadata_value, node_meta_label, output_preview_text, output_value, plan_run_card_display,
    plan_run_tooltip_content, status_text, token_value,
    tool_workflow_view, usage_label, usage_value,
    workflow_node_detail_body, workflow_node_primary_message, workflow_node_state,
    workflow_preview_message,
};
use crate::app::message;
use crate::app::{App, Message};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size, Theme};
use serde_json::json;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn workflow_node_state_maps_running_failures_and_fallbacks() {
    assert_eq!(workflow_node_state("running"), WorkflowNodeState::Running);
    assert_eq!(workflow_node_state("waiting"), WorkflowNodeState::Waiting);
    assert_eq!(workflow_node_state("cancelled"), WorkflowNodeState::Cancelled);
    for status in ["error", "failed", "denied"] {
        assert_eq!(workflow_node_state(status), WorkflowNodeState::Failed);
    }
    for status in ["completed", "success", ""] {
        assert_eq!(workflow_node_state(status), WorkflowNodeState::Completed);
    }
}

#[test]
fn workflow_preview_message_requires_nonempty_yaml_and_trims_focus_node() {
    let message = workflow_preview_message(&json!({
        "metadata": {
            "workflow_yaml": "  nodes: []  ",
            "node_id": "  node-a  "
        }
    }))
    .expect("workflow preview message");

    match message {
        Message::WorkflowTool(crate::apps::workflow::WorkflowMessage::OpenInlineYaml {
            workflow_yaml,
            focus_node_id,
        }) => {
            assert_eq!(workflow_yaml, "nodes: []");
            assert_eq!(focus_node_id.as_deref(), Some("node-a"));
        }
        other => panic!("unexpected preview message: {other:?}"),
    }

    let blank_node = workflow_preview_message(&json!({
        "metadata": {"workflow_yaml": "nodes: []", "node_id": "   "}
    }))
    .expect("preview without focused node");
    assert!(matches!(
        blank_node,
        Message::WorkflowTool(crate::apps::workflow::WorkflowMessage::OpenInlineYaml {
            focus_node_id: None,
            ..
        })
    ));

    for value in [
        json!({}),
        json!({"metadata": []}),
        json!({"metadata": {}}),
        json!({"metadata": {"workflow_yaml": "   "}}),
        json!({"metadata": {"workflow_yaml": 7}}),
    ] {
        assert!(workflow_preview_message(&value).is_none());
    }
}

#[test]
fn status_copy_matches_each_workflow_state() {
    assert_eq!(status_text(WorkflowNodeState::Running), "正在执行节点");
    assert_eq!(status_text(WorkflowNodeState::Waiting), "正在等待后代智能体");
    assert_eq!(status_text(WorkflowNodeState::Completed), "节点执行完成");
    assert_eq!(status_text(WorkflowNodeState::Failed), "节点执行失败");
    assert_eq!(status_text(WorkflowNodeState::Cancelled), "节点执行已取消");
}

#[test]
fn chat_summary_text_stays_compact_and_detail_body_keeps_full_output() {
    let value = json!({
        "status": "completed",
        "output": "{\"answer\":\"这是一段很长的节点输出，不应该整段塞进聊天摘要行\",\"usage\":{\"total_tokens\":9}}",
        "metadata": {"node_type": "llm", "index": 3, "elapsed_time": 0.5}
    });
    let summary = chat_summary_text(&value, WorkflowNodeState::Completed);
    assert!(summary.contains("很长的节点输出"));
    assert!(summary.contains("#3"));
    assert!(summary.chars().count() <= 96);

    let detail = workflow_node_detail_body(&value);
    assert!(detail.contains("这是一段很长的节点输出，不应该整段塞进聊天摘要行"));
    assert!(detail.contains('\n') || detail.contains("answer"));
}

#[test]
fn metadata_helpers_trim_strings_and_reject_missing_empty_or_wrong_types() {
    let value = json!({"metadata": {"title": "  Build  ", "empty": " ", "count": 2}});

    assert_eq!(metadata_value(&value, "count"), Some(&json!(2)));
    assert_eq!(metadata_string(&value, "title"), "Build");
    assert_eq!(metadata_string(&value, "empty"), "");
    assert_eq!(metadata_string(&value, "count"), "");
    assert_eq!(metadata_string(&value, "missing"), "");
    assert!(metadata_value(&json!({"metadata": null}), "title").is_none());
}

#[test]
fn elapsed_label_formats_milliseconds_seconds_and_invalid_values() {
    assert_eq!(
        elapsed_label(&json!({"metadata": {"elapsed_time": 0.125}})).as_deref(),
        Some("125ms")
    );
    assert_eq!(
        elapsed_label(&json!({"metadata": {"elapsed_time": 1.234}})).as_deref(),
        Some("1.23s")
    );
    for value in [
        json!({}),
        json!({"metadata": {"elapsed_time": 0}}),
        json!({"metadata": {"elapsed_time": -1}}),
        json!({"metadata": {"elapsed_time": "fast"}}),
    ] {
        assert!(elapsed_label(&value).is_none());
    }
}

#[test]
fn token_value_accepts_unsigned_signed_and_rounded_float_aliases() {
    assert_eq!(token_value(&json!({"a": 7}), &["a"]), Some(7));
    assert_eq!(token_value(&json!({"a": 8_i64}), &["a"]), Some(8));
    assert_eq!(token_value(&json!({"a": 2.6}), &["a"]), Some(3));
    assert_eq!(token_value(&json!({"a": -1, "b": 9}), &["a", "b"]), Some(9));
    assert_eq!(token_value(&json!({"a": -0.5}), &["a"]), None);
    assert_eq!(token_value(&json!({"a": "4"}), &["a"]), None);
}

#[test]
fn output_and_usage_values_follow_documented_precedence() {
    let value = json!({
        "metadata": {"usage": {"total_tokens": 1}},
        "usage": {"total_tokens": 2},
        "output": "{\"usage\":{\"total_tokens\":3}}"
    });
    assert_eq!(usage_value(&value), Some(json!({"total_tokens": 1})));

    let root_usage =
        json!({"usage": {"total_tokens": 2}, "output": "{\"usage\":{\"total_tokens\":3}}"});
    assert_eq!(usage_value(&root_usage), Some(json!({"total_tokens": 2})));

    let output_usage = json!({"output": " {\"usage\":{\"total_tokens\":3}} "});
    assert_eq!(output_value(&output_usage), Some(json!({"usage": {"total_tokens": 3}})));
    assert_eq!(usage_value(&output_usage), Some(json!({"total_tokens": 3})));
    assert!(output_value(&json!({"output": "not json"})).is_none());
    assert!(output_value(&json!({"output": 4})).is_none());
    assert!(usage_value(&json!({})).is_none());
}

#[test]
fn usage_label_formats_aliases_and_computes_missing_total() {
    let label = usage_label(&json!({"metadata": {"usage": {
        "input_tokens": 10,
        "output_tokens": 4,
        "reasoning_tokens": 2,
        "cache_read_tokens": 3
    }}}));
    assert_eq!(label.as_deref(), Some("Token 输入 10 · 输出 4 · 推理 2 · 缓存 3 · 总计 16"));

    assert_eq!(
        usage_label(
            &json!({"usage": {"prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 20}})
        )
        .as_deref(),
        Some("Token 输入 1 · 输出 2 · 总计 20")
    );
    assert!(usage_label(&json!({"usage": {"total_tokens": 0}})).is_none());
    assert!(usage_label(&json!({"usage": {"prompt_tokens": "many"}})).is_none());
}

#[test]
fn output_preview_uses_state_defaults_and_failure_errors() {
    assert_eq!(output_preview_text(&json!({}), WorkflowNodeState::Running), "正在执行工作流节点");
    assert_eq!(output_preview_text(&json!({}), WorkflowNodeState::Completed), "节点执行完成");
    assert_eq!(
        output_preview_text(&json!({"error": "  broken  "}), WorkflowNodeState::Failed),
        "broken"
    );
    assert_eq!(output_preview_text(&json!({}), WorkflowNodeState::Failed), "节点执行失败");
    assert_eq!(
        output_preview_text(&json!({"output": "   "}), WorkflowNodeState::Completed),
        "节点执行完成"
    );
    assert_eq!(
        output_preview_text(&json!({"output": "正在执行工作流节点"}), WorkflowNodeState::Running),
        "正在执行工作流节点"
    );
}

#[test]
fn output_preview_extracts_json_text_and_removes_usage_only_payloads() {
    for key in ["answer", "text", "result"] {
        let output =
            serde_json::to_string(&json!({(key): "  node result  ", "usage": {"total_tokens": 2}}))
                .expect("output json");
        assert_eq!(
            output_preview_text(&json!({"output": output}), WorkflowNodeState::Completed),
            "node result"
        );
    }

    assert_eq!(
        output_preview_text(
            &json!({"output": "{\"usage\":{\"total_tokens\":2}}"}),
            WorkflowNodeState::Completed
        ),
        "节点执行完成"
    );
    assert_eq!(
        output_preview_text(
            &json!({"output": "{\"value\":1,\"usage\":{\"total_tokens\":2}}"}),
            WorkflowNodeState::Completed
        ),
        "{\"value\":1}"
    );
    assert_eq!(
        output_preview_text(&json!({"output": " plain output "}), WorkflowNodeState::Completed),
        "plain output"
    );
    assert_eq!(
        output_preview_text(&json!({"output": "[1,2]"}), WorkflowNodeState::Completed),
        "[1,2]"
    );
}

#[test]
fn node_meta_label_joins_only_available_metadata() {
    let value = json!({"metadata": {
        "index": 3,
        "elapsed_time": 0.5,
        "usage": {"total_tokens": 9}
    }});
    assert_eq!(node_meta_label(&value, "llm"), "#3 · llm · 耗时 500ms · Token 总计 9");
    assert_eq!(node_meta_label(&json!({}), ""), "");
    assert_eq!(node_meta_label(&json!({"metadata": {"index": "3"}}), "code"), "code");
}

#[test]
fn tool_workflow_view_rejects_malformed_wrong_tool_and_invalid_json() {
    let app = test_app();

    assert!(tool_workflow_view(&app, 0, 0, "tool workflow_node").is_none());
    assert!(tool_workflow_view(&app, 0, 0, "workflow_node\n{}").is_none());
    assert!(tool_workflow_view(&app, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_workflow_view(&app, 0, 0, "tool workflow_node\nnot json").is_none());
}

#[test]
fn tool_workflow_view_builds_running_waiting_completed_failed_and_cancelled_cards() {
    let app = test_app();
    let running = tool_workflow_view(
        &app,
        1,
        1,
        "tool workflow_node\n{\"status\":\"running\",\"metadata\":{\"node_type\":\"llm\"}}",
    )
    .expect("running workflow card");
    keep_element(running);

    let waiting = tool_workflow_view(
        &app,
        1,
        2,
        "tool workflow_node\n{\"status\":\"waiting\",\"metadata\":{\"node_type\":\"agent\"}}",
    )
    .expect("waiting workflow card");
    keep_element(waiting);

    let completed = tool_workflow_view(
        &app,
        1,
        3,
        "tool workflow_node\n{\"status\":\"completed\",\"output\":\"{\\\"answer\\\":\\\"done\\\"}\",\"metadata\":{\"title\":\"Answer\"}}",
    )
    .expect("completed workflow card");
    keep_element(completed);

    let failed = tool_workflow_view(
        &app,
        1,
        4,
        "tool workflow_node\n{\"status\":\"denied\",\"error\":\"blocked\"}",
    )
    .expect("failed workflow card");
    keep_element(failed);

    let cancelled = tool_workflow_view(
        &app,
        1,
        5,
        "tool workflow_node\n{\"status\":\"cancelled\",\"metadata\":{\"plan_run\":true}}",
    )
    .expect("cancelled workflow card");
    keep_element(cancelled);
}

#[test]
fn tool_workflow_view_builds_hovered_preview_card() {
    let mut app = test_app();
    app.chat_tool_hovered_idx = Some((2_u64 << 32) | 4);
    let visible = r#"tool WORKFLOW_NODE
{"status":"completed","output":"ok","metadata":{"workflow_yaml":"nodes: []","node_id":"node-a"}}"#;

    let element = tool_workflow_view(&app, 2, 4, visible).expect("hovered preview card");
    keep_element(element);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn plan_run_planning_card_uses_compact_state_copy_and_hover_details() {
    let started = json!({
        "status": "running",
        "output": "{\"answer\":\"/plans/task-1\"}",
        "metadata": {
            "plan_run": true,
            "event_type": "planning_started",
            "task_id": "task-1-uuid",
            "task_path": "/plans/task-1"
        }
    });
    let created = json!({
        "status": "completed",
        "output": "{\"answer\":\"/plans/task-1/plan.md\"}",
        "metadata": {
            "plan_run": true,
            "event_type": "plan_created",
            "task_id": "task-1-uuid",
            "task_path": "/plans/task-1",
            "subtask_count": 3
        }
    });

    let started_display = plan_run_card_display(&started).expect("planning display");
    assert_eq!(started_display.primary_text, "正在规划计划任务");
    assert!(!started_display.primary_text.contains("task-1-uuid"));
    assert!(!started_display.primary_text.contains("/plans/task-1"));
    assert_eq!(
        started_display.tooltip_text.as_deref(),
        Some("UUID：task-1-uuid\n任务路径：/plans/task-1")
    );

    let created_display = plan_run_card_display(&created).expect("created display");
    assert_eq!(created_display.primary_text, "计划任务已创建，共 3 个任务。");
    assert!(!created_display.primary_text.contains("task-1-uuid"));
    assert!(!created_display.primary_text.contains("/plans/task-1"));

    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    for theme in [Theme::Light, Theme::Dark] {
        let mut tooltip = plan_run_tooltip_content(
            created_display.tooltip_text.clone().expect("created tooltip text"),
        );
        let limits = layout::Limits::new(Size::ZERO, Size::new(520.0, 160.0));
        let mut tree = Tree::new(tooltip.as_widget());
        let node = tooltip.as_widget_mut().layout(&mut tree, &renderer, &limits);
        tooltip.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(Size::new(520.0, 160.0)),
        );
    }

    let app = test_app();
    for value in [started, created] {
        let visible = format!("tool workflow_node\n{value}");
        let element = tool_workflow_view(&app, 2, 4, &visible).expect("plan-run planning card");
        keep_element(element);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn plan_run_subtask_card_hides_path_until_hover() {
    let task_path = "/项目/计划/2026-08-21-子任务卡片路径精简/1-子任务路径移入-tooltip.md";
    let cases = [
        (
            json!({
                "status": "running",
                "output": format!(r#"{{"answer":"子任务开始：整理卡片\n任务文件：{task_path}"}}"#),
                "metadata": {
                    "plan_run": true,
                    "event_type": "subtask_started",
                    "title": "整理卡片",
                    "task_path": task_path
                }
            }),
            "子任务开始",
        ),
        (
            json!({
                "status": "completed",
                "output": format!(r#"{{"answer":"子任务完成，任务文件已处理：{task_path}\n执行输出：完成"}}"#),
                "metadata": {
                    "plan_run": true,
                    "event_type": "subtask_finished",
                    "title": "整理卡片",
                    "task_path": task_path
                }
            }),
            "执行输出：完成",
        ),
        (
            json!({
                "status": "failed",
                "error": format!("子任务失败，任务文件保留：{task_path}\n执行失败"),
                "metadata": {
                    "plan_run": true,
                    "event_type": "subtask_finished",
                    "title": "整理卡片",
                    "task_path": task_path
                }
            }),
            "执行失败",
        ),
    ];

    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = test_app();
    for (value, detail_fragment) in cases {
        let display = plan_run_card_display(&value).expect("subtask display");
        assert_eq!(display.primary_text, "整理卡片");
        assert!(display.summary_text.is_empty());
        assert!(!display.primary_text.contains("任务文件"));
        assert!(!display.primary_text.contains(task_path));
        let expected_tooltip = format!("任务路径：{task_path}");
        assert_eq!(display.tooltip_text.as_deref(), Some(expected_tooltip.as_str()));

        let detail = workflow_node_detail_body(&value);
        assert!(detail.contains(task_path));
        assert!(detail.contains(detail_fragment));

        let visible = format!("tool workflow_node\n{value}");
        let mut element = tool_workflow_view(&app, 2, 4, &visible).expect("plan-run subtask card");
        let viewport = Size::new(1_200.0, 180.0);
        let limits = layout::Limits::new(Size::ZERO, viewport);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let width = Layout::new(&node).bounds().width;
        assert!(width <= PLAN_RUN_CARD_MAX_WIDTH);
        assert!(width < viewport.width);
    }

    let without_path = json!({
        "status": "running",
        "metadata": {
            "plan_run": true,
            "event_type": "subtask_started",
            "title": "整理卡片",
            "task_path": ""
        }
    });
    assert_eq!(
        plan_run_card_display(&without_path).and_then(|display| display.tooltip_text),
        None
    );

    for theme in [Theme::Light, Theme::Dark] {
        let mut tooltip = plan_run_tooltip_content(format!("任务路径：{task_path}"));
        let limits = layout::Limits::new(Size::ZERO, Size::new(440.0, 160.0));
        let mut tree = Tree::new(tooltip.as_widget());
        let node = tooltip.as_widget_mut().layout(&mut tree, &renderer, &limits);
        tooltip.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(Size::new(440.0, 160.0)),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn plan_run_planning_card_caps_width_in_wide_viewport() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = test_app();
    let visible = r#"tool workflow_node
{"status":"completed","metadata":{"plan_run":true,"event_type":"plan_created","task_id":"task-1-uuid","task_path":"/plans/task-1","subtask_count":3}}"#;
    let mut element = tool_workflow_view(&app, 2, 4, visible).expect("plan-run planning card");
    let viewport = Size::new(1_200.0, 160.0);
    let limits = layout::Limits::new(Size::ZERO, viewport);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let width = Layout::new(&node).bounds().width;

    assert!(
        width <= PLAN_RUN_CARD_MAX_WIDTH,
        "plan-run planning card should be capped at {PLAN_RUN_CARD_MAX_WIDTH}px, got {width}px"
    );
    assert!(width < viewport.width, "planning card should not fill the viewport");
}

#[test]
fn plan_run_node_click_opens_session_when_session_id_present() {
    let visible = "tool workflow_node\n{\"status\":\"completed\"}";
    let value = json!({
        "status": "completed",
        "metadata": {
            "plan_run": true,
            "session_id": " task-1-plan "
        }
    });

    let message = workflow_node_primary_message(Some("/work/proj"), &value, 1, 2, visible);
    match message {
        Message::Project(message::ProjectMessage::OpenProjectSessionPressed(path, id)) => {
            assert_eq!(path, "/work/proj");
            assert_eq!(id, "task-1-plan");
        }
        other => panic!("expected OpenProjectSessionPressed, got {other:?}"),
    }
}

#[test]
fn plan_run_completed_node_uses_event_directory_to_open_session() {
    let visible = "tool workflow_node\n{\"status\":\"completed\"}";
    let value = json!({
        "status": "completed",
        "metadata": {
            "plan_run": true,
            "session_id": " task-1-sub ",
            "session_directory": " /work/event-project "
        }
    });

    for project_path in [None, Some("/work/other-project")] {
        let message = workflow_node_primary_message(project_path, &value, 1, 2, visible);
        match message {
            Message::Project(message::ProjectMessage::OpenProjectSessionPressed(path, id)) => {
                assert_eq!(path, "/work/event-project");
                assert_eq!(id, "task-1-sub");
            }
            other => panic!("expected event directory navigation, got {other:?}"),
        }
    }
}

#[test]
fn plan_run_running_node_click_opens_session_with_session_id() {
    let visible = "tool workflow_node\n{\"status\":\"running\"}";
    let value = json!({
        "status": "running",
        "metadata": {
            "plan_run": true,
            "session_id": "task-1-plan"
        }
    });

    let message = workflow_node_primary_message(Some("/work/proj"), &value, 1, 2, visible);
    match message {
        Message::Project(message::ProjectMessage::OpenProjectSessionPressed(path, id)) => {
            assert_eq!(path, "/work/proj");
            assert_eq!(id, "task-1-plan");
        }
        other => panic!("expected OpenProjectSessionPressed, got {other:?}"),
    }
}

#[test]
fn plan_run_node_click_keeps_detail_without_session_id() {
    let visible = "tool workflow_node\n{\"status\":\"completed\"}";
    let value = json!({
        "status": "completed",
        "metadata": {"plan_run": true, "title": "Plan"}
    });

    let message = workflow_node_primary_message(Some("/work/proj"), &value, 3, 4, visible);
    assert!(matches!(
        message,
        Message::Chat(message::ChatMessage::OpenToolDetail(3, 4, ref raw)) if raw == visible
    ));
}

#[test]
fn plan_run_node_click_falls_back_to_detail_without_project_path() {
    let visible = "tool workflow_node\n{\"status\":\"completed\"}";
    let value = json!({
        "metadata": {"session_id": "task-1-sub"}
    });

    for project_path in [None, Some(""), Some("   ")] {
        let message = workflow_node_primary_message(project_path, &value, 5, 6, visible);
        assert!(
            matches!(
                message,
                Message::Chat(message::ChatMessage::OpenToolDetail(5, 6, ref raw)) if raw == visible
            ),
            "expected detail fallback for project_path={project_path:?}"
        );
    }
}

#[test]
fn specialized_tool_actions_survive_header_migration() {
    let visible = "tool workflow_node\n{\"status\":\"completed\"}";
    let value = json!({
        "status": "completed",
        "metadata": {
            "plan_run": true,
            "session_id": "task-activity-row",
            "workflow_yaml": "nodes: []"
        }
    });

    assert!(matches!(
        workflow_node_primary_message(Some("/work/project"), &value, 5, 6, visible),
        Message::Project(message::ProjectMessage::OpenProjectSessionPressed(ref path, ref id))
            if path == "/work/project" && id == "task-activity-row"
    ));
    assert!(matches!(
        workflow_preview_message(&value),
        Some(Message::WorkflowTool(
            crate::apps::workflow::WorkflowMessage::OpenInlineYaml { .. }
        ))
    ));
}
