//! 通道工作项目与首页项目列表保持一致，不展示内部 workspace。
use super::project_choices;
use vw_gateway_client::vw_api_types::project::ProjectDto;

fn project(id: &str, name: &str, directory: &str, updated: u64) -> ProjectDto {
    serde_json::from_value(serde_json::json!({
        "id": id, "name": name, "directory": directory, "display_path": directory,
        "status": "ready", "created_at_ms": 0, "updated_at_ms": updated,
        "git": {"is_repo": false, "has_uncommitted_changes": false}
    }))
    .unwrap()
}

#[test]
fn project_choices_lists_two_user_projects_without_internal_workspaces() {
    let choices = project_choices(vec![
        project("internal", "workspace", "/data/.workspace", 30),
        project("one", "项目一", "/data/one", 10),
        project("two", "项目二", "/data/two", 20),
        project("runtime", "runtime", "/data/.runtime", 40),
    ]);
    assert_eq!(choices, vec![("two".into(), "项目二".into()), ("one".into(), "项目一".into())]);
}
