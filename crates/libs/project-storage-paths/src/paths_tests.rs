// 验证记忆数据目录在普通仓库、工作树与回退场景下的作用域。

// linked worktree 与缺失路径分别验证规范化和临时目录回退不会混淆作用域。
use super::*;
use git2::Repository;
use std::path::Path;
use tempfile::TempDir;

fn commit_tree(repo: &Repository, message: &str) {
    let tree_id = {
        let mut index = repo.index().expect("index");
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).expect("add all");
        index.write().expect("index write");
        index.write_tree().expect("tree")
    };
    let tree = repo.find_tree(tree_id).expect("tree");
    let signature = git2::Signature::now("Vibe Window", "vw@example.com").expect("signature");
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[]).expect("commit");
}

fn repo_with_initial_commit(path: &Path) -> Repository {
    let repo = Repository::init(path).expect("repo init");
    std::fs::write(path.join("README.md"), "demo\n").expect("readme");
    commit_tree(&repo, "initial");
    repo
}

fn add_linked_worktree(repo: &Repository, name: &str, path: &Path) {
    let mut options = git2::WorktreeAddOptions::new();
    repo.worktree(name, path, Some(&mut options)).expect("linked worktree");
}

#[test]
fn project_data_dir_is_user_scoped() {
    let workspace = TempDir::new().unwrap();
    let path = project_data_dir(workspace.path()).unwrap();
    let home = directories::UserDirs::new().unwrap().home_dir().to_path_buf();

    assert!(path.starts_with(vw_config_types::paths::home_config_dir(home).join("worktree")));
    assert!(!path.starts_with(workspace.path()));
    assert!(path.components().any(|component| component.as_os_str() == "projects"));
}

#[test]
fn project_and_workspace_data_dirs_use_different_scopes() {
    let workspace = TempDir::new().unwrap();

    let project = project_data_dir(workspace.path()).unwrap();
    let worktree = workspace_data_dir(workspace.path()).unwrap();

    assert!(project.components().any(|component| component.as_os_str() == "projects"));
    assert!(worktree.components().any(|component| component.as_os_str() == "workspaces"));
    assert_ne!(project, worktree);
}

#[test]
fn best_effort_uses_normal_project_dir_when_home_is_available() {
    let workspace = TempDir::new().unwrap();

    assert_eq!(
        project_data_dir_best_effort(workspace.path()),
        project_data_dir(workspace.path()).unwrap()
    );
}

#[test]
fn fallback_project_data_dir_uses_temp_worktree_projects_scope() {
    let workspace = TempDir::new().unwrap();

    let fallback = fallback_project_data_dir(workspace.path());

    assert!(fallback.starts_with(
        std::env::temp_dir().join(vw_config_types::paths::APP_DIR_NAME).join("worktree")
    ));
    assert!(fallback.components().any(|component| component.as_os_str() == "projects"));
    assert!(fallback.file_name().and_then(|name| name.to_str()).unwrap().contains('-'));
}

#[test]
fn canonical_path_keeps_absolute_missing_paths_without_fs_errors() {
    let workspace = TempDir::new().unwrap();
    let missing = workspace.path().join("missing").join("child");

    assert_eq!(canonical_path(&missing), missing);
}

#[test]
fn canonical_path_resolves_existing_relative_paths() {
    let expected = std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap();

    assert_eq!(canonical_path(Path::new(".")), expected);
}

#[test]
fn canonical_path_keeps_relative_missing_paths_absolute_to_current_dir() {
    let relative = Path::new("target").join("definitely-missing-memory-path");
    let expected = std::env::current_dir().unwrap().join(&relative);

    assert_eq!(canonical_path(&relative), expected);
}

#[test]
fn git_common_dir_returns_none_outside_git_repositories() {
    let workspace = TempDir::new().unwrap();

    assert!(git_common_dir(workspace.path()).is_none());
}

#[test]
fn git_common_dir_returns_none_when_working_directory_is_missing() {
    let workspace = TempDir::new().unwrap();
    let missing = workspace.path().join("missing");

    assert!(git_common_dir(&missing).is_none());
}

#[test]
fn git_common_dir_returns_absolute_common_dir_inside_repo() {
    let workspace = TempDir::new().unwrap();
    repo_with_initial_commit(workspace.path());

    let common_dir = git_common_dir(workspace.path()).unwrap();

    assert!(common_dir.is_absolute());
    assert_eq!(common_dir.file_name().and_then(|name| name.to_str()), Some(".git"));
}

#[test]
fn project_state_id_uses_git_common_dir_when_available() {
    let workspace = TempDir::new().unwrap();
    let primary = workspace.path().join("primary");
    let linked = workspace.path().join("linked");
    std::fs::create_dir(&primary).expect("primary dir");
    let repo = repo_with_initial_commit(&primary);
    add_linked_worktree(&repo, "linked-memory", &linked);

    let common_dir = git_common_dir(&primary).unwrap();

    assert_eq!(project_state_id(&linked), scoped_id(&common_dir));
    assert_eq!(project_state_id(&linked), project_state_id(&primary));
}

#[test]
fn workspace_state_id_uses_workspace_path_not_git_common_dir() {
    let workspace = TempDir::new().unwrap();
    repo_with_initial_commit(workspace.path());

    let canonical_workspace = canonical_path(workspace.path());

    assert_eq!(workspace_state_id(workspace.path()), scoped_id(&canonical_workspace));
    assert_ne!(workspace_state_id(workspace.path()), project_state_id(workspace.path()));
}

#[test]
fn scoped_id_is_deterministic_and_keeps_only_a_short_hash_suffix() {
    let workspace = Path::new("/tmp/My Workspace!");
    let id = scoped_id(workspace);

    assert_eq!(id, scoped_id(workspace));
    assert!(id.starts_with("my-workspace-"));
    assert_eq!(id.rsplit_once('-').unwrap().1.len(), 16);
}

#[test]
fn safe_name_replaces_unsafe_chars_and_falls_back_for_empty_names() {
    assert_eq!(safe_name(" Project_01 "), "project_01");
    assert_eq!(safe_name("a.b/c"), "a-b-c");
    assert_eq!(safe_name("..."), "workspace");
}

#[test]
fn git_common_dir_does_not_start_system_git() {
    assert!(!include_str!("paths.rs").contains("Command::new(\"git\")"));
}
