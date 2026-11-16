// 生产入口读取进程级 runtime_flags，保留宿主配置契约。
use super::*;
use axum::{Router, body::Body, http::{Request, StatusCode}, middleware, routing::get};
use tower::util::ServiceExt;

#[tokio::test]
async fn basic_auth_production_entry_allows_when_password_flag_is_absent() {
    let app = Router::new()
        .route("/protected", get(|| async { StatusCode::NO_CONTENT }))
        .layer(middleware::from_fn(basic_auth_middleware));
    let request =
        Request::builder().uri("/protected").body(Body::empty()).expect("request should build");

    let response = app.oneshot(request).await.expect("router should respond");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
