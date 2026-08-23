//! 验证项目图标发现的格式编码、候选优先级与文件系统边界。

use super::*;
use crate::app::agent::project::{Icon, TimeInfo};
use base64::Engine as _;
use tempfile::TempDir;
use uuid::Uuid;

fn info(id: &str, worktree: &str) -> Info {
    Info {
        id: id.to_string(),
        worktree: worktree.to_string(),
        vcs: Some(Vcs::Git),
        name: None,
        icon: None,
        commands: None,
        time: TimeInfo { created: 1, updated: 1, initialized: None },
        sandboxes: Vec::new(),
    }
}

fn scope(id: &str) -> ProjectScope {
    crate::app::agent::project::project_scope_for_default_tenant(id)
        .expect("project scope should build")
}

fn store(info: &Info) {
    crate::app::agent::project::store_local_test_project(info).expect("project should write");
}

fn read(id: &str) -> Info {
    crate::app::agent::project::read_local_test_project(id)
        .expect("project should read")
        .expect("project should exist")
}

fn remove(id: &str) {
    crate::app::agent::project::remove_local_test_project(id).expect("cleanup should succeed");
}

// 单图标夹具负责注册项目并回读持久化结果，统一验证完整发现链路。
async fn discover_single_icon(file_name: &str, bytes: &[u8]) -> String {
    let temp = TempDir::new().expect("tempdir should create");
    std::fs::write(temp.path().join(file_name), bytes).expect("icon should write");
    let id = format!("icon-test-{}", Uuid::new_v4());
    let input = info(&id, &temp.path().to_string_lossy());
    store(&input);

    discover(&scope(&id), &input).await.expect("icon discovery should succeed");

    let url = read(&id).icon.and_then(|icon| icon.url).expect("icon url should be set");
    remove(&id);
    url
}

#[tokio::test]
async fn discover_writes_shortest_favicon_as_data_url() {
    let temp = TempDir::new().expect("tempdir should create");
    let id = format!("icon-test-{}", Uuid::new_v4());
    let nested = temp.path().join("deep").join("assets");
    std::fs::create_dir_all(&nested).expect("nested dir should create");
    std::fs::write(nested.join("favicon.png"), b"nested").expect("nested icon should write");
    std::fs::write(temp.path().join("favicon.svg"), b"<svg/>").expect("root icon should write");

    let input = info(&id, &temp.path().to_string_lossy());
    store(&input);

    discover(&scope(&id), &input).await.expect("icon discovery should succeed");

    let stored = read(&id);
    let url = stored.icon.and_then(|i| i.url).expect("icon url should be set");
    assert_eq!(url, "data:image/svg+xml;base64,PHN2Zy8+");

    remove(&id);
}

#[tokio::test]
async fn discover_encodes_supported_favicon_formats_with_matching_mime_types() {
    let cases: [(&str, &[u8], &str); 6] = [
        ("favicon.ico", b"ico", "image/x-icon"),
        ("favicon.svg", b"svg", "image/svg+xml"),
        ("favicon.jpg", b"jpg", "image/jpeg"),
        ("FAVICON.JPEG", b"jpeg", "image/jpeg"),
        ("favicon.webp", b"webp", "image/webp"),
        ("favicon.png", b"png", "image/png"),
    ];

    for (file_name, bytes, mime) in cases {
        let url = discover_single_icon(file_name, bytes).await;
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        assert_eq!(url, format!("data:{mime};base64,{encoded}"));
    }
}

#[tokio::test]
async fn discover_skips_non_git_project_even_when_favicon_exists() {
    let temp = TempDir::new().expect("tempdir should create");
    std::fs::write(temp.path().join("favicon.png"), b"png").expect("icon should write");
    let id = format!("non-git-icon-test-{}", Uuid::new_v4());
    let mut input = info(&id, &temp.path().to_string_lossy());
    input.vcs = None;

    discover(&scope(&id), &input).await.expect("non git should be ignored");
}

