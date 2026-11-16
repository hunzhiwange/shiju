use crate::global_database::GlobalDatabase;
use gateway_agent_apps::icon_acceptance::check_scopes;
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
};
use tower::ServiceExt;

fn router(database: GlobalDatabase) -> Router {
    Router::new()
        .nest("/v1", crate::gateway::router::handler_router())
        .layer(Extension(database))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
        .layer(middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }))
}
fn request(company: Option<&str>, auth: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .uri("/v1/agent-apps/custom-icons?command=touch&path=/tmp/never&key=other")
        .header("authorization", auth)
        .header("x-platform-id", "p");
    if let Some(company) = company {
        builder = builder.header("x-company-id", company);
    }
    builder.body(Body::empty()).unwrap()
}
#[tokio::test]
async fn cc_base_icon_api_read_scoped_sqlite() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    check_scopes(GlobalDatabase::for_home(home.path())?, router).await
}
#[test]
fn cc_base_icon_api_read_scoped_postgres() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(check_scopes(fixture.database().clone(), router))
}
#[tokio::test]
async fn cc_base_icon_api_read_rejects_auth_and_tenant_headers() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let app = router(GlobalDatabase::for_home(home.path())?);
    assert_eq!(
        app.clone().oneshot(request(Some("a"), "Basic YmFkOmJhZA==")).await?.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.clone().oneshot(request(None, "Basic dGVzdDpzZWNyZXQ=")).await?.status(),
        StatusCode::BAD_REQUEST
    );
    let mut duplicate = request(Some("a"), "Basic dGVzdDpzZWNyZXQ=");
    duplicate.headers_mut().append("x-company-id", "b".parse()?);
    assert_eq!(app.oneshot(duplicate).await?.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
