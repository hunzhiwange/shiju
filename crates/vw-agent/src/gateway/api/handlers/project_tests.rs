use super::*;
use axum::body::Body;
use axum::extract::{Extension, Path, Query};
use axum::http::{Request, StatusCode};
use std::ffi::OsString;
use std::path::Path as FsPath;
use tempfile::TempDir;
use tower::ServiceExt;
use uuid::Uuid;

struct PathGuard {
    old: Option<OsString>,
    _bin_dir: TempDir,
}

impl PathGuard {
    /// 用仅含失败 stub 的 PATH 屏蔽真实 `git` 可执行文件。
    ///
    /// 仅清空 PATH 不够：`git_std_command` 会通过补全 PATH / 常见目录
    /// 再次解析到系统 Git。
    fn without_git() -> Self {
        let bin_dir = TempDir::new().expect("git stub bin dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let stub = bin_dir.path().join("git");
            std::fs::write(&stub, "#!/bin/sh\necho 'git stub: blocked' >&2\nexit 1\n")
                .expect("write git stub");
            let mut perms = std::fs::metadata(&stub).expect("stub metadata").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&stub, perms).expect("stub permissions");
        }
        #[cfg(windows)]
        {
            let stub = bin_dir.path().join("git.cmd");
            std::fs::write(&stub, "@echo off\r\necho git stub: blocked 1>&2\r\nexit /b 1\r\n")
                .expect("write git stub");
        }
        let old = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", bin_dir.path()) };
        Self { old, _bin_dir: bin_dir }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => unsafe { std::env::set_var("PATH", value) },
            None => unsafe { std::env::remove_var("PATH") },
        }
    }
}

#[test]
fn router_builds_with_app_state() {
    let _: axum::Router<()> = router();
}

