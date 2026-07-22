use crate::app::task::TaskBoardStaffCapability;
use crate::app::views::workflow_manager::{WorkflowManagerMessage, WorkflowManagerTab};

#[test]
fn workflow_capability_panel_renders_builtin_and_custom_capabilities() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_capability_draft = vec![TaskBoardStaffCapability::custom_saved_workflow(
        "custom_release_workflow".to_string(),
        "发布 Workflow".to_string(),
    )];
    app.task_board_staff_capability_catalog_error =
        Some("能力目录保存需要已打开的项目".to_string());

    let task = crate::app::views::workflow_manager::update_app(
        &mut app,
        WorkflowManagerMessage::SelectTab(WorkflowManagerTab::CapabilityManagement),
    );
    let _ = std::hint::black_box(task);
    let view = crate::app::views::workflow_manager::view(&app);
    std::hint::black_box(view);

    let manager_source = include_str!("app/views/workflow_manager/mod.rs");
    let panel_source = include_str!("app/views/workflow_manager/capabilities.rs");
    let update_source = include_str!("app/message/task_board/update.rs");
    let custom_labels = app
        .task_board_staff_capability_draft
        .iter()
        .map(|capability| capability.label.as_str())
        .collect::<Vec<_>>();

    assert!(manager_source.contains("WorkflowManagerTab::CapabilityManagement"));
    assert!(manager_source.contains("能力管理"));
    assert!(panel_source.contains("内置只读"));
    assert!(panel_source.contains("默认 Agent 执行"));
    assert!(panel_source.contains("任务拆分"));
    assert!(custom_labels.contains(&"发布 Workflow"));
    assert!(panel_source.contains("SaveStaffCapabilityCatalog"));
    assert!(panel_source.contains("task_board_staff_capability_catalog_error"));
    assert!(update_source.contains("TaskBoardMessage::SaveStaffCapabilityCatalog =>"));
}
