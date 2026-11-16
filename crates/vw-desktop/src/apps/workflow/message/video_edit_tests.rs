//! Built-in template validation and editor behavior.



use super::*;
use crate::apps::workflow::{
    model,
    state::{
        WorkflowState,
        video_edit::{TEMPLATE_ID, VideoEditUiState},
    },
};
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
    include_str!("../../../../../libs/workflow-template-contract/resources/cloud_video_edit.yaml").into()
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
fn video_edit_ui_stale_tenant_reply_cannot_replace_current_template() {
    let mut app = App::new().0;
    sync_video_tenant(&mut app);
    let other =
        vw_api_transport::TenantScope::new("10000", "other-company").unwrap();
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
        vw_api_transport::TenantScope::new("10000", "previous-company").unwrap(),
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