#[tokio::test]
async fn project_routes_require_explicit_tenant_scope() {
    let app = router::<()>()
        .layer(axum::middleware::from_fn(
            crate::app::agent::gateway::middleware::tenant_scope_middleware,
        ))
        .with_state(());
    let old = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/platforms/platform-a/companies/company-a")
                .header(vw_api_types::id::PLATFORM_ID_HEADER, "platform-a")
                .header(vw_api_types::id::COMPANY_ID_HEADER, "company-a")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(old.status(), StatusCode::NOT_FOUND);

    let current = app
        .oneshot(
            Request::builder()
                .uri("/projects")
                .header(vw_api_types::id::PLATFORM_ID_HEADER, "platform-a")
                .header(vw_api_types::id::COMPANY_ID_HEADER, "company-a")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_ne!(current.status(), StatusCode::NOT_FOUND);
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

fn store_project(info: &project::Info) {
    project::store_local_test_project(info).expect("project should write");
}

fn remove_project(project_id: &str) {
    project::remove_local_test_project(project_id).expect("project should clean up");
}

fn default_tenant_scope() -> Extension<TenantScope> {
    Extension(project::default_tenant_scope())
}

fn local_project_path(project_id: impl Into<String>) -> Path<String> {
    Path(project_id.into())
}

fn local_worktree_path(
    project_id: impl Into<String>,
    worktree_id: impl Into<String>,
) -> Path<(String, String)> {
    Path((project_id.into(), worktree_id.into()))
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

    let dto = map_project(&info);

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

    let dto = map_project(&info);

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

    let dto = map_project(&info);

    assert_eq!(dto.name, "fallback-name");
}

#[test]
fn map_project_falls_back_to_directory_when_no_name_component_exists() {
    let info = project_info("project-empty-directory", String::new(), Vec::new());

    let dto = map_project(&info);

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

    let dto = map_project(&info);

    assert!(dto.git.is_repo);
    assert!(dto.git.has_uncommitted_changes);
    assert!(dto.current_branch.is_some());
}

#[test]
fn gateway_git_metadata_works_without_git_executable() {
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp = TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    write_file(&temp.path().join("README.md"), "clean\n");
    write_file(&temp.path().join("tracked.txt"), "one\ntwo\n");
    commit_all(&repo, "initial");
    write_file(&temp.path().join("tracked.txt"), "one\ntwo\nthree\n");
    write_file(&temp.path().join("new.txt"), "alpha\nbeta\n");
    drop(repo);

    let directory = temp.path().to_string_lossy().to_string();
    let mut info = project_info("project-git2-metadata", directory.clone(), vec![]);
    info.vcs = Some(project::Vcs::Git);

    let _path = PathGuard::without_git();

    let dto = map_project(&info);
    assert!(dto.git.is_repo);
    assert!(dto.git.has_uncommitted_changes);
    let branch = dto.current_branch.expect("current branch should resolve without git CLI");
    assert!(!branch.is_empty());
    assert_ne!(branch, "HEAD");

    let worktree = map_worktree("project-git2-metadata", &info, directory.clone());
    assert_eq!(worktree.branch, branch);

    let changed = file::status_git(temp.path());
    let tracked = changed
        .iter()
        .find(|info| info.path == "tracked.txt" && info.status == file::Status::Modified)
        .expect("modified tracked file should be reported without git CLI");
    assert_eq!(tracked.added, 1);
    assert_eq!(tracked.removed, 0);

    let added = changed
        .iter()
        .find(|info| info.path == "new.txt" && info.status == file::Status::Added)
        .expect("untracked file should be reported without git CLI");
    assert_eq!(added.added, 2);
    assert_eq!(added.removed, 0);
}

#[test]
fn map_worktree_uses_default_name_for_blank_directory() {
    let info = project_info("project-worktree-default", String::new(), vec![]);

    let dto = map_worktree("project-worktree-default", &info, String::new());

    assert_eq!(dto.name, "worktree");
    assert_eq!(dto.branch, "");
}

#[tokio::test]
async fn load_project_filters_missing_sandboxes_and_maps_missing_project() {
    let temp = TempDir::new().expect("tempdir");
    let id = format!("handler-project-{}", Uuid::new_v4());
    let existing = temp.path().join("sandbox");
    std::fs::create_dir_all(&existing).expect("sandbox dir");
    let info = project_info(
        &id,
        existing.to_string_lossy().to_string(),
        vec![existing.to_string_lossy().to_string(), "/definitely/missing".to_string()],
    );
    store_project(&info);

    let scope = project::project_scope_for_default_tenant(id.clone()).expect("scope");
    let loaded = load_project(&scope).await.expect("project should load");
    assert_eq!(loaded.sandboxes, vec![existing.to_string_lossy().to_string()]);

    remove_project(&id);
    let missing = load_project(&scope).await.expect_err("missing project should map to not found");
    assert_eq!(missing.status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn project_get_and_update_round_trip_storage() {
    let temp = TempDir::new().expect("tempdir");
    let id = format!("handler-project-update-{}", Uuid::new_v4());
    let info = project_info(&id, temp.path().to_string_lossy().to_string(), Vec::new());
    store_project(&info);

    let Json(response) = project_get_v1(default_tenant_scope(), local_project_path(id.clone()))
        .await
        .expect("project should get");
    assert_eq!(response.project.id.0, id);

    let Json(response) = project_update_v1(
        default_tenant_scope(),
        local_project_path(id.clone()),
        Json(UpdateProjectRequest {
            name: Some("Updated".to_string()),
            active_worktree_id: None,
            icon: Some(vw_api_types::project::IconUpdateDto {
                override_icon: Some("rocket".to_string()),
                color: Some("#123456".to_string()),
            }),
            commands: Some(vw_api_types::project::CommandsUpdateDto {
                start: Some("cargo run".to_string()),
            }),
        }),
    )
    .await
    .expect("project should update");

    assert_eq!(response.project.name, "Updated");
    remove_project(&id);
}

#[tokio::test]
async fn project_get_missing_project_returns_not_found() {
    let error = project_get_v1(
        default_tenant_scope(),
        local_project_path(format!("handler-project-get-missing-{}", Uuid::new_v4())),
    )
    .await
    .expect_err("missing project should fail");

    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.message, "project not found");
}

#[tokio::test]
async fn project_routes_do_not_cross_tenant_for_same_project_id() {
    let temp = TempDir::new().expect("tempdir");
    let first_dir = temp.path().join("first");
    let second_dir = temp.path().join("second");
    std::fs::create_dir_all(&first_dir).expect("first dir");
    std::fs::create_dir_all(&second_dir).expect("second dir");
    let project_id = format!("shared-project-{}", Uuid::new_v4());
    let first_tenant = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second_tenant = TenantScope::new("platform-b", "company-b").expect("second tenant");
    let first = project_info(&project_id, first_dir.to_string_lossy().into_owned(), Vec::new());
    let second = project_info(&project_id, second_dir.to_string_lossy().into_owned(), Vec::new());
    let store = project::runtime_store().expect("project store");
    store.create(&first_tenant, &first).expect("first project");
    store.create(&second_tenant, &second).expect("second project");

    let Json(first_response) =
        project_get_v1(Extension(first_tenant.clone()), Path(project_id.clone()))
            .await
            .expect("first response");
    let Json(second_response) =
        project_get_v1(Extension(second_tenant.clone()), Path(project_id.clone()))
            .await
            .expect("second response");

    assert_eq!(first_response.project.directory, first_dir.to_string_lossy());
    assert_eq!(second_response.project.directory, second_dir.to_string_lossy());
    store
        .delete(
            &ProjectScope::new(first_tenant, ProjectId::from(project_id.as_str())).expect("scope"),
        )
        .expect("delete first");
    store
        .delete(&ProjectScope::new(second_tenant, ProjectId::from(project_id)).expect("scope"))
        .expect("delete second");
}

#[tokio::test]
async fn project_update_missing_project_returns_bad_request() {
    let err = project_update_v1(
        default_tenant_scope(),
        local_project_path(format!("handler-project-missing-{}", Uuid::new_v4())),
        Json(UpdateProjectRequest::default()),
    )
    .await
    .expect_err("missing project should fail");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn project_list_filters_sorts_limits_and_cursors() {
    let temp = TempDir::new().expect("tempdir");
    let token = Uuid::new_v4().to_string();
    let alpha_dir = temp.path().join("alpha");
    let beta_dir = temp.path().join("beta");
    std::fs::create_dir_all(&alpha_dir).expect("alpha dir");
    std::fs::create_dir_all(&beta_dir).expect("beta dir");
    let alpha_id = format!("handler-list-alpha-{}", Uuid::new_v4());
    let beta_id = format!("handler-list-beta-{}", Uuid::new_v4());
    let mut alpha = project_info(&alpha_id, alpha_dir.to_string_lossy().to_string(), Vec::new());
    alpha.name = Some(format!("Alpha {token}"));
    alpha.time.updated = 100;
    let mut beta = project_info(&beta_id, beta_dir.to_string_lossy().to_string(), Vec::new());
    beta.name = Some(format!("Beta {token}"));
    beta.time.updated = 200;
    store_project(&alpha);
    store_project(&beta);

    let Json(page) = project_list_v1(
        default_tenant_scope(),
        Query(ListProjectsRequest {
            cursor: None,
            limit: Some(1),
            query: Some(token.clone()),
            status: Some(ProjectStatus::Ready),
        }),
    )
    .await
    .expect("projects should list");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id.0, beta_id);
    assert_eq!(page.next_cursor.as_deref(), Some(beta_id.as_str()));

    let Json(next_page) = project_list_v1(
        default_tenant_scope(),
        Query(ListProjectsRequest {
            cursor: page.next_cursor,
            limit: Some(50),
            query: Some(token),
            status: Some(ProjectStatus::Ready),
        }),
    )
    .await
    .expect("next projects should list");
    assert!(next_page.items.iter().any(|item| item.id.0 == alpha_id));

    remove_project(&alpha_id);
    remove_project(&beta_id);
}

#[tokio::test]
async fn project_list_handles_missing_cursor_blank_query_and_status_miss() {
    let temp = TempDir::new().expect("tempdir");
    let id = format!("handler-list-miss-{}", Uuid::new_v4());
    let info = project_info(&id, temp.path().to_string_lossy().to_string(), Vec::new());
    store_project(&info);

    let Json(page) = project_list_v1(
        default_tenant_scope(),
        Query(ListProjectsRequest {
            cursor: Some("missing-cursor".to_string()),
            limit: Some(0),
            query: Some("   ".to_string()),
            status: Some(ProjectStatus::Indexing),
        }),
    )
    .await
    .expect("projects should list");

    assert!(page.items.is_empty());
    assert_eq!(page.next_cursor, None);
    remove_project(&id);
}

#[tokio::test]
async fn project_list_uses_name_directory_and_id_tie_breakers() {
    let temp = TempDir::new().expect("tempdir");
    let token = Uuid::new_v4().to_string();
    let directory_a = temp.path().join("a");
    let directory_b = temp.path().join("b");
    std::fs::create_dir_all(&directory_a).expect("directory a");
    std::fs::create_dir_all(&directory_b).expect("directory b");

    let alpha_id = format!("tie-alpha-{token}");
    let same_a_first_id = format!("tie-a-first-{token}");
    let same_a_second_id = format!("tie-a-second-{token}");
    let same_b_id = format!("tie-b-{token}");
    let fixtures = [
        (&alpha_id, format!("Alpha {token}"), &directory_b),
        (&same_a_first_id, format!("Same {token}"), &directory_a),
        (&same_a_second_id, format!("Same {token}"), &directory_a),
        (&same_b_id, format!("Same {token}"), &directory_b),
    ];
    for (id, name, directory) in &fixtures {
        let mut info = project_info(id, directory.to_string_lossy().to_string(), Vec::new());
        info.name = Some(name.clone());
        info.time.updated = 100;
        store_project(&info);
    }

    let Json(page) = project_list_v1(
        default_tenant_scope(),
        Query(ListProjectsRequest {
            cursor: None,
            limit: Some(20),
            query: Some(token),
            status: None,
        }),
    )
    .await
    .expect("projects should list");

    let ids = page.items.into_iter().map(|item| item.id.0).collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            alpha_id.clone(),
            same_a_first_id.clone(),
            same_a_second_id.clone(),
            same_b_id.clone(),
        ]
    );

    for (id, _, _) in fixtures {
        remove_project(id);
    }
}

#[tokio::test]
async fn project_list_searches_directory_case_insensitively() {
    let temp = TempDir::new().expect("tempdir");
    let token = Uuid::new_v4().to_string();
    let directory = temp.path().join(format!("DirectoryNeedle-{token}"));
    std::fs::create_dir_all(&directory).expect("project dir");
    let id = format!("handler-directory-search-{token}");
    let mut info = project_info(&id, directory.to_string_lossy().to_string(), Vec::new());
    info.name = Some("Neutral Project".to_string());
    store_project(&info);

    let Json(page) = project_list_v1(
        default_tenant_scope(),
        Query(ListProjectsRequest {
            cursor: None,
            limit: None,
            query: Some(format!("DIRECTORYNEEDLE-{token}")),
            status: None,
        }),
    )
    .await
    .expect("projects should list");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id.0, id);
    remove_project(&id);
}

