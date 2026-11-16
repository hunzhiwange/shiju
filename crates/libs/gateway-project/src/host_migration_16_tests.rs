use vw_api_project::ProjectStatus;
use crate::routes::*;
use agent_project as project;
use vw_api_transport::TimestampMs;
use vw_api_worktree::WorktreeStatus;
use base64::Engine;
use tempfile::TempDir;
use std::path::Path as FsPath;
fn project_info(id: &str, worktree: String, sandboxes: Vec<String>) -> project::Info {
    project::Info {
        id: id.to_string(),
        worktree,
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 10, updated: 20, initialized: None },
        sandboxes,
    }
}

fn write_file(path: &FsPath, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, content).expect("file write");
}

fn commit_all(repo: &git2::Repository, message: &str) {
    let mut index = repo.index().expect("index");
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).expect("add all");
    let tree_id = index.write_tree().expect("tree");
    let tree = repo.find_tree(tree_id).expect("tree lookup");
    let signature = git2::Signature::now("Vibe Window", "vibe@example.test").expect("signature");
    let parents = repo
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repo.find_commit(oid).ok())
        .into_iter()
        .collect::<Vec<_>>();
    let parent_refs = parents.iter().collect::<Vec<_>>();
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parent_refs)
        .expect("commit");
    index.write().expect("index write");
}

#[test]
fn normalize_path_removes_current_directory_prefix() {
    assert_eq!(normalize_path("./crates/vw-agent"), "crates/vw-agent");
    assert_eq!(normalize_path("/crates/vw-agent"), "crates/vw-agent");
    assert_eq!(normalize_path(r"\crates\vw-agent\"), "crates/vw-agent");
    assert_eq!(normalize_path("crates/vw-agent/"), "crates/vw-agent");
}

#[test]
fn normalize_path_handles_empty_root_and_repeated_edge_separators() {
    assert_eq!(normalize_path(""), "");
    assert_eq!(normalize_path("/"), "");
    assert_eq!(normalize_path("///crates/vw-agent///"), "crates/vw-agent");
}

#[test]
fn worktree_id_round_trips_directory() {
    let directory = "/tmp/vibe window/project";
    let id = worktree_id_from_directory(directory);

    assert_eq!(directory_from_worktree_id(&id).expect("decode"), directory);
}

#[test]
fn directory_from_worktree_id_rejects_invalid_values() {
    let bad_base64 = directory_from_worktree_id("not valid base64!");
    assert_eq!(
        bad_base64.expect_err("bad base64 should fail").status,
        axum::http::StatusCode::NOT_FOUND
    );

    let invalid_utf8 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0xff]);
    assert_eq!(
        directory_from_worktree_id(&invalid_utf8).expect_err("bad utf8 should fail").status,
        axum::http::StatusCode::NOT_FOUND
    );
}

#[test]
fn timestamp_ms_clamps_to_i64_range() {
    assert_eq!(timestamp_ms(42), TimestampMs(42));
    assert_eq!(timestamp_ms(u64::MAX), TimestampMs(i64::MAX));
}

#[test]
fn project_context_directory_prefers_worktree_then_sandbox() {
    let with_worktree =
        project_info("project-1", "/repo/main".to_string(), vec!["/repo/sandbox".to_string()]);
    assert_eq!(project_context_directory(&with_worktree).expect("worktree"), "/repo/main");

    let with_sandbox = project_info(
        "project-2",
        "   ".to_string(),
        vec!["   ".to_string(), "/repo/sandbox".to_string()],
    );
    assert_eq!(project_context_directory(&with_sandbox).expect("sandbox"), "/repo/sandbox");

    let missing = project_info("project-3", String::new(), vec![" ".to_string()]);
    let error = project_context_directory(&missing).expect_err("missing directory should fail");
    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "project directory missing");
}

#[test]
fn project_scope_rejects_blank_project_id() {
    let error = project_scope(project::default_tenant_scope(), " \t ".to_string())
        .expect_err("blank project id should fail");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.message.contains("project id"));
}

#[test]
fn project_owns_directory_normalizes_worktree_and_sandbox_paths() {
    let info = project_info(
        "project-owned-paths",
        r"C:\repo\main\".to_string(),
        vec!["/repo/sandbox/".to_string()],
    );

    assert!(project_owns_directory(&info, "C:/repo/main"));
    assert!(project_owns_directory(&info, "./repo/sandbox"));
    assert!(!project_owns_directory(&info, "/repo/unrelated"));
}

#[test]
fn map_project_uses_sandbox_directory_and_path_name_fallback() {
    let temp = TempDir::new().expect("tempdir");
    let directory = temp.path().join("alpha-project");
    std::fs::create_dir_all(&directory).expect("project dir");
    let info =
        project_info("project-map", String::new(), vec![directory.to_string_lossy().to_string()]);

    let dto = map_project(&info, project_host());

    assert_eq!(dto.id.0, "project-map");
    assert_eq!(dto.name, "alpha-project");
    assert_eq!(dto.directory, directory.to_string_lossy());
    assert_eq!(dto.status, ProjectStatus::Ready);
    assert_eq!(dto.created_at_ms, TimestampMs(10));
    assert_eq!(dto.updated_at_ms, TimestampMs(20));
    assert!(!dto.git.is_repo);
    assert_eq!(dto.session_count, Some(0));
}

