//! Built-in template validation and editor behavior.
use crate::apps::workflow::model::load_document_from_value;

#[test]
fn video_edit_ui_rejects_invalid_shared_parallel_limit() {
    for limit in [0, 6, 10] {
        let root = serde_yaml::from_str(&format!("app: {{name: 通用云端视频剪辑}}\nworkflow:\n  video_edit: {{max_parallel: {limit}}}\n  graph: {{nodes: [], edges: []}}\n")).unwrap();
        assert!(load_document_from_value(None, root).is_err(), "accepted limit {limit}");
    }
}

use super::*;
use crate::apps::workflow::{
    model,
    state::{
        WorkflowState,
        video_edit::{TEMPLATE_ID, VideoEditUiState},
    },
};
use vw_gateway_client::WorkflowRecord;
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
    include_str!("../../../../../vw-agent/resources/workflows/cloud_video_edit.yaml").into()
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
fn video_edit_ui_refresh_and_reopen_preserve_unsaved_draft() {
    let mut state = VideoEditUiState::default();
    state.accept_record(record(&yaml()));
    state.editor = Some(text_editor::Content::with_text("未保存草稿"));
    state.accept_record(record(&yaml()));
    assert_eq!(state.editor.as_ref().unwrap().text(), "未保存草稿");
    let mut app = App::new().0;
    app.workflow_state.video_edit = state;
    let _ = update(&mut app, WorkflowMessage::CloseVideoTemplate);
    let _ = update(&mut app, WorkflowMessage::OpenVideoTemplate);
    assert_eq!(app.workflow_state.video_edit.editor.as_ref().unwrap().text(), "未保存草稿");
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
fn video_edit_ui_stale_tenant_reply_cannot_replace_current_template() {
    let mut app = App::new().0;
    sync_video_tenant(&mut app);
    let other =
        vw_gateway_client::vw_api_types::TenantScope::new("10000", "other-company").unwrap();
    let _ = update(
        &mut app,
        WorkflowMessage::VideoTemplateLoaded {
            tenant: other,
            graph: false,
            result: Ok(record(&yaml())),
        },
    );
    assert!(app.workflow_state.video_edit.record.is_none());
    app.workflow_state.video_edit.tenant = Some(
        vw_gateway_client::vw_api_types::TenantScope::new("10000", "previous-company").unwrap(),
    );
    app.workflow_state.saved_app_search_query = "private".into();
    sync_video_tenant(&mut app);
    assert!(app.workflow_state.saved_app_search_query.is_empty());
}
#[test]
fn video_edit_ui_builtin_run_and_delete_use_safe_actions() {
    let mut app = App::new().0;
    let _ = update(&mut app, WorkflowMessage::RequestDeleteSavedApp(TEMPLATE_ID.into()));
    assert!(app.workflow_state.confirm_delete_saved_app_uuid.is_none());
    app.workflow_state.saved_app_run_prompt =
        Some(crate::apps::workflow::state::WorkflowRunPromptDraft {
            uuid: TEMPLATE_ID.into(),
            query: "video".into(),
            ..Default::default()
        });
    let _ = update(&mut app, WorkflowMessage::SubmitSavedAppRunPrompt);
    assert!(app.workflow_state.running_saved_app_uuid.is_none());
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
fn video_edit_ui_start_uses_existing_native_chat_queue_and_permissions() {
    let mut app = App::new().0;
    app.acp_agent = None;
    app.project_path = Some("/原生项目".into());
    app.active_session_id = Some("ses_video_ui".into());
    app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
    {
        let runtime = app.current_session_runtime_mut();
        runtime.is_requesting = true;
        runtime.task_mode_enabled = false;
        runtime.workflow_mode_enabled = false;
        runtime.acp_agent = None;
        runtime.full_access_enabled = false;
        runtime.input_editor = text_editor::Content::new();
    }
    app.workflow_state.video_edit.path = "/素材/中文 采访.mp4".into();
    let _ = update(&mut app, WorkflowMessage::StartNativeVideoEdit);
    let runtime = app.current_session_runtime();
    assert_eq!(runtime.queue.len(), 1);
    assert!(runtime.queue[0].query.starts_with("/shiju 剪辑 \"/素材/中文 采访.mp4\""));
    assert!(!runtime.queue[0].acp_test);
    assert!(!runtime.queue[0].full_access_enabled);
    assert!(app.workflow_state.running_saved_app_uuid.is_none());
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
fn video_edit_ui_restore_requires_confirmation_and_cancel_keeps_yaml() {
    let mut app = App::new().0;
    app.workflow_state.video_edit.accept_record(record(&yaml()));
    let before = app.workflow_state.video_edit.editor.as_ref().unwrap().text();
    let _ = update(&mut app, WorkflowMessage::ConfirmVideoTemplateRestore);
    assert!(!app.workflow_state.video_edit.busy);
    let _ = update(&mut app, WorkflowMessage::RequestVideoTemplateRestore);
    assert!(app.workflow_state.video_edit.restore_confirm);
    let _ = update(&mut app, WorkflowMessage::CancelVideoTemplateRestore);
    assert!(!app.workflow_state.video_edit.restore_confirm);
    assert_eq!(app.workflow_state.video_edit.editor.as_ref().unwrap().text(), before);
}

#[test]
fn video_edit_ui_tenant_switch_restores_own_unsaved_draft_without_restarting_run() {
    let a = vw_gateway_client::vw_api_types::TenantScope::new("ui", "a").unwrap();
    let b = vw_gateway_client::vw_api_types::TenantScope::new("ui", "b").unwrap();
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