#[tokio::test]
async fn discover_skips_project_with_existing_icon_url() {
    let temp = TempDir::new().expect("tempdir should create");
    std::fs::write(temp.path().join("favicon.png"), b"png").expect("icon should write");
    let id = format!("existing-url-icon-test-{}", Uuid::new_v4());
    let mut input = info(&id, &temp.path().to_string_lossy());
    input.icon =
        Some(Icon { url: Some("https://example.test/icon.png".to_string()), ..Default::default() });

    discover(&scope(&id), &input).await.expect("existing url should be ignored");
}

#[tokio::test]
async fn discover_skips_project_with_override_icon() {
    let temp = TempDir::new().expect("tempdir should create");
    std::fs::write(temp.path().join("favicon.png"), b"png").expect("icon should write");
    let id = format!("override-icon-test-{}", Uuid::new_v4());
    let mut input = info(&id, &temp.path().to_string_lossy());
    input.icon = Some(Icon { override_icon: Some("rocket".to_string()), ..Default::default() });

    discover(&scope(&id), &input).await.expect("override icon should be ignored");
}

#[tokio::test]
async fn discover_skips_missing_worktree_directory() {
    let temp = TempDir::new().expect("tempdir should create");
    let id = format!("missing-worktree-icon-test-{}", Uuid::new_v4());
    let input = info(&id, &temp.path().join("missing").to_string_lossy());

    discover(&scope(&id), &input).await.expect("missing dir should be ignored");
}

#[tokio::test]
async fn discover_leaves_icon_unset_without_file_candidates() {
    let temp = TempDir::new().expect("tempdir should create");
    std::fs::write(temp.path().join("logo.svg"), b"svg").expect("non-favicon should write");
    std::fs::create_dir(temp.path().join("favicon.png")).expect("favicon directory should create");
    let id = format!("no-favicon-icon-test-{}", Uuid::new_v4());
    let input = info(&id, &temp.path().to_string_lossy());
    store(&input);

    discover(&scope(&id), &input).await.expect("missing favicon should be ignored");

    assert!(read(&id).icon.is_none());
    remove(&id);
}

#[tokio::test]
async fn discover_propagates_missing_project_update_error() {
    let temp = TempDir::new().expect("tempdir should create");
    std::fs::write(temp.path().join("favicon.png"), b"png").expect("icon should write");
    let id = format!("missing-project-icon-test-{}", Uuid::new_v4());
    let input = info(&id, &temp.path().to_string_lossy());

    let error =
        discover(&scope(&id), &input).await.expect_err("missing project should fail update");

    assert!(error.to_string().contains("project not found"));
}

#[cfg(unix)]
#[tokio::test]
async fn discover_propagates_favicon_read_permission_error() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().expect("tempdir should create");
    let icon = temp.path().join("favicon.png");
    std::fs::write(&icon, b"png").expect("icon should write");
    std::fs::set_permissions(&icon, std::fs::Permissions::from_mode(0o000))
        .expect("icon permissions should update");
    let id = format!("unreadable-icon-test-{}", Uuid::new_v4());
    let input = info(&id, &temp.path().to_string_lossy());

    let error = discover(&scope(&id), &input).await.expect_err("unreadable icon should fail");

    assert!(matches!(
        error,
        Error::Io(error) if error.kind() == std::io::ErrorKind::PermissionDenied
    ));
}

#[cfg(unix)]
#[tokio::test]
// 发现过程不跟随目录符号链接，避免扫描逃出项目根目录。
async fn discover_does_not_follow_symlinked_directories() {
    let root = TempDir::new().expect("root tempdir should create");
    let external = TempDir::new().expect("external tempdir should create");
    std::fs::write(external.path().join("favicon.png"), b"external")
        .expect("external icon should write");
    std::os::unix::fs::symlink(external.path(), root.path().join("linked-assets"))
        .expect("directory symlink should create");
    let id = format!("symlink-icon-test-{}", Uuid::new_v4());
    let input = info(&id, &root.path().to_string_lossy());
    store(&input);

    discover(&scope(&id), &input).await.expect("symlink should be ignored");

    assert!(read(&id).icon.is_none());
    remove(&id);
}
