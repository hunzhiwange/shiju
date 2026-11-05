use crate::{agent_apps::store::IconLibraryStore, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vw_api_types::{agent_apps::CustomIconEntry, id::TenantScope};

const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";

fn verify(database: GlobalDatabase) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let tenant = TenantScope::new("p", "a")?;
    let other = TenantScope::new("p", "b")?;
    let store = IconLibraryStore::new(database.clone())?;
    store.add_icon(&tenant, "删除", PNG, "fixed", 123).unwrap();
    store.add_icon(&other, "其他公司", PNG, "fixed", 456).unwrap();
    let untouched = store.read_icons(&other)?;
    let app = Router::new()
        .nest("/v1", crate::gateway::router::handler_router())
        .layer(Extension(database))
        .layer(Extension(tenant.clone()));
    for id in ["missing", "fixed", "fixed"] {
        let response = runtime.block_on(app.clone().oneshot(
            Request::delete(format!("/v1/agent-apps/custom-icons/{id}")).body(Body::empty())?,
        ))?;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(runtime.block_on(to_bytes(response.into_body(), 200_000))?.is_empty());
        assert_eq!(store.read_icons(&other)?, untouched);
        assert_eq!(store.read_icons(&tenant)?.len(), usize::from(id == "missing"));
    }
    let response = runtime
        .block_on(app.oneshot(Request::get("/v1/agent-apps/custom-icons").body(Body::empty())?))?;
    assert_eq!(response.status(), StatusCode::OK);
    let icons: Vec<CustomIconEntry> =
        serde_json::from_slice(&runtime.block_on(to_bytes(response.into_body(), 200_000))?)?;
    assert!(icons.is_empty());
    Ok(())
}
#[test]
fn cc_base_icon_api_delete_sqlite_idempotent_and_scoped() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    verify(GlobalDatabase::for_home(home.path())?)
}
#[test]
fn cc_base_icon_api_delete_postgres_idempotent_and_scoped() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    verify(fixture.database().clone())
}
