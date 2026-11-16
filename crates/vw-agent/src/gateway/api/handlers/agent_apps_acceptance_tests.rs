//! Real gateway assembly injected into the domain icon acceptance suite.
use crate::global_database::{GlobalDatabase, test_support::PostgresTestDatabase};
use gateway_agent_apps::icon_acceptance::{roundtrip, rejection, disconnect, Gate, RequestCompletion};

fn router(key: &str, gate: Option<std::sync::Arc<Gate>>) -> axum::Router {
    use axum::{body::{Body, to_bytes}, http::{Request, StatusCode}, middleware::{Next, from_fn}, response::IntoResponse};
    let guard = crate::security::pairing::PairingGuard::new(true, &[key.to_owned()]);
    // Instance-local authentication uses the production Bearer parser and hashed skey guard.
    // No process-global credentials/configuration are changed by concurrent tests.
    crate::gateway::router::build_router(Vec::new())
        .layer(from_fn(move |req: Request<Body>, next: Next| {
            let guard = guard.clone();
            let gate = gate.clone();
            async move {
                let skey =
                    crate::gateway::api::auth::extract_auth_skey(req.headers()).unwrap_or("");
                if !guard.is_authenticated(skey) {
                    return StatusCode::UNAUTHORIZED.into_response();
                }
                if req.headers().contains_key("x-acceptance-gate") {
                    let gate = gate.expect("disconnect test gate");
                    let (parts, body) = req.into_parts();
                    let bytes =
                        to_bytes(body, 128 * 1024).await.expect("complete request body");
                    let _completion = RequestCompletion(gate.clone());
                    gate.received.notify_one();
                    gate.proceed.notified().await;
                    next.run(Request::from_parts(parts, Body::from(bytes))).await
                } else {
                    next.run(req).await
                }
            }
        }))
}

macro_rules! acceptance {
    ($sqlite:ident, $postgres:ident, $behavior:ident) => {
        #[test]
        fn $sqlite() -> anyhow::Result<()> {
            let home = tempfile::tempdir()?;
            tokio::runtime::Runtime::new()?
                .block_on($behavior(GlobalDatabase::for_home(home.path())?, router))
        }
        #[test]
        fn $postgres() -> anyhow::Result<()> {
            // Missing PostgreSQL is an error, never a skipped passing test.
            let fixture = PostgresTestDatabase::from_environment()?;
            tokio::runtime::Runtime::new()?.block_on($behavior(fixture.database().clone(), router))
        }
    };
}
acceptance!(
    cc_base_icon_headless_sqlite_roundtrip,
    cc_base_icon_headless_postgres_roundtrip,
    roundtrip
);
acceptance!(
    cc_base_icon_headless_sqlite_rejection_and_scope,
    cc_base_icon_headless_postgres_rejection_and_scope,
    rejection
);
acceptance!(
    cc_base_icon_headless_sqlite_disconnect_recovery,
    cc_base_icon_headless_postgres_disconnect_recovery,
    disconnect
);
