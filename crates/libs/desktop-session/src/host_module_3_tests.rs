//! 流事件控制器的领域契约；宿主仅保留真实跨域效果的装配断言。
use crate::message::chat::stream::{draft, projection};
use crate::session::agent::SessionRuntimeState;

fn info(id: &str, title: &str, project: &str) -> session_info::Info {
    session_info::Info {
        id: id.into(),
        slug: format!("{id}-slug"),
        project_id: project.into(),
        directory: "/project".into(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.into(),
        version: "0.0.0".into(),
        time: session_info::TimeInfo { created: 10, updated: 20, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn desktop_first_response_title_updates_only_untitled_copies_for_its_project() {
    let mut sessions = vec![
        info("session-1", "手工改名", "project-1"),
        info("session-2", "当前会话", "project-1"),
    ];
    let mut project_sessions =
        vec![info("session-1", "", "project-1"), info("session-1", "", "foreign-project")];
    assert!(draft::apply_published_title(
        sessions.iter_mut().chain(project_sessions.iter_mut()),
        &info("session-1", "服务端标题", "project-1")
    ));
    assert_eq!(sessions[0].title, "手工改名");
    assert_eq!(sessions[1].title, "当前会话");
    assert_eq!(project_sessions[0].title, "服务端标题");
    assert!(project_sessions[1].title.is_empty());
}

#[test]
fn cancelled_runtime_clears_request_and_animation() {
    let mut runtime = SessionRuntimeState::<()>::default();
    runtime.request_lifecycle.is_requesting = true;
    runtime.request_lifecycle.submit_anim = 4;
    runtime.has_unseen_success = true;
    projection::cancel_runtime(&mut runtime);
    assert!(!runtime.request_lifecycle.is_requesting);
    assert!(runtime.request_lifecycle.active_agent_request.is_none());
    assert!(!runtime.has_unseen_success);
    assert_eq!(runtime.request_lifecycle.submit_anim, 0);
}

#[path = "stream_event_transition_tests.rs"]
mod stream_events;
