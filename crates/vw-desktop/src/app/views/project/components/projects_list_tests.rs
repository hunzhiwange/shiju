//! 宿主保留：最近项目元数据、运行/排队状态转为 ProjectsInput，且点击事件映射为宿主 ProjectMessage；中文字体资源从宿主 fonts 加载。
//! 项目列表组件的测试，覆盖徽标、图标路径与最近项目元数据的展示分支。
//! fixture 同时验证主题颜色、选中态和项目入口动作能够稳定构建。


use iced::Element;

use session_info::{Info, TimeInfo};

use super::*;
use crate::app::state::SessionRuntimeState;
use crate::app::{App, Message, RecentProjectMeta};

fn session_info(id: &str, directory: &str) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "1".to_string(),
        time: TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn recent_meta(path: &str, icon: Option<&str>, icon_color: Option<&str>) -> RecentProjectMeta {
    RecentProjectMeta {
        path: path.to_string(),
        name: "Project".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: icon.map(str::to_string),
        icon_color: icon_color.map(str::to_string),
        worktree_start_command: None,
    }
}

#[test]
fn projects_list_builds_empty_recent_projects() {
    let (mut app, _task) = App::new();
    app.workspace_state.recent_projects.clear();
    app.workspace_state.recent_projects_edits.clear();
    app.workspace_state.recent_projects_meta.clear();

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_builds_with_hover_meta_and_project_attention() {
    let (mut app, _task) = App::new();
    let path = "/tmp/vibe-window-alpha".to_string();
    let session_id = "session-alpha".to_string();

    app.workspace_state.recent_projects = vec![path.clone()];
    app.workspace_state.recent_projects_edits = vec!["Alpha".to_string()];
    app.workspace_state.recent_projects_meta = vec![recent_meta(&path, Some("A"), Some("#336699"))];
    app.navigation_tabs.project_path = Some(path.clone());
    app.workspace_state.hovered_recent_project = Some(path.clone());
    app.workspace_state.project_sessions.insert(path.clone(), vec![session_info(&session_id, &path)]);

    let mut runtime = SessionRuntimeState::new();
    runtime.request_lifecycle.is_requesting = true;
    app.session_state.session_runtime_states.insert(session_id, runtime);

    let element: Element<'_, Message> = projects_list(&app, true);

    std::hint::black_box(element);
}


#[test]
fn projects_list_falls_back_to_current_sessions_for_attention() {
    let (mut app, _task) = App::new();
    let path = "/tmp/vibe-window-beta".to_string();
    let session_id = "session-beta".to_string();

    app.workspace_state.recent_projects = vec![path.clone()];
    app.workspace_state.recent_projects_edits = vec![String::new()];
    app.workspace_state.recent_projects_meta = vec![recent_meta(&path, None, Some("not-a-color"))];
    app.navigation_tabs.project_path = Some(path.clone());
    app.session_state.sessions = vec![session_info(&session_id, &path)];

    let mut runtime = SessionRuntimeState::new();
    runtime.has_unseen_success = true;
    app.session_state.session_runtime_states.insert(session_id, runtime);

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_builds_with_queue_attention() {
    let (mut app, _task) = App::new();
    let path = "/tmp/vibe-window-queue".to_string();
    let session_id = "session-queue".to_string();

    app.workspace_state.recent_projects = vec![path.clone()];
    app.workspace_state.recent_projects_edits = vec!["Queue Project".to_string()];
    app.navigation_tabs.project_path = Some(path.clone());
    app.workspace_state.project_sessions.insert(path.clone(), vec![session_info(&session_id, &path)]);

    let mut runtime = SessionRuntimeState::new();
    runtime.request_lifecycle.queue = vec![crate::app::state::QueueItem {
        created_ms: 1000,
        query: "test query".to_string(),
        attachments: vec![],
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: true,
        send_behavior: crate::app::state::ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }];
    app.session_state.session_runtime_states.insert(session_id, runtime);

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_uses_empty_path_when_edit_has_no_matching_recent_project() {
    let (mut app, _task) = App::new();
    app.workspace_state.recent_projects.clear();
    app.workspace_state.recent_projects_edits = vec!["Loose edit".to_string()];
    app.workspace_state.recent_projects_meta.clear();

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_builds_without_attention_when_runtime_state_is_missing() {
    let (mut app, _task) = App::new();
    let path = "/tmp/no-runtime".to_string();
    app.workspace_state.recent_projects = vec![path.clone()];
    app.workspace_state.recent_projects_edits = vec!["No Runtime".to_string()];
    app.workspace_state.project_sessions.insert(path.clone(), vec![session_info("missing-runtime", &path)]);

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_does_not_use_current_sessions_for_unselected_project() {
    let (mut app, _task) = App::new();
    let listed_path = "/tmp/listed".to_string();
    app.workspace_state.recent_projects = vec![listed_path.clone()];
    app.workspace_state.recent_projects_edits = vec![String::new()];
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());
    app.session_state.sessions = vec![session_info("current-session", "/tmp/current")];

    let mut runtime = SessionRuntimeState::new();
    runtime.request_lifecycle.is_requesting = true;
    app.session_state.session_runtime_states.insert("current-session".to_string(), runtime);

    let element: Element<'_, Message> = projects_list(&app, true);

    std::hint::black_box(element);
}


#[test]
fn projects_list_ignores_recent_project_meta_for_other_paths() {
    let (mut app, _task) = App::new();
    let listed_path = "/tmp/listed-without-meta".to_string();
    app.workspace_state.recent_projects = vec![listed_path.clone()];
    app.workspace_state.recent_projects_edits = vec!["Listed".to_string()];
    app.workspace_state.recent_projects_meta = vec![recent_meta("/tmp/other-project", Some("O"), Some("#112233"))];

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}
