use super::*;
use crate::app::App;
#[test]
fn video_edit_ui_start_uses_existing_native_chat_queue_and_permissions() {
    let mut app = App::new().0;
    app.session_state.acp_agent = None;
    app.navigation_tabs.project_path = Some("/原生项目".into());
    app.session_state.active_session_id = Some("ses_video_ui".into());
    app.session_state.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
    {
        let runtime = app.current_session_runtime_mut();
        runtime.request_lifecycle.is_requesting = true;
        runtime.task_mode_enabled = false;
        runtime.workflow_mode_enabled = false;
        runtime.acp_agent = None;
        runtime.full_access_enabled = false;
        runtime.input_editor = text_editor::Content::new();
    }
    app.workflow_state.video_edit.path = "/素材/中文 采访.mp4".into();
    let _ = start_native_video_edit(&mut app);
    let runtime = app.current_session_runtime();
    assert_eq!(runtime.request_lifecycle.queue.len(), 1);
    assert!(runtime.request_lifecycle.queue[0].query.starts_with("/shiju 剪辑 \"/素材/中文 采访.mp4\""));
    assert!(!runtime.request_lifecycle.queue[0].acp_test);
    assert!(!runtime.request_lifecycle.queue[0].full_access_enabled);
    assert!(app.workflow_state.running_saved_app_uuid.is_none());
}
