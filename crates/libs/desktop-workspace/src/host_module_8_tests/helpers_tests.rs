use crate::project_message::data::{find_session_project_path, map_loaded_project_info};
use vw_api_project::{ProjectDto, ProjectGitStateDto, ProjectStatus};
use vw_api_transport::{common::TimestampMs, id::ProjectId};
fn project_dto(current_branch: Option<&str>) -> ProjectDto {
    ProjectDto {
        id: ProjectId("project-id".to_string()),
        name: "Gateway Project".to_string(),
        directory: "/project".to_string(),
        display_path: "/project".to_string(),
        status: ProjectStatus::Ready,
        created_at_ms: TimestampMs(-100),
        updated_at_ms: TimestampMs(5000),
        default_branch: Some("main".to_string()),
        current_branch: current_branch.map(ToString::to_string),
        git: ProjectGitStateDto {
            is_repo: true,
            has_uncommitted_changes: false,
            ahead: None,
            behind: None,
        },
        active_worktree_id: None,
        session_count: Some(2),
    }
}
#[test]
fn find_session_project_path_falls_back_to_active_project() {
    let sessions = std::collections::HashMap::new();
    let mut project_path: Option<String>;
    project_path = Some("/active/project".to_string());

    assert_eq!(
        find_session_project_path(&sessions, project_path.as_deref(), "missing-session"),
        Some("/active/project".to_string())
    );

    project_path = None;
    assert_eq!(
        find_session_project_path(&sessions, project_path.as_deref(), "missing-session"),
        None
    );
}

#[test]
fn map_loaded_project_info_drops_blank_current_branch() {
    let loaded = map_loaded_project_info("/current".to_string(), project_dto(Some("  ")));

    assert_eq!(loaded.project_path, "/current");
    assert_eq!(loaded.info.id, "project-id");
    assert!(loaded.current_branch.is_none());
}

#[test]
fn map_loaded_project_info_keeps_non_blank_current_branch() {
    let loaded = map_loaded_project_info("/current".to_string(), project_dto(Some("feature")));

    assert_eq!(loaded.current_branch.as_deref(), Some("feature"));
}
