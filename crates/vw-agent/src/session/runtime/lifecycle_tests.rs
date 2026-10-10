//! 验证会话创建和加载时的目录规范化与租户范围隔离。
//! 目录必须是规范化绝对路径，加载时还要同时匹配租户、项目和原生会话身份。

use std::path::Path;

use tempfile::TempDir;
use tokio::sync::Mutex;
use vw_api_types::id::TenantScope;

use super::{
    CreateSessionInput, LifecycleError, SessionScope, create_session, create_visible_task_session,
    load_session,
};

static LIFECYCLE_TEST_LOCK: Mutex<()> = Mutex::const_new(());

fn tenant(platform_id: &str, company_id: &str) -> TenantScope {
    TenantScope::new(platform_id, company_id).expect("test tenant should be valid")
}

fn scope(tenant: TenantScope, directory: &Path) -> SessionScope {
    SessionScope::new(tenant, directory).expect("test scope should be valid")
}

#[test]
fn create_session_requires_canonical_absolute_directory() {
    let tenant = tenant("runtime-platform-directory", "runtime-company-directory");
    let relative = SessionScope::new(tenant.clone(), "relative/project");
    assert!(matches!(relative, Err(LifecycleError::DirectoryMustBeAbsolute { .. })));

    let missing =
        std::env::temp_dir().join(format!("vw-agent-runtime-missing-{}", uuid::Uuid::new_v4()));
    let missing = SessionScope::new(tenant.clone(), &missing);
    assert!(matches!(missing, Err(LifecycleError::DirectoryUnavailable { .. })));

    let project = TempDir::new().expect("project directory");
    let non_canonical = project.path().join(".");
    let scope =
        SessionScope::new(tenant, &non_canonical).expect("existing absolute directory should work");
    assert_eq!(
        scope.directory(),
        project.path().canonicalize().expect("project directory should canonicalize")
    );
}

#[tokio::test]
// 加载同时校验目录和租户作用域，任一不匹配都不能复用持久化会话。
async fn load_session_rejects_directory_or_scope_mismatch() {
    let _guard = LIFECYCLE_TEST_LOCK.lock().await;
    crate::project::instance::dispose_all().await;

    let project_a = TempDir::new().expect("project a directory");
    let project_b = TempDir::new().expect("project b directory");
    let tenant_a = tenant("runtime-platform-owner-a", "runtime-company-owner-a");
    let tenant_b = tenant("runtime-platform-owner-b", "runtime-company-owner-b");
    let created = create_session(CreateSessionInput {
        scope: scope(tenant_a.clone(), project_a.path()),
        parent_id: None,
        title: Some("Owned session".to_string()),
        permission: None,
    })
    .await
    .expect("session should be created");

    let wrong_directory = load_session(scope(tenant_a, project_b.path()), &created.info.id).await;
    assert!(matches!(wrong_directory, Err(LifecycleError::SessionOwnershipMismatch { .. })));

    let wrong_tenant = load_session(scope(tenant_b, project_a.path()), &created.info.id).await;
    assert!(matches!(wrong_tenant, Err(LifecycleError::SessionNotFound { .. })));

    crate::project::instance::dispose_all().await;
}

#[tokio::test]
async fn created_session_preserves_native_identity() {
    let _guard = LIFECYCLE_TEST_LOCK.lock().await;
    crate::project::instance::dispose_all().await;

    let project = TempDir::new().expect("project directory");
    let scope =
        scope(tenant("runtime-platform-identity", "runtime-company-identity"), project.path());
    let created = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Native identity".to_string()),
        permission: None,
    })
    .await
    .expect("session should be created");
    let loaded = load_session(scope, &created.info.id).await.expect("created session should load");

    assert_eq!(loaded.info.id, created.info.id);
    assert_eq!(loaded.info.slug, created.info.slug);
    assert_eq!(loaded.info.project_id, created.info.project_id);
    assert_eq!(loaded.info.directory, created.info.directory);
    assert!(loaded.history.is_empty());

    crate::project::instance::dispose_all().await;
}

#[tokio::test]
async fn session_tool_create_marks_visible_parent_relation() {
    let _guard = LIFECYCLE_TEST_LOCK.lock().await;
    crate::project::instance::dispose_all().await;

    let project = TempDir::new().expect("project directory");
    let scope =
        scope(tenant("runtime-platform-visible", "runtime-company-visible"), project.path());
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".to_string()),
        permission: None,
    })
    .await
    .expect("parent should be created");
    let created = create_visible_task_session(CreateSessionInput {
        scope,
        parent_id: Some(parent.info.id.clone()),
        title: Some("Visible task".to_string()),
        permission: None,
    })
    .await
    .expect("visible task should be created");

    assert_eq!(created.info.parent_id.as_deref(), Some(parent.info.id.as_str()));
    assert!(created.info.is_visible_task());
    crate::project::instance::dispose_all().await;
}