#[test]
fn map_project_uses_explicit_non_blank_name() {
    let temp = TempDir::new().expect("tempdir");
    let mut info =
        project_info("project-named", temp.path().to_string_lossy().to_string(), Vec::new());
    info.name = Some("Named Project".to_string());

    let dto = map_project(&info, project_host());

    assert_eq!(dto.name, "Named Project");
    assert_eq!(dto.directory, temp.path().to_string_lossy());
}

#[test]
fn map_project_ignores_blank_explicit_name() {
    let temp = TempDir::new().expect("tempdir");
    let directory = temp.path().join("fallback-name");
    std::fs::create_dir_all(&directory).expect("project dir");
    let mut info =
        project_info("project-blank-name", directory.to_string_lossy().to_string(), Vec::new());
    info.name = Some(" \t ".to_string());

    let dto = map_project(&info, project_host());

    assert_eq!(dto.name, "fallback-name");
}

#[test]
fn map_project_falls_back_to_directory_when_no_name_component_exists() {
    let info = project_info("project-empty-directory", String::new(), Vec::new());

    let dto = map_project(&info, project_host());

    assert_eq!(dto.name, "");
    assert_eq!(dto.directory, "");
    assert_eq!(dto.display_path, "");
}

#[test]
fn map_worktree_derives_name_id_and_timestamps() {
    let temp = TempDir::new().expect("tempdir");
    let directory = temp.path().join("feature-tree");
    std::fs::create_dir_all(&directory).expect("worktree dir");
    let info = project_info("project-worktree", temp.path().to_string_lossy().to_string(), vec![]);

    let dto = map_worktree("project-worktree", &info, directory.to_string_lossy().to_string());

    assert_eq!(dto.project_id.0, "project-worktree");
    assert_eq!(dto.name, "feature-tree");
    assert_eq!(directory_from_worktree_id(&dto.id.0).expect("id should decode"), dto.directory);
    assert_eq!(dto.status, WorktreeStatus::Ready);
    assert_eq!(dto.created_at_ms, TimestampMs(10));
    assert_eq!(dto.updated_at_ms, TimestampMs(20));
}

#[test]
fn current_branch_and_change_records_are_empty_for_non_repo() {
    let temp = TempDir::new().expect("tempdir");

    assert!(current_branch(&temp.path().to_string_lossy()).is_none());
    assert!(collect_project_change_records(&temp.path().to_string_lossy()).is_empty());
}

#[test]
fn current_branch_reads_committed_repository_branch() {
    let temp = TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    write_file(&temp.path().join("README.md"), "demo\n");
    commit_all(&repo, "initial");

    let branch = current_branch(&temp.path().to_string_lossy()).expect("branch");
    assert!(!branch.is_empty());
    assert_ne!(branch, "HEAD");
}

#[test]
fn collect_project_change_records_empty_for_unborn_git_repo() {
    let temp = TempDir::new().expect("tempdir");
    git2::Repository::init(temp.path()).expect("repo init");

    assert!(collect_project_change_records(&temp.path().to_string_lossy()).is_empty());
}

#[test]
fn collect_project_change_records_groups_modified_files() {
    let temp = TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    write_file(&temp.path().join("src/main.rs"), "fn main() {}\n");
    commit_all(&repo, "initial");

    write_file(&temp.path().join("src/main.rs"), "fn main() {\n    println!(\"hi\");\n}\n");
    write_file(&temp.path().join("README.md"), "# demo\n");

    let items = collect_project_change_records(&temp.path().to_string_lossy());

    assert!(
        items
            .iter()
            .any(|item| { item.path == "src/main.rs" && item.patch.contains("println!(\"hi\")") })
    );
}

#[test]
fn collect_project_change_records_includes_nested_untracked_files() {
    let temp = TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    write_file(&temp.path().join("README.md"), "clean\n");
    commit_all(&repo, "initial");

    write_file(&temp.path().join("src/generated/new_file.rs"), "pub fn created() {}\n");

    let items = collect_project_change_records(&temp.path().to_string_lossy());
    let item = items
        .iter()
        .find(|item| item.path == "src/generated/new_file.rs")
        .expect("nested untracked file should be reported");

    assert!(item.patch.contains("pub fn created() {}"));
}

#[test]
fn map_project_reports_git_branch_and_dirty_state() {
    let temp = TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    write_file(&temp.path().join("README.md"), "clean\n");
    commit_all(&repo, "initial");
    write_file(&temp.path().join("README.md"), "dirty\n");
    let mut info = project_info("project-git", temp.path().to_string_lossy().to_string(), vec![]);
    info.vcs = Some(project::Vcs::Git);

    let dto = map_project(&info, project_host());

    assert!(dto.git.is_repo);
    assert!(dto.git.has_uncommitted_changes);
    assert!(dto.current_branch.is_some());
}

#[test]
fn map_worktree_uses_default_name_for_blank_directory() {
    let info = project_info("project-worktree-default", String::new(), vec![]);

    let dto = map_worktree("project-worktree-default", &info, String::new());

    assert_eq!(dto.name, "worktree");
    assert_eq!(dto.branch, "");
}

// These mapping tests inject only the session-count port; bootstrapping is not used.
fn project_host() -> crate::ProjectHost {
    crate::ProjectHost {
        session_count: |_| 0,
        instance_bootstrap: |_| panic!("mapping must not bootstrap an instance"),
    }
}
