//! macOS loopback acceptance: production router, SDK and real isolated databases.
use crate::global_database::{GlobalDatabase, test_support::PostgresTestDatabase};
use vw_api_types::{
    agent_apps::{AddCustomIconRequest, RenameCustomIconRequest},
    id::TenantScope,
};

const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";

async fn roundtrip(db: GlobalDatabase) -> anyhow::Result<()> {
    let server = Server::start(db, None).await.map_err(anyhow::Error::msg)?;
    let client = server.client("p", "a", true)?;
    assert!(client.custom_icons_list().await.map_err(anyhow::Error::msg)?.is_empty());
    let request = AddCustomIconRequest { name: "工作图标".into(), data: PNG.into() };
    let added = client.custom_icon_add(&request).await.map_err(anyhow::Error::msg)?;
    assert!(!added.existed);
    assert!(!added.icon.id.is_empty());
    assert!(added.icon.created_at > 0);
    assert_eq!(added.icon.name, request.name);
    assert_eq!(added.icon.data, PNG);
    let duplicate = client.custom_icon_add(&request).await.map_err(anyhow::Error::msg)?;
    assert!(duplicate.existed);
    assert_eq!(duplicate.icon, added.icon);
    let renamed = client
        .custom_icon_rename(&added.icon.id, &RenameCustomIconRequest { name: "新名称".into() })
        .await
        .map_err(anyhow::Error::msg)?;
    assert_eq!(
        renamed,
        vw_api_types::agent_apps::CustomIconEntry { name: "新名称".into(), ..added.icon }
    );
    assert_eq!(
        client.custom_icons_list().await.map_err(anyhow::Error::msg)?,
        vec![renamed.clone()]
    );
    client.custom_icon_delete(&renamed.id).await.map_err(anyhow::Error::msg)?;
    client.custom_icon_delete(&renamed.id).await.map_err(anyhow::Error::msg)?;
    assert!(client.custom_icons_list().await.map_err(anyhow::Error::msg)?.is_empty());
    server.stop().await.map_err(anyhow::Error::msg)?;
    Ok(())
}

async fn rejection(db: GlobalDatabase) -> anyhow::Result<()> {
    let server = Server::start(db, None).await.map_err(anyhow::Error::msg)?;
    let client = server.client("p", "a", true)?;
    let added = client
        .custom_icon_add(&AddCustomIconRequest { name: "原名".into(), data: PNG.into() })
        .await
        .map_err(anyhow::Error::msg)?
        .icon;
    let bad = server.client("p", "a", false)?;
    assert!(bad.custom_icons_list().await.unwrap_err().contains("401"));
    assert!(
        bad.custom_icon_add(&AddCustomIconRequest { name: "拒绝".into(), data: PNG.into() })
            .await
            .unwrap_err()
            .contains("401")
    );
    assert!(
        bad.custom_icon_rename(&added.id, &RenameCustomIconRequest { name: "拒绝".into() })
            .await
            .unwrap_err()
            .contains("401")
    );
    assert!(bad.custom_icon_delete(&added.id).await.unwrap_err().contains("401"));
    let http = reqwest::Client::builder().no_proxy().build()?;
    for method in [
        reqwest::Method::GET,
        reqwest::Method::POST,
        reqwest::Method::PATCH,
        reqwest::Method::DELETE,
    ] {
        let path = if matches!(method, reqwest::Method::PATCH | reqwest::Method::DELETE) {
            format!("/{}", added.id)
        } else {
            String::new()
        };
        for headers in [vec![], vec![("x-platform-id", "p")], vec![("x-company-id", "a")]] {
            let mut request = http
                .request(
                    method.clone(),
                    format!("{}/v1/agent-apps/custom-icons{path}", server.url()),
                )
                .bearer_auth(&server.key)
                .json(&serde_json::json!({"name":"拒绝","data":PNG}));
            for (name, value) in headers {
                request = request.header(name, value);
            }
            let response = request.send().await?;
            assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        }
    }

    for (platform, company) in [("p", "b"), ("other", "a")] {
        let other = server.client(platform, company, true)?;
        assert!(other.custom_icons_list().await.map_err(anyhow::Error::msg)?.is_empty());
        assert!(
            other
                .custom_icon_rename(&added.id, &RenameCustomIconRequest { name: "越界".into() })
                .await
                .unwrap_err()
                .contains("404")
        );
        other.custom_icon_delete(&added.id).await.map_err(anyhow::Error::msg)?;
        let own = other
            .custom_icon_add(&AddCustomIconRequest { name: "隔离".into(), data: PNG.into() })
            .await
            .map_err(anyhow::Error::msg)?
            .icon;
        assert_ne!(own.id, added.id);
        assert_eq!(other.custom_icons_list().await.map_err(anyhow::Error::msg)?, vec![own]);
    }
    assert!(
        client
            .custom_icon_rename(&added.id, &RenameCustomIconRequest { name: "图".repeat(65) })
            .await
            .unwrap_err()
            .contains("400")
    );
    assert!(
        client
            .custom_icon_add(&AddCustomIconRequest {
                name: "非法".into(),
                data: "/tmp/icon.png".into()
            })
            .await
            .unwrap_err()
            .contains("400")
    );
    assert_eq!(client.custom_icons_list().await.map_err(anyhow::Error::msg)?, vec![added]);
    server.stop().await.map_err(anyhow::Error::msg)?;
    Ok(())
}