#[tokio::test]
async fn project_change_records_requires_directory() {
    let err = project_change_records_v1(
        default_tenant_scope(),
        local_project_path("missing"),
        Query(ListProjectChangeRecordsRequest { directory: None }),
    )
    .await
    .expect_err("directory is required");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.message, "directory is required");
}

#[tokio::test]
async fn project_change_records_rejects_blank_and_unowned_directory() {
    let blank = project_change_records_v1(
        default_tenant_scope(),
        local_project_path("missing"),
        Query(ListProjectChangeRecordsRequest { directory: Some(" \t ".to_string()) }),
    )
    .await
    .expect_err("blank directory should fail");
    assert_eq!(blank.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(blank.message, "directory is required");

    let temp = TempDir::new().expect("tempdir");
    let owned = temp.path().join("owned");
    let unrelated = temp.path().join("unrelated");
    std::fs::create_dir_all(&owned).expect("owned dir");
    std::fs::create_dir_all(&unrelated).expect("unrelated dir");
    let id = format!("handler-project-unowned-changes-{}", Uuid::new_v4());
    let info = project_info(&id, owned.to_string_lossy().to_string(), Vec::new());
    store_project(&info);

    let unowned = project_change_records_v1(
        default_tenant_scope(),
        local_project_path(id.clone()),
        Query(ListProjectChangeRecordsRequest {
            directory: Some(unrelated.to_string_lossy().to_string()),
        }),
    )
    .await
    .expect_err("unowned directory should fail");

    assert_eq!(unowned.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(unowned.message, "project directory not found");
    remove_project(&id);
}

#[tokio::test]
async fn project_resolve_and_change_records_handle_plain_directory() {
    let temp = TempDir::new().expect("tempdir");

    let Json(resolved) = project_resolve_v1(
        default_tenant_scope(),
        Json(ResolveProjectRequest {
            directory: temp.path().to_string_lossy().to_string(),
            create_if_missing: true,
        }),
    )
    .await
    .expect("plain directory should resolve");
    assert_eq!(resolved.project.directory, temp.path().to_string_lossy());

    let Json(changes) = project_change_records_v1(
        default_tenant_scope(),
        local_project_path(resolved.project.id.0.clone()),
        Query(ListProjectChangeRecordsRequest {
            directory: Some(temp.path().to_string_lossy().to_string()),
        }),
    )
    .await
    .expect("plain directory changes should list");
    assert!(changes.items.is_empty());

    remove_project(&resolved.project.id.0);
}

#[tokio::test]
async fn project_change_records_uses_stored_sandbox_match() {
    let temp = TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    write_file(&temp.path().join("README.md"), "clean\n");
    commit_all(&repo, "initial");
    write_file(&temp.path().join("README.md"), "dirty\n");
    let id = format!("handler-project-changes-{}", Uuid::new_v4());
    let mut info = project_info(
        &id,
        temp.path().join("primary").to_string_lossy().to_string(),
        vec![temp.path().to_string_lossy().to_string()],
    );
    info.vcs = Some(project::Vcs::Git);
    store_project(&info);

    let Json(response) = project_change_records_v1(
        default_tenant_scope(),
        local_project_path(id.clone()),
        Query(ListProjectChangeRecordsRequest {
            directory: Some(format!("{}/", temp.path().to_string_lossy())),
        }),
    )
    .await
    .expect("project changes should list");

    assert!(response.items.iter().any(|item| item.path == "README.md"));
    remove_project(&id);
}

#[tokio::test]
async fn project_worktrees_return_empty_for_non_git_project() {
    let temp = TempDir::new().expect("tempdir");
    let id = format!("handler-project-worktrees-{}", Uuid::new_v4());
    let info = project_info(&id, temp.path().to_string_lossy().to_string(), Vec::new());
    store_project(&info);

    let Json(response) =
        project_worktrees_v1(default_tenant_scope(), local_project_path(id.clone()))
            .await
            .expect("worktrees should list");

    assert!(response.items.is_empty());
    remove_project(&id);
}

#[tokio::test]
async fn project_worktrees_list_git_project_primary_worktree() {
    let temp = TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    write_file(&temp.path().join("README.md"), "demo\n");
    commit_all(&repo, "initial");
    let tenant = project::default_tenant_scope();
    let (info, _) = project::from_directory(&tenant, temp.path()).await.expect("project discover");

    let Json(response) =
        project_worktrees_v1(default_tenant_scope(), local_project_path(info.id.clone()))
            .await
            .expect("worktrees should list");
    let expected_directory =
        std::fs::canonicalize(temp.path()).expect("canonical temp").to_string_lossy().to_string();

    assert!(response.items.iter().any(|item| {
        let directory = std::fs::canonicalize(&item.directory)
            .expect("canonical worktree")
            .to_string_lossy()
            .to_string();
        directory == expected_directory && item.project_id.0 == info.id
    }));
    remove_project(&info.id);
}

#[tokio::test]
async fn project_store_rejects_project_without_directory() {
    let id = format!("handler-project-worktree-create-{}", Uuid::new_v4());
    let missing = std::env::temp_dir().join(format!("missing-project-{}", Uuid::new_v4()));
    let info = project_info(&id, missing.to_string_lossy().to_string(), Vec::new());
    store_project(&info);

    let err = project_worktree_create_v1(
        default_tenant_scope(),
        local_project_path(id.clone()),
        Json(CreateWorktreeRequest {
            name: "feature".to_string(),
            branch: "feature".to_string(),
            from_ref: None,
            checkout: false,
        }),
    )
    .await
    .expect_err("missing project directory should fail");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    remove_project(&id);
}

#[tokio::test]
async fn project_worktree_create_rejects_non_git_project() {
    let temp = TempDir::new().expect("tempdir");
    let id = format!("handler-project-worktree-non-git-{}", Uuid::new_v4());
    let info = project_info(&id, temp.path().to_string_lossy().to_string(), Vec::new());
    store_project(&info);

    let err = project_worktree_create_v1(
        default_tenant_scope(),
        local_project_path(id.clone()),
        Json(CreateWorktreeRequest {
            name: "feature".to_string(),
            branch: "feature".to_string(),
            from_ref: None,
            checkout: false,
        }),
    )
    .await
    .expect_err("non git project should fail");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    let tenant = project::default_tenant_scope();
    let (local, _) = project::from_directory(&tenant, temp.path()).await.expect("local project");
    remove_project(&id);
    remove_project(&local.id);
}

#[tokio::test]
async fn worktree_get_rejects_invalid_id_before_project_lookup() {
    let err = worktree_get_v1(default_tenant_scope(), local_worktree_path("missing", "invalid!"))
        .await
        .expect_err("invalid worktree id should fail");

    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn worktree_get_maps_stored_project_directory() {
    let temp = TempDir::new().expect("tempdir");
    let id = format!("handler-worktree-get-{}", Uuid::new_v4());
    let info = project_info(&id, temp.path().to_string_lossy().to_string(), Vec::new());
    store_project(&info);

    let Json(response) = worktree_get_v1(
        default_tenant_scope(),
        local_worktree_path(id.clone(), worktree_id_from_directory(&temp.path().to_string_lossy())),
    )
    .await
    .expect("worktree should get");

    assert_eq!(response.worktree.project_id.0, id);
    assert_eq!(
        normalize_path(&response.worktree.directory),
        normalize_path(&temp.path().to_string_lossy())
    );
    remove_project(&id);
}

#[tokio::test]
async fn worktree_handlers_reject_directory_outside_project() {
    let temp = TempDir::new().expect("tempdir");
    let owned = temp.path().join("owned");
    let unrelated = temp.path().join("unrelated");
    std::fs::create_dir_all(&owned).expect("owned dir");
    std::fs::create_dir_all(&unrelated).expect("unrelated dir");
    let id = format!("handler-worktree-unowned-{}", Uuid::new_v4());
    let info = project_info(&id, owned.to_string_lossy().to_string(), Vec::new());
    store_project(&info);
    let unrelated_id = worktree_id_from_directory(&unrelated.to_string_lossy());

    let get_error = worktree_get_v1(
        default_tenant_scope(),
        local_worktree_path(id.clone(), unrelated_id.clone()),
    )
    .await
    .expect_err("unowned worktree should not be returned");
    assert_eq!(get_error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(get_error.message, "worktree not found");

    let delete_error = worktree_delete_v1(
        default_tenant_scope(),
        local_worktree_path(id.clone(), unrelated_id.clone()),
        Json(DeleteWorktreeRequest { force: false }),
    )
    .await
    .expect_err("unowned worktree should not be deleted");
    assert_eq!(delete_error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(delete_error.message, "worktree not found");

    let reset_error = worktree_reset_v1(
        default_tenant_scope(),
        local_worktree_path(id.clone(), unrelated_id),
        Json(ResetWorktreeRequest {
            mode: ResetMode::Mixed,
            target_ref: Some("HEAD~1".to_string()),
        }),
    )
    .await
    .expect_err("unowned worktree should not be reset");
    assert_eq!(reset_error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(reset_error.message, "worktree not found");

    remove_project(&id);
}

#[tokio::test]
async fn worktree_delete_and_reset_reject_invalid_ids_before_project_lookup() {
    let delete_err = worktree_delete_v1(
        default_tenant_scope(),
        local_worktree_path("missing", "invalid!"),
        Json(DeleteWorktreeRequest { force: true }),
    )
    .await
    .expect_err("invalid delete id should fail");
    assert_eq!(delete_err.status, axum::http::StatusCode::NOT_FOUND);

    let reset_err = worktree_reset_v1(
        default_tenant_scope(),
        local_worktree_path("missing", "invalid!"),
        Json(ResetWorktreeRequest { mode: ResetMode::Hard, target_ref: Some("HEAD".to_string()) }),
    )
    .await
    .expect_err("invalid reset id should fail");
    assert_eq!(reset_err.status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn worktree_delete_and_reset_reject_non_git_project_after_lookup() {
    let temp = TempDir::new().expect("tempdir");
    let id = format!("handler-worktree-non-git-{}", Uuid::new_v4());
    let info = project_info(&id, temp.path().to_string_lossy().to_string(), Vec::new());
    store_project(&info);
    let worktree_id = worktree_id_from_directory(&temp.path().to_string_lossy());

    let delete_err = worktree_delete_v1(
        default_tenant_scope(),
        local_worktree_path(id.clone(), worktree_id.clone()),
        Json(DeleteWorktreeRequest { force: true }),
    )
    .await
    .expect_err("non git delete should fail");
    assert_eq!(delete_err.status, axum::http::StatusCode::BAD_REQUEST);

    let reset_err = worktree_reset_v1(
        default_tenant_scope(),
        local_worktree_path(id.clone(), worktree_id),
        Json(ResetWorktreeRequest { mode: ResetMode::Soft, target_ref: Some("HEAD".to_string()) }),
    )
    .await
    .expect_err("non git reset should fail");
    assert_eq!(reset_err.status, axum::http::StatusCode::BAD_REQUEST);

    let tenant = project::default_tenant_scope();
    let (local, _) = project::from_directory(&tenant, temp.path()).await.expect("local project");
    remove_project(&id);
    remove_project(&local.id);
}
