use session_info::Info;
use crate::session_host::SessionHost;
use axum::Router;
use std::path::PathBuf;
use session_port::{CreateSessionInput, SessionScope};
use session_store::ui_store;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tempfile::TempDir;
use tower::ServiceExt;
use vw_api_transport::id::TenantScope;

fn tenant() -> TenantScope {
    let id = format!("archived-delete-{}", uuid::Uuid::new_v4());
    TenantScope::new(id.clone(), id).unwrap()
}

pub async fn archived_chat_delete_strict_enumeration_rejects_a_file_prefix<H: SessionHost>(_app: fn(TenantScope) -> Router, _database: PathBuf) {
    let id = uuid::Uuid::new_v4().to_string();
    agent_storage::write(&["archived-delete-fixture", &id], &serde_json::json!({}))
        .await
        .unwrap();
    let filename = format!("{id}.json");
    let result = agent_storage::list_strict(&["archived-delete-fixture", &filename]).await;
    agent_storage::remove(&["archived-delete-fixture", &id]).await.unwrap();
    assert!(result.is_err(), "failed enumeration must not masquerade as an empty subtree");
}

async fn create<H: SessionHost>(scope: &SessionScope, parent: Option<&str>) -> Info {
    H::create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: parent.map(str::to_owned),
        title: Some("Fixture".into()),
        permission: None,
    })
    .await
    .unwrap()
    .info
}

async fn archive<H: SessionHost>(tenant: &TenantScope, id: &str, archived: bool) {
    ui_store::with_tenant_scope(
        tenant.clone(),
        H::update_any(id, |info| {
            info.time.archived = archived.then_some(1);
        }),
    )
    .await
    .unwrap();
}

async fn read(tenant: &TenantScope, id: &str) -> Option<Info> {
    let tenant = tenant.clone();
    let id = id.to_owned();
    agent_database::run_isolated(move || {
        ui_store::load_agent_session_any_for_tenant_fallible(&tenant, &id)
    })
    .await
    .unwrap()
    .unwrap()
}

async fn save_ui(tenant: &TenantScope, info: &Info) {
    let tenant = tenant.clone();
    let project_id = info.project_id.clone();
    let snapshot = session_ui_types::ChatSession {
        id: info.id.clone(),
        title: info.title.clone(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: info.time.created,
        updated_ms: info.time.updated,
    };
    agent_database::run_isolated(move || {
        ui_store::save_session_ui_scoped_for_tenant(
            &tenant,
            &snapshot,
            Some(&project_id),
            false,
        )
    })
    .await
    .unwrap()
    .unwrap();
}

async fn has_ui(tenant: &TenantScope, info: &Info) -> bool {
    let tenant = tenant.clone();
    let id = info.id.clone();
    let project_id = info.project_id.clone();
    agent_database::run_isolated(move || {
        ui_store::load_session_scoped_for_tenant_fallible(&tenant, &id, Some(&project_id))
    })
    .await
    .unwrap()
    .unwrap()
    .is_some()
}

async fn delete(
    app: fn(TenantScope) -> Router,
    tenant: &TenantScope,
    id: &str,
    directory: &std::path::Path,
) -> (StatusCode, Value) {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("directory", &directory.to_string_lossy())
        .finish();
    let response = app(tenant.clone())
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/session/{id}/archived?{query}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

pub async fn archived_chat_delete_http_removes_multilevel_tree_and_preserves_other_scopes<H: SessionHost>(app: fn(TenantScope) -> Router, _database: PathBuf) {
    let tenant = tenant();
    let directory = TempDir::new().unwrap();
    let scope = SessionScope::new(tenant.clone(), directory.path()).unwrap();
    let root = create::<H>(&scope, None).await;
    let child = create::<H>(&scope, Some(&root.id)).await;
    let grandchild = create::<H>(&scope, Some(&child.id)).await;
    save_ui(&tenant, &root).await;
    save_ui(&tenant, &child).await;
    assert!(has_ui(&tenant, &root).await);
    let pending = H::reserve_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: Some(grandchild.id.clone()),
        title: Some("Pending fixture".into()),
        permission: None,
    })
    .await
    .unwrap()
    .info;
    let unrelated = create::<H>(&scope, None).await;
    let foreign_tenant = self::tenant();
    let foreign_scope = SessionScope::new(foreign_tenant.clone(), directory.path()).unwrap();
    let foreign = create::<H>(&foreign_scope, None).await;
    archive::<H>(&tenant, &root.id, true).await;
    let other_directory = TempDir::new().unwrap();
    let other_scope = SessionScope::new(tenant.clone(), other_directory.path()).unwrap();
    let other = create::<H>(&other_scope, None).await;
    assert_eq!(
        delete(app, &tenant, &root.id, other_directory.path()).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        delete(app, &self::tenant(), &root.id, directory.path()).await.0,
        StatusCode::NOT_FOUND
    );
    let (status, body) = delete(app, &tenant, &root.id, directory.path()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["deleted_ids"],
        serde_json::json!([pending.id, grandchild.id, child.id, root.id])
    );
    for id in [&root.id, &child.id, &grandchild.id, &pending.id] {
        assert!(read(&tenant, id).await.is_none());
    }
    assert!(!has_ui(&tenant, &root).await);
    assert!(!has_ui(&tenant, &child).await);
    assert!(read(&tenant, &unrelated.id).await.is_some());
    assert!(read(&tenant, &other.id).await.is_some());
    assert!(read(&foreign_tenant, &foreign.id).await.is_some());
    assert!(
        H::create_session(CreateSessionInput {
            scope,
            parent_id: Some(root.id),
            title: None,
            permission: None,
        })
        .await
        .is_err(),
        "deleted parents cannot acquire new descendants"
    );
}

