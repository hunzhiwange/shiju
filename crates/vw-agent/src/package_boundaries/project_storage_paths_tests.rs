use crate::memory::paths;

#[test]
fn project_storage_paths_production_boundary() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let primary = temp.path().join("primary");
    let linked = temp.path().join("linked");
    let repo = git2::Repository::init(&primary).expect("repo init");
    let tree_id = repo.index().expect("index").write_tree().expect("tree id");
    let tree = repo.find_tree(tree_id).expect("tree");
    let signature = git2::Signature::now("Boundary", "boundary@example.com").expect("signature");
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[]).expect("commit");
    repo.worktree("linked-boundary", &linked, None).expect("linked worktree");

    let project = paths::project_data_dir(&primary).expect("primary project data dir");
    let linked_project = paths::project_data_dir(&linked).expect("linked project data dir");
    assert_eq!(project, linked_project);
    assert_eq!(linked_project, project_storage_paths::paths::project_data_dir(&linked).unwrap());
    assert_eq!(paths::project_data_dir_best_effort(&linked), linked_project);
    assert!(!project.starts_with(temp.path()));
    assert!(project.components().any(|component| component.as_os_str() == "projects"));

    let workspace = paths::workspace_data_dir(&primary).expect("primary workspace data dir");
    let linked_workspace = paths::workspace_data_dir(&linked).expect("linked workspace data dir");
    assert_ne!(workspace, linked_workspace);
    assert_ne!(linked_workspace, linked_project);
    assert_eq!(
        linked_workspace,
        project_storage_paths::paths::workspace_data_dir(&linked).unwrap()
    );
    assert!(linked_workspace.components().any(|component| component.as_os_str() == "workspaces"));
}
