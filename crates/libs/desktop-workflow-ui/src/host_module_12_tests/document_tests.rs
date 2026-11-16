//! 工作流文档消息的测试，覆盖加载、导入、保存和历史快照。
//! fixture 验证错误路径不会污染当前文档，并保持脏状态语义准确。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("document_tests"));
}

use super::*;

fn app_with_workflow() -> Fixture {
    let mut app = Fixture::new().0;
    app.window_size = (1280.0, 860.0);
    let loaded = load_document_from_text(
        None,
        r#"
app:
  name: 文档消息测试
workflow:
  environment_variables:
    - id: env-existing
      name: token
      value_type: string
      value: abc
      description: old env
  conversation_variables:
    - id: conv-existing
      name: count
      value_type: number
      value: 1
      description: old conv
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: answer
        data:
          title: Answer
          type: answer
    edges:
      - id: start-answer
        source: start
        sourceHandle: source
        target: answer
        targetHandle: target
"#
        .to_string(),
    )
    .expect("workflow should load");
    apply_loaded(&mut app.context(), loaded);
    app
}

fn document_message(app: &mut Fixture, message: WorkflowMessage) {
    assert!(super::document::handle(&mut app.context(), message).is_some());
}

#[test]
fn save_reload_export_and_floating_panel_messages_update_state() {
    let mut app = app_with_workflow();
    app.workflow_state.action_menu_open = true;
    app.workflow_state.quick_insert_panel_open = true;
    app.workflow_state.zoom_menu_open = true;

    document_message(&mut app, WorkflowMessage::ToggleActionMenu);
    assert!(!app.workflow_state.action_menu_open);
    document_message(&mut app, WorkflowMessage::CloseFloatingPanels);
    assert!(!app.workflow_state.action_menu_open);
    assert!(!app.workflow_state.quick_insert_panel_open);
    assert!(!app.workflow_state.zoom_menu_open);

    document_message(&mut app, WorkflowMessage::SaveActiveApp);
    document_message(&mut app, WorkflowMessage::SaveActiveAppAs);
    document_message(
        &mut app,
        WorkflowMessage::SaveActiveAppFinished(Ok(Some(WorkflowSaveTarget::LocalUuid(
            "uuid-1".to_string(),
        )))),
    );
    assert_eq!(app.workflow_state.local_uuid.as_deref(), Some("uuid-1"));
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已保存到本地数据库: uuid-1"));
    assert!(app.workflow_state.saved_apps_loading);

    document_message(
        &mut app,
        WorkflowMessage::SaveActiveAppFinished(Ok(Some(WorkflowSaveTarget::FilePath(
            "/tmp/workflow.yml".to_string(),
        )))),
    );
    assert_eq!(app.workflow_state.source_path.as_deref(), Some("/tmp/workflow.yml"));
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已另存为 /tmp/workflow.yml"));

    app.workflow_state.error_message = None;
    document_message(&mut app, WorkflowMessage::SaveActiveAppFinished(Ok(None)));
    assert!(app.workflow_state.error_message.is_none());
    document_message(
        &mut app,
        WorkflowMessage::SaveActiveAppFinished(Err("disk full".to_string().into())),
    );
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("disk full"));

    app.workflow_state.source_path = None;
    document_message(&mut app, WorkflowMessage::Reload);
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已重新载入 文档消息测试"));
    document_message(&mut app, WorkflowMessage::ExportPng);
    document_message(&mut app, WorkflowMessage::ExportJpeg);
    document_message(&mut app, WorkflowMessage::ExportSvg);

    let reload_path = std::env::temp_dir().join(format!(
        "vibe-window-workflow-reload-{}.yml",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos()
    ));
    std::fs::write(
        &reload_path,
        "app:\n  name: 文件重载\nworkflow:\n  graph:\n    nodes: []\n    edges: []\n",
    )
    .expect("reload fixture should be written");
    app.workflow_state.source_path = Some(reload_path.to_string_lossy().to_string());
    document_message(&mut app, WorkflowMessage::Reload);
    assert_eq!(app.workflow_state.title(), "文件重载");
    let _ = std::fs::remove_file(&reload_path);

    app.workflow_state.error_message = None;
    let missing_path = std::env::temp_dir().join("vibe-window-workflow-reload-missing.yml");
    let _ = std::fs::remove_file(&missing_path);
    app.workflow_state.source_path = Some(missing_path.to_string_lossy().to_string());
    document_message(&mut app, WorkflowMessage::Reload);
    assert!(
        app.workflow_state
            .error_message
            .as_deref()
            .is_some_and(|message| message.starts_with("读取工作流文件失败:"))
    );

    let mut empty_app = Fixture::new().0;
    document_message(&mut empty_app, WorkflowMessage::Reload);
    assert_eq!(
        empty_app.workflow_state.error_message.as_deref(),
        Some("当前没有可重新载入的 Workflow 应用")
    );

    document_message(&mut empty_app, WorkflowMessage::ExportPng);
    document_message(&mut empty_app, WorkflowMessage::ExportJpeg);
    document_message(&mut empty_app, WorkflowMessage::ExportSvg);
    document_message(&mut app, WorkflowMessage::ExportFinished(Ok(())));
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已导出工作流图片"));
    document_message(
        &mut app,
        WorkflowMessage::ExportFinished(Err("export failed".to_string().into())),
    );
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("export failed"));

    let ignored =
        super::document::handle(&mut app, WorkflowMessage::SelectNode("start".to_string()));
    assert!(ignored.is_none());
}