async fn disconnect(db: GlobalDatabase) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    let gate = std::sync::Arc::new(Gate::default());
    let server = Server::start(db.clone(), Some(gate.clone())).await.map_err(anyhow::Error::msg)?;
    let before = server
        .client("p", "a", true)?
        .custom_icon_add(&AddCustomIconRequest { name: "保留".into(), data: PNG.into() })
        .await
        .map_err(anyhow::Error::msg)?
        .icon;
    let body =
        serde_json::to_string(&RenameCustomIconRequest { name: "断开后完整名称".into() })?;
    let mut socket =
        tokio::net::TcpStream::connect(server.addr).await.map_err(anyhow::Error::msg)?;
    let request = format!(
        "PATCH /v1/agent-apps/custom-icons/{} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nx-platform-id: p\r\nx-company-id: a\r\nx-acceptance-gate: true\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        before.id,
        server.addr,
        server.key,
        body.len(),
        body
    );
    socket.write_all(request.as_bytes()).await.map_err(anyhow::Error::msg)?;
    tokio::time::timeout(std::time::Duration::from_secs(10), gate.received.notified())
        .await
        .map_err(anyhow::Error::msg)?;
    // The server has consumed the entire body, but has not entered the real handler yet.
    socket.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(socket);
    gate.proceed.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(10), gate.finished.notified())
        .await
        .map_err(anyhow::Error::msg)?;
    let actual =
        server.client("p", "a", true)?.custom_icons_list().await.map_err(anyhow::Error::msg)?;
    let after = vw_api_types::agent_apps::CustomIconEntry {
        name: "断开后完整名称".into(),
        ..before.clone()
    };
    assert!(
        actual == vec![before.clone()] || actual == vec![after.clone()],
        "partial mutation: {actual:?}"
    );
    eprintln!(
        "disconnect observed {}",
        if actual == vec![before.clone()] {
            "complete pre-commit state"
        } else {
            "complete post-commit state"
        }
    );
    let stored = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        crate::agent_apps::store::IconLibraryStore::new(db)?
            .read_icons(&TenantScope::new("p", "a")?)
    })
    .await??;
    assert_eq!(stored, actual);

    server
        .client("p", "a", true)?
        .custom_icon_delete(&before.id)
        .await
        .map_err(anyhow::Error::msg)?;
    assert!(
        server
            .client("p", "a", true)?
            .custom_icons_list()
            .await
            .map_err(anyhow::Error::msg)?
            .is_empty()
    );
    server.stop().await.map_err(anyhow::Error::msg)?;
    Ok(())
}

macro_rules! acceptance {
    ($sqlite:ident, $postgres:ident, $behavior:ident) => {
        #[test]
        fn $sqlite() -> anyhow::Result<()> {
            let home = tempfile::tempdir()?;
            tokio::runtime::Runtime::new()?
                .block_on($behavior(GlobalDatabase::for_home(home.path())?))
        }
        #[test]
        fn $postgres() -> anyhow::Result<()> {
            // Missing PostgreSQL is an error, never a skipped passing test.
            let fixture = PostgresTestDatabase::from_environment()?;
            tokio::runtime::Runtime::new()?.block_on($behavior(fixture.database().clone()))
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

#[derive(Default)]
struct Gate {
    received: tokio::sync::Notify,
    proceed: tokio::sync::Notify,
    finished: tokio::sync::Notify,
}

// Hyper may cancel the request future when the peer disconnects. Signal both
// completion and cancellation so recovery never depends on a response being sent.
struct RequestCompletion(std::sync::Arc<Gate>);
impl Drop for RequestCompletion {
    fn drop(&mut self) {
        self.0.finished.notify_one();
    }
}

struct Server {
    addr: std::net::SocketAddr,
    key: String,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
}

impl Server {
    async fn start(db: GlobalDatabase, gate: Option<std::sync::Arc<Gate>>) -> anyhow::Result<Self> {
        use axum::{
            body::{Body, to_bytes},
            http::{Request, StatusCode},
            middleware::{Next, from_fn},
            response::IntoResponse,
        };
        let key = uuid::Uuid::new_v4().to_string();
        let guard = crate::security::pairing::PairingGuard::new(true, std::slice::from_ref(&key));
        // Instance-local authentication uses the production Bearer parser and hashed skey guard.
        // No process-global credentials/configuration are changed by concurrent tests.
        let app = crate::gateway::router::build_router(Vec::new())
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
            .layer(axum::Extension(db));
        let listener =
            tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.map_err(anyhow::Error::msg)?;
        let addr = listener.local_addr()?;
        let (shutdown, receive) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = receive.await;
                })
                .await
        });
        Ok(Self { addr, key, shutdown: Some(shutdown), task: Some(task) })
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn client(
        &self,
        platform: &str,
        company: &str,
        authenticated: bool,
    ) -> anyhow::Result<vw_gateway_client::GatewayClient> {
        use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};
        GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", self.addr.port())
                .with_tenant_scope(TenantScope::new(platform, company)?)
                .with_auth(GatewayAuth {
                    skey: Some(if authenticated {
                        self.key.clone()
                    } else {
                        "invalid-test-key".into()
                    }),
                }),
        )
        .map_err(anyhow::Error::msg)
    }

    async fn stop(mut self) -> anyhow::Result<()> {
        let _ = self.shutdown.take().expect("shutdown sender").send(());
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.task.take().expect("server task"),
        )
        .await
        .map_err(anyhow::Error::msg)???;
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
