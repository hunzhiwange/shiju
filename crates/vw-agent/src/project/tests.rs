use super::*;
use serde_json::json;

#[test]
fn extra_builds_owned_json_map() {
    let map = extra([("name", json!("demo")), ("ok", json!(true))]);
    assert_eq!(map.get("name"), Some(&json!("demo")));
    assert_eq!(map.get("ok"), Some(&json!(true)));
}

#[test]
fn extra_handles_empty_input_and_duplicate_keys() {
    assert!(extra::<0>([]).is_empty());

    let map = extra([("name", json!("first")), ("name", json!("second"))]);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("name"), Some(&json!("second")));
}

#[test]
fn project_error_display_delegates_to_inner_errors() {
    let io = Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "io boom"));
    assert_eq!(io.to_string(), "io boom");

    let json_err = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    assert!(Error::Json(json_err).to_string().contains("EOF"));
}

#[test]
fn project_error_conversions_preserve_variants_and_messages() {
    let storage: Error =
        storage::Error::NotFound(storage::NotFoundError { message: "missing project".to_string() })
            .into();
    assert!(matches!(&storage, Error::Storage(_)));
    assert_eq!(storage.to_string(), "missing project");

    let io_inner = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let io: Error = io_inner.into();
    assert!(matches!(&io, Error::Io(_)));
    assert_eq!(io.to_string(), "denied");

    let json_inner = serde_json::from_str::<serde_json::Value>("[").unwrap_err();
    let json_message = json_inner.to_string();
    let json: Error = json_inner.into();
    assert!(matches!(&json, Error::Json(_)));
    assert_eq!(json.to_string(), json_message);

    let database_inner = anyhow::anyhow!("database unavailable");
    let database: Error = database_inner.into();
    assert!(matches!(&database, Error::Database(_)));
    assert_eq!(database.to_string(), "database unavailable");
}

#[tokio::test]
async fn project_error_display_delegates_to_join_error() {
    let join_error = tokio::spawn(async { panic!("join failure") })
        .await
        .expect_err("panicking task should produce a join error");
    let expected = join_error.to_string();

    assert_eq!(Error::Join(join_error).to_string(), expected);
}

#[test]
fn default_tenant_scope_uses_stable_ids() {
    let scope = default_tenant_scope();

    assert_eq!(scope.platform_id().as_ref(), "10000");
    assert_eq!(scope.company_id().as_ref(), "100000000");
}

#[test]
fn project_scope_for_default_tenant_preserves_project_and_tenant_ids() {
    let scope = project_scope_for_default_tenant("project-109")
        .expect("non-empty project id should create a scope");

    assert_eq!(scope.project_id().as_ref(), "project-109");
    assert_eq!(scope.tenant_scope(), &default_tenant_scope());
}

#[test]
fn project_scope_for_default_tenant_rejects_blank_project_id() {
    let error =
        project_scope_for_default_tenant(" \t ").expect_err("blank project id should be rejected");

    assert!(matches!(&error, Error::Scope(_)));
    assert_eq!(error.to_string(), "project id must not be empty");
}

#[tokio::test]
async fn instance_bootstrap_accepts_existing_directory() {
    let dir = tempfile::TempDir::new().expect("tempdir should create");
    instance_bootstrap(dir.path()).await;
}

#[tokio::test]
async fn instance_bootstrap_records_worktree_once() {
    let dir = tempfile::TempDir::new().expect("tempdir should create");
    let key = dir.path().to_string_lossy().to_string();
    {
        let mut bootstrapped = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|e| e.into_inner());
        bootstrapped.remove(&key);
    }

    instance_bootstrap(dir.path()).await;
    instance_bootstrap(dir.path()).await;

    let bootstrapped = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|e| e.into_inner());
    assert!(bootstrapped.contains(&key));
    assert_eq!(bootstrapped.iter().filter(|entry| *entry == &key).count(), 1);
}

#[tokio::test]
async fn instance_bootstrap_recovers_from_poisoned_worktree_registry() {
    let poisoned = std::panic::catch_unwind(|| {
        let _guard = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|error| error.into_inner());
        panic!("poison worktree registry for recovery coverage");
    });
    assert!(poisoned.is_err());

    let dir = tempfile::TempDir::new().expect("tempdir should create");
    let key = dir.path().to_string_lossy().to_string();
    instance_bootstrap(dir.path()).await;

    let bootstrapped = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|error| error.into_inner());
    assert!(bootstrapped.contains(&key));
}

#[test]
fn project_event_type_is_stable_and_extra_keeps_values() {
    assert_eq!(event::UPDATED.r#type, "project.updated");
    let map = extra([("count", json!(2)), ("name", json!("demo"))]);
    assert_eq!(map.get("count"), Some(&json!(2)));
    assert_eq!(map.get("name"), Some(&json!("demo")));
}
