use desktop_workflow_model::{self as model, load_document_from_value};
use desktop_workflow_state::state::{
    WorkflowSavedAppSummary, WorkflowState,
    video_edit::{TEMPLATE_ID, VideoEditUiState},
};
use iced::widget::text_editor;
use vw_api_workflow::WorkflowRecord;
fn record(yaml: &str) -> WorkflowRecord {
    WorkflowRecord {
        uuid: TEMPLATE_ID.into(),
        name: "通用云端视频剪辑".into(),
        description: String::new(),
        workflow_yaml: yaml.into(),
        created_at_ms: 1,
        updated_at_ms: 2,
    }
}
fn yaml() -> String {
    include_str!("../../../workflow-template-contract/resources/cloud_video_edit.yaml").into()
}

#[test]
fn video_edit_ui_rejects_invalid_shared_parallel_limit() {
    for limit in [0, 6, 10] {
        let root = serde_yaml::from_str(&format!("app: {{name: 通用云端视频剪辑}}\nworkflow:\n  video_edit: {{max_parallel: {limit}}}\n  graph: {{nodes: [], edges: []}}\n")).unwrap();
        assert!(load_document_from_value(None, root).is_err(), "accepted limit {limit}");
    }
}
#[test]
fn video_edit_ui_preserves_metadata_and_defaults_through_graph_save() {
    let source = yaml();
    let loaded = model::load_document_from_text(None, source).unwrap();
    let output = model::serialize_workflow_yaml(
        &loaded.app_meta,
        &loaded.document,
        &loaded.environment_variables,
        &loaded.conversation_variables,
        &loaded.raw_root,
        model::WorkflowViewport::default(),
    )
    .unwrap();
    let raw: serde_yaml::Value = serde_yaml::from_str(&output).unwrap();
    assert_eq!(raw["workflow"]["video_edit"]["max_parallel"].as_u64(), Some(5));
    let planning = raw["workflow"]["graph"]["nodes"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|n| n["id"].as_str() == Some("planning"))
        .unwrap();
    assert_eq!(planning["data"]["max_parallel"].as_u64(), Some(5));
    assert_eq!(planning["data"]["graph"]["nodes"][0]["id"].as_str(), Some("model"));
    assert!(
        raw["workflow"]["graph"]["nodes"]
            .as_sequence()
            .unwrap()
            .iter()
            .any(|n| n["data"]["provider"].as_str() == Some("shiju_video"))
    );
    let mut state = VideoEditUiState::default();
    state.accept_record(record(&output));
    let edited = output.replace("max_parallel: 5", "max_parallel: 2");
    state.editor = Some(text_editor::Content::with_text(&edited));
    assert!(state.validated_yaml().unwrap().contains("max_parallel: 2"));
}
#[test]
fn video_edit_ui_native_command_preserves_chinese_path() {
    let state = VideoEditUiState { path: "/素材/中文 采访.mp4".into(), ..Default::default() };
    assert_eq!(state.native_command().unwrap(), "/shiju 剪辑 \"/素材/中文 采访.mp4\"");
    assert!(
        VideoEditUiState { path: "bad\npath".into(), ..Default::default() }
            .native_command()
            .is_err()
    );
}
#[test]
fn video_edit_ui_source_list_exposes_server_record() {
    let mut state = WorkflowState::default();
    state.finish_saved_apps_load(Ok(vec![WorkflowSavedAppSummary {
        uuid: TEMPLATE_ID.into(),
        name: "通用云端视频剪辑".into(),
        description: String::new(),
        created_at_ms: 1,
        updated_at_ms: 2,
    }]));
    assert_eq!(state.saved_apps[0].uuid, TEMPLATE_ID);
}
#[test]
fn video_edit_ui_rejects_nested_iteration_limit_and_keeps_draft_version() {
    let source = yaml()
        .replacen("max_parallel: 5", "max_parallel: 3", 1)
        .replace("max_parallel: 5", "max_parallel: 10");
    assert!(model::load_document_from_text(None, source).is_err());
    let mut state = VideoEditUiState::default();
    state.accept_record(record(&yaml()));
    state.editor = Some(text_editor::Content::with_text("unsaved draft"));
    let mut newer = record(&yaml());
    newer.updated_at_ms = 99;
    state.accept_record(newer);
    assert_eq!(state.record.unwrap().updated_at_ms, 2);
}
#[test]
fn video_edit_ui_tenant_switch_restores_own_unsaved_draft_without_restarting_run() {
    let a = vw_api_transport::TenantScope::new("ui", "a").unwrap();
    let b = vw_api_transport::TenantScope::new("ui", "b").unwrap();
    let mut state = WorkflowState::default();
    state.select_tenant(a.clone());
    state.video_edit.editor = Some(text_editor::Content::with_text("tenant A 未保存"));
    state.running_saved_app_uuid = Some("old-run".into());
    state.select_tenant(b);
    assert!(state.video_edit.editor.is_none());
    state.video_edit.editor = Some(text_editor::Content::with_text("tenant B draft"));
    state.select_tenant(a);
    assert_eq!(state.video_edit.editor.unwrap().text(), "tenant A 未保存");
    assert!(state.running_saved_app_uuid.is_none());
}