pub async fn archived_chat_delete_http_rejects_live_root_and_restore_while_waiting_for_history<H: SessionHost>(app: fn(TenantScope) -> Router, _database: PathBuf) {
    let tenant = tenant();
    let directory = TempDir::new().unwrap();
    let scope = SessionScope::new(tenant.clone(), directory.path()).unwrap();
    let root = create::<H>(&scope, None).await;
    assert_eq!(delete(app, &tenant, &root.id, directory.path()).await.0, StatusCode::BAD_REQUEST);
    archive::<H>(&tenant, &root.id, true).await;
    let history = session_engine::message::lock_history(&root.id).await;
    let mut pending = tokio::spawn({
        let tenant = tenant.clone();
        let id = root.id.clone();
        let path = directory.path().to_owned();
        async move { delete(app, &tenant, &id, &path).await }
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut pending)
            .await
            .is_err(),
        "the request must wait for the held history guard"
    );
    archive::<H>(&tenant, &root.id, false).await;
    drop(history);
    assert_eq!(pending.await.unwrap().0, StatusCode::BAD_REQUEST);
    assert!(read(&tenant, &root.id).await.unwrap().time.archived.is_none());
}

pub async fn archived_chat_delete_storage_failure_keeps_root_and_allows_retry<H: SessionHost>(app: fn(TenantScope) -> Router, _database: PathBuf) {
    let tenant = tenant();
    let directory = TempDir::new().unwrap();
    let scope = SessionScope::new(tenant.clone(), directory.path()).unwrap();
    let root = create::<H>(&scope, None).await;
    let child = create::<H>(&scope, Some(&root.id)).await;
    let grandchild = create::<H>(&scope, Some(&child.id)).await;
    save_ui(&tenant, &child).await;
    save_ui(&tenant, &grandchild).await;
    archive::<H>(&tenant, &root.id, true).await;
    let database = _database;
    let connection = rusqlite::Connection::open(database).unwrap();
    let trigger = format!("archived_delete_{}", uuid::Uuid::new_v4().simple());
    connection.execute_batch(&format!(
        "CREATE TRIGGER {trigger} BEFORE DELETE ON session_agent_sessions
         WHEN OLD.session_id = '{}' BEGIN SELECT RAISE(FAIL, 'injected deletion failure'); END;",
        child.id,
    )).unwrap();
    let (status, _) = delete(app, &tenant, &root.id, directory.path()).await;
    connection.execute_batch(&format!("DROP TRIGGER {trigger}")).unwrap();
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(read(&tenant, &root.id).await.is_some());
    assert!(read(&tenant, &child.id).await.is_some());
    assert!(has_ui(&tenant, &child).await, "single-node transaction rolls back UI deletion");
    assert!(!has_ui(&tenant, &grandchild).await);
    assert!(
        read(&tenant, &grandchild.id).await.is_none(),
        "successful descendants stay deleted"
    );
    let (status, body) = delete(app, &tenant, &root.id, directory.path()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["deleted_ids"], serde_json::json!([child.id, root.id]));
    assert!(read(&tenant, &root.id).await.is_none());
}
