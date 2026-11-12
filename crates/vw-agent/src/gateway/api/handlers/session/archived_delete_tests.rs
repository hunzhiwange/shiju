use super::archived_delete::delete_order;
use crate::session::session::{Info, TimeInfo};

fn info(id: &str, parent: Option<&str>, archived: bool) -> Info {
    Info {
        id: id.into(),
        slug: "fixture".into(),
        project_id: "project".into(),
        directory: "/tmp/fixture".into(),
        parent_id: parent.map(str::to_owned),
        summary: None,
        share: None,
        title: "Fixture".into(),
        version: "test".into(),
        time: TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: archived.then_some(1),
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn archived_chat_delete_orders_descendants_before_root_and_preserves_unrelated() {
    let root = info("root", None, true);
    let sessions = vec![
        root.clone(),
        info("child", Some("root"), false),
        info("grandchild", Some("child"), false),
        info("unrelated", None, true),
    ];
    assert_eq!(delete_order(&root, &sessions).unwrap(), ["grandchild", "child", "root"]);
}

#[test]
fn archived_chat_delete_rejects_unarchived_root() {
    let root = info("root", None, false);
    assert!(delete_order(&root, std::slice::from_ref(&root)).is_err());
}

#[test]
fn archived_chat_delete_rejects_cross_project_descendant() {
    let root = info("root", None, true);
    let mut child = info("child", Some("root"), false);
    child.project_id = "other".into();
    assert!(delete_order(&root, &[root.clone(), child]).is_err());
}

#[test]
fn archived_chat_delete_rejects_cycle() {
    let root = info("root", Some("child"), true);
    let child = info("child", Some("root"), false);
    assert!(delete_order(&root, &[root.clone(), child]).is_err());
}

#[test]
fn archived_chat_delete_rejects_cross_directory_and_conflicting_ids() {
    let root = info("root", None, true);
    let mut child = info("child", Some("root"), false);
    child.directory = "/tmp/other".into();
    assert!(delete_order(&root, &[root.clone(), child]).is_err());
    let mut duplicate = root.clone();
    duplicate.project_id = "other".into();
    assert!(delete_order(&root, &[root.clone(), duplicate]).is_err());
}

#[test]
fn archived_chat_delete_rejects_hidden_root_and_deduplicates_identical_records() {
    let mut root = info("root", None, true);
    assert_eq!(delete_order(&root, &[root.clone(), root.clone()]).unwrap(), ["root"]);
    root.mark_agent_tool();
    assert!(delete_order(&root, std::slice::from_ref(&root)).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
mod persistence {
    use super::*;
    use crate::session::runtime::{CreateSessionInput, SessionScope, create_session};
    use crate::session::{session, ui_store};
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tempfile::TempDir;
    use tower::ServiceExt;
    use vw_api_types::id::TenantScope;

    fn tenant() -> TenantScope {
        let id = format!("archived-delete-{}", uuid::Uuid::new_v4());
        TenantScope::new(id.clone(), id).unwrap()
    }

    #[tokio::test]
    async fn archived_chat_delete_strict_enumeration_rejects_a_file_prefix() {
        let id = uuid::Uuid::new_v4().to_string();
        crate::storage::write(&["archived-delete-fixture", &id], &serde_json::json!({}))
            .await
            .unwrap();
        let filename = format!("{id}.json");
        let result = crate::storage::list_strict(&["archived-delete-fixture", &filename]).await;
        crate::storage::remove(&["archived-delete-fixture", &id]).await.unwrap();
        assert!(result.is_err(), "failed enumeration must not masquerade as an empty subtree");
    }

    async fn create(scope: &SessionScope, parent: Option<&str>) -> Info {
        create_session(CreateSessionInput {
            scope: scope.clone(),
            parent_id: parent.map(str::to_owned),
            title: Some("Fixture".into()),
            permission: None,
        })
        .await
        .unwrap()
        .info
    }

    async fn archive(tenant: &TenantScope, id: &str, archived: bool) {
        ui_store::with_tenant_scope(
            tenant.clone(),
            session::update_any(id, |info| {
                info.time.archived = archived.then_some(1);
            }),
        )
        .await
        .unwrap();
    }

    async fn read(tenant: &TenantScope, id: &str) -> Option<Info> {
        let tenant = tenant.clone();
        let id = id.to_owned();
        crate::global_database::run_isolated(move || {
            ui_store::load_agent_session_any_for_tenant_fallible(&tenant, &id)
        })
        .await
        .unwrap()
        .unwrap()
    }

    async fn save_ui(tenant: &TenantScope, info: &Info) {
        let tenant = tenant.clone();
        let project_id = info.project_id.clone();
        let snapshot = crate::session::ui_types::ChatSession {
            id: info.id.clone(),
            title: info.title.clone(),
            messages: Vec::new(),
            message_ids: Vec::new(),
            calls: Vec::new(),
            steps: Vec::new(),
            created_ms: info.time.created,
            updated_ms: info.time.updated,
        };
        crate::global_database::run_isolated(move || {
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
        crate::global_database::run_isolated(move || {
            ui_store::load_session_scoped_for_tenant_fallible(&tenant, &id, Some(&project_id))
        })
        .await
        .unwrap()
        .unwrap()
        .is_some()
    }

    async fn delete(
        tenant: &TenantScope,
        id: &str,
        directory: &std::path::Path,
    ) -> (StatusCode, Value) {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("directory", &directory.to_string_lossy())
            .finish();
        let response = super::super::tests::test_app_for_tenant(tenant.clone())
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

    #[tokio::test]
    async fn archived_chat_delete_http_removes_multilevel_tree_and_preserves_other_scopes() {
        let tenant = tenant();
        let directory = TempDir::new().unwrap();
        let scope = SessionScope::new(tenant.clone(), directory.path()).unwrap();
        let root = create(&scope, None).await;
        let child = create(&scope, Some(&root.id)).await;
        let grandchild = create(&scope, Some(&child.id)).await;
        save_ui(&tenant, &root).await;
        save_ui(&tenant, &child).await;
        assert!(has_ui(&tenant, &root).await);
        let pending = crate::session::runtime::reserve_session(CreateSessionInput {
            scope: scope.clone(),
            parent_id: Some(grandchild.id.clone()),
            title: Some("Pending fixture".into()),
            permission: None,
        })
        .await
        .unwrap()
        .info;
        let unrelated = create(&scope, None).await;
        let foreign_tenant = self::tenant();
        let foreign_scope = SessionScope::new(foreign_tenant.clone(), directory.path()).unwrap();
        let foreign = create(&foreign_scope, None).await;
        archive(&tenant, &root.id, true).await;
        let other_directory = TempDir::new().unwrap();
        let other_scope = SessionScope::new(tenant.clone(), other_directory.path()).unwrap();
        let other = create(&other_scope, None).await;
        assert_eq!(
            delete(&tenant, &root.id, other_directory.path()).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            delete(&self::tenant(), &root.id, directory.path()).await.0,
            StatusCode::NOT_FOUND
        );
        let (status, body) = delete(&tenant, &root.id, directory.path()).await;
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
            create_session(CreateSessionInput {
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

    #[tokio::test]
    async fn archived_chat_delete_http_rejects_live_root_and_restore_while_waiting_for_history() {
        let tenant = tenant();
        let directory = TempDir::new().unwrap();
        let scope = SessionScope::new(tenant.clone(), directory.path()).unwrap();
        let root = create(&scope, None).await;
        assert_eq!(delete(&tenant, &root.id, directory.path()).await.0, StatusCode::BAD_REQUEST);
        archive(&tenant, &root.id, true).await;
        let history = crate::session::message::lock_history(&root.id).await;
        let mut pending = tokio::spawn({
            let tenant = tenant.clone();
            let id = root.id.clone();
            let path = directory.path().to_owned();
            async move { delete(&tenant, &id, &path).await }
        });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), &mut pending)
                .await
                .is_err(),
            "the request must wait for the held history guard"
        );
        archive(&tenant, &root.id, false).await;
        drop(history);
        assert_eq!(pending.await.unwrap().0, StatusCode::BAD_REQUEST);
        assert!(read(&tenant, &root.id).await.unwrap().time.archived.is_none());
    }

    #[tokio::test]
    async fn archived_chat_delete_storage_failure_keeps_root_and_allows_retry() {
        let tenant = tenant();
        let directory = TempDir::new().unwrap();
        let scope = SessionScope::new(tenant.clone(), directory.path()).unwrap();
        let root = create(&scope, None).await;
        let child = create(&scope, Some(&root.id)).await;
        let grandchild = create(&scope, Some(&child.id)).await;
        save_ui(&tenant, &child).await;
        save_ui(&tenant, &grandchild).await;
        archive(&tenant, &root.id, true).await;
        let database = std::env::temp_dir()
            .join(format!("vw-agent-test-global-store-{}.sqlite3", std::process::id()));
        let connection = rusqlite::Connection::open(database).unwrap();
        let trigger = format!("archived_delete_{}", uuid::Uuid::new_v4().simple());
        connection.execute_batch(&format!(
            "CREATE TRIGGER {trigger} BEFORE DELETE ON session_agent_sessions
             WHEN OLD.session_id = '{}' BEGIN SELECT RAISE(FAIL, 'injected deletion failure'); END;",
            child.id,
        )).unwrap();
        let (status, _) = delete(&tenant, &root.id, directory.path()).await;
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
        let (status, body) = delete(&tenant, &root.id, directory.path()).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["deleted_ids"], serde_json::json!([child.id, root.id]));
        assert!(read(&tenant, &root.id).await.is_none());
    }
}
