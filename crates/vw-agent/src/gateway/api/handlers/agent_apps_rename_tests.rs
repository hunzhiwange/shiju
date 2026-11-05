use crate::{agent_apps::store::IconLibraryStore, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vw_api_types::{
    agent_apps::{AgentAppsErrorCode, AgentAppsErrorResponse, CustomIconEntry},
    id::TenantScope,
};

const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";

fn verify(database: GlobalDatabase) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let tenant = TenantScope::new("p", "a")?;
    let other = TenantScope::new("p", "b")?;
    let store = IconLibraryStore::new(database.clone())?;
    let original = store.add_icon(&tenant, "原名", PNG, "fixed", 123).unwrap().icon;
    store.add_icon(&other, "其他公司", PNG, "fixed", 456).unwrap();
    let untouched = store.read_icons(&other)?;
    let app = Router::new()
        .nest("/v1", crate::gateway::router::handler_router())
        .layer(Extension(database))
        .layer(Extension(tenant.clone()));
    for name in [" 新名称 ".to_owned(), format!(" {} ", "图".repeat(64))] {
        let response = runtime
            .block_on(app.clone().oneshot(patch("fixed", serde_json::json!({"name":name}))))?;
        assert_eq!(response.status(), StatusCode::OK);
        let renamed: CustomIconEntry =
            serde_json::from_slice(&runtime.block_on(to_bytes(response.into_body(), 200_000))?)?;
        assert_eq!(renamed, CustomIconEntry { name: name.trim().to_owned(), ..original.clone() });
        let response = runtime.block_on(
            app.clone().oneshot(Request::get("/v1/agent-apps/custom-icons").body(Body::empty())?),
        )?;
        assert_eq!(response.status(), StatusCode::OK);
        let icons: Vec<CustomIconEntry> =
            serde_json::from_slice(&runtime.block_on(to_bytes(response.into_body(), 200_000))?)?;
        assert_eq!(icons, vec![renamed]);
    }
    let before = store.read_icons(&tenant)?;
    for value in [
        serde_json::json!({"name":""}),
        serde_json::json!({"name":" \t　"}),
        serde_json::json!({"name":"图".repeat(65)}),
        serde_json::json!({"name":42}),
        serde_json::json!({}),
    ] {
        let response = runtime.block_on(app.clone().oneshot(patch("fixed", value)))?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error: AgentAppsErrorResponse =
            serde_json::from_slice(&runtime.block_on(to_bytes(response.into_body(), 200_000))?)?;
        assert_eq!(error.code, AgentAppsErrorCode::InvalidInput);
    }
    let response =
        runtime.block_on(app.oneshot(patch("missing", serde_json::json!({"name":"名称"}))))?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let error: AgentAppsErrorResponse =
        serde_json::from_slice(&runtime.block_on(to_bytes(response.into_body(), 200_000))?)?;
    assert_eq!(error.code, AgentAppsErrorCode::NotFound);
    assert_eq!(store.read_icons(&tenant)?, before);
    assert_eq!(store.read_icons(&other)?, untouched);
    Ok(())
}
fn patch(id: &str, value: serde_json::Value) -> Request<Body> {
    Request::patch(format!("/v1/agent-apps/custom-icons/{id}"))
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}
#[test]
fn cc_base_icon_api_rename_sqlite_boundaries_and_scope() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    verify(GlobalDatabase::for_home(home.path())?)
}
#[test]
fn cc_base_icon_api_rename_postgres_boundaries_and_scope() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    verify(fixture.database().clone())
}
