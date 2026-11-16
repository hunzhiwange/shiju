//! 会话作用域的目录校验，不依赖宿主运行时。

use super::{LifecycleError, SessionScope};
use tempfile::TempDir;
use vw_api_transport::id::TenantScope;

#[test]
fn create_session_requires_canonical_absolute_directory() {
    let tenant = TenantScope::new("runtime-platform-directory", "runtime-company-directory")
        .expect("test tenant should be valid");
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
