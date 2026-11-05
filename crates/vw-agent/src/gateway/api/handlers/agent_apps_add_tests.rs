use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::Engine;
use tower::ServiceExt;
use vw_api_types::{
    agent_apps::{AddCustomIconResult, CustomIconEntry},
    id::TenantScope,
};
const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";
fn router(db: GlobalDatabase) -> Router {
    Router::new()
        .nest("/v1", crate::gateway::router::handler_router())
        .layer(Extension(db))
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
}
fn post(body: String) -> Request<Body> {
    Request::post("/v1/agent-apps/custom-icons")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}
async fn read(app: Router) -> anyhow::Result<Vec<CustomIconEntry>> {
    let response =
        app.oneshot(Request::get("/v1/agent-apps/custom-icons").body(Body::empty())?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(serde_json::from_slice(&to_bytes(response.into_body(), 200_000).await?)?)
}
async fn add_and_deduplicate(db: GlobalDatabase) -> anyhow::Result<()> {
    let app = router(db);
    let body = serde_json::json!({"name":"图标", "data":PNG}).to_string();
    let response = app.clone().oneshot(post(body.clone())).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let first: AddCustomIconResult =
        serde_json::from_slice(&to_bytes(response.into_body(), 200_000).await?)?;
    assert!(!first.existed);
    assert!(!first.icon.id.is_empty());
    assert!(first.icon.created_at > 0);
    let response = app.clone().oneshot(post(body)).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let duplicate: AddCustomIconResult =
        serde_json::from_slice(&to_bytes(response.into_body(), 200_000).await?)?;
    assert!(duplicate.existed);
    assert_eq!(duplicate.icon, first.icon);
    assert_eq!(read(app).await?, vec![first.icon]);
    Ok(())
}
#[tokio::test]
async fn cc_base_icon_api_add_sqlite_read_and_deduplicate() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    add_and_deduplicate(GlobalDatabase::for_home(home.path())?).await
}
#[test]
fn cc_base_icon_api_add_postgres_read_and_deduplicate() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(add_and_deduplicate(fixture.database().clone()))
}
#[tokio::test]
async fn cc_base_icon_api_add_invalid_input_does_not_write() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let app = router(GlobalDatabase::for_home(home.path())?);
    for value in [
        serde_json::json!({"name":"bad", "data":"/tmp/icon.png"}),
        serde_json::json!({"name":"bad", "data":"data:image/png;base64,!!!"}),
        serde_json::json!({"name":"", "data":PNG}),
        serde_json::json!({"command":"add_custom_icon", "path":"/tmp/icon.png"}),
    ] {
        assert_eq!(
            app.clone().oneshot(post(value.to_string())).await?.status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(read(app).await?.is_empty());
    Ok(())
}
#[tokio::test]
async fn cc_base_icon_api_add_body_limit() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let app = router(GlobalDatabase::for_home(home.path())?);
    // JSON whitespace permits a valid PNG request exactly at the fixed transport limit.
    let mut bytes =
        base64::engine::general_purpose::STANDARD.decode(PNG.split_once(',').unwrap().1)?;
    // Largest base64 payload fitting the native 64 KiB data URI boundary.
    bytes.resize(((64 * 1024 - "data:image/png;base64,".len()) / 4) * 3, 0);
    let data = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    assert!(vw_agent_apps::icons::validation::is_valid_custom_icon(&data));
    let mut body = serde_json::json!({"name":"icon", "data":data}).to_string();
    body.push_str(&" ".repeat(128 * 1024 - body.len()));
    assert_eq!(app.clone().oneshot(post(body.clone())).await?.status(), StatusCode::OK);
    body.push(' ');
    assert_eq!(app.clone().oneshot(post(body)).await?.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(read(app).await?.len(), 1);
    Ok(())
}
