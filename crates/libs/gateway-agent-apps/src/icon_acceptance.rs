//! Icon contracts using the caller’s real gateway and authentication layers.
use axum::{body::{Body, to_bytes}, http::{Request, StatusCode}};
use tower::ServiceExt;
use agent_database::GlobalDatabase;
use axum::Router;
pub type RouterFactory = fn(&str, Option<std::sync::Arc<Gate>>) -> Router;
use {
    vw_api_app_identity::AddCustomIconRequest,
    vw_api_app_identity::RenameCustomIconRequest,
    vw_api_transport::id::TenantScope,
};

const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";

pub async fn roundtrip(db: GlobalDatabase, router: RouterFactory) -> anyhow::Result<()> {
    let server = Server::start(db, None, router).await.map_err(anyhow::Error::msg)?;
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
        vw_api_app_identity::CustomIconEntry { name: "新名称".into(), ..added.icon }
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

pub async fn rejection(db: GlobalDatabase, router: RouterFactory) -> anyhow::Result<()> {
    let server = Server::start(db, None, router).await.map_err(anyhow::Error::msg)?;
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

pub async fn disconnect(db: GlobalDatabase, router: RouterFactory) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    let gate = std::sync::Arc::new(Gate::default());
    let server = Server::start(db.clone(), Some(gate.clone()), router).await.map_err(anyhow::Error::msg)?;
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
    let after = vw_api_app_identity::CustomIconEntry {
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
        agent_app_services::store::IconLibraryStore::new(db)?
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

#[derive(Default)]
pub struct Gate {
    pub received: tokio::sync::Notify,
    pub proceed: tokio::sync::Notify,
    pub finished: tokio::sync::Notify,
}

// Hyper may cancel the request future when the peer disconnects. Signal both
// completion and cancellation so recovery never depends on a response being sent.
pub struct RequestCompletion(pub std::sync::Arc<Gate>);
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
    async fn start(db: GlobalDatabase, gate: Option<std::sync::Arc<Gate>>, router: RouterFactory) -> anyhow::Result<Self> {
        let key = uuid::Uuid::new_v4().to_string();
        let app = router(&key, gate).layer(axum::Extension(db));
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
    ) -> anyhow::Result<gateway_app_read::GatewayAppReadClient> {
        use gateway_endpoint::{GatewayAuth, GatewayEndpoint};
        gateway_http::GatewayTransport::new(
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
        .map(gateway_app_read::GatewayAppReadClient::new)
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

pub async fn check_scopes(db: GlobalDatabase, router: fn(GlobalDatabase) -> Router) -> anyhow::Result<()> {
    let seed_db = db.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
    let store = agent_app_services::store::IconLibraryStore::new(seed_db)?;
    let image = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";
    for company in ["a", "b"] {
        store.add_icon(&TenantScope::new("p", company)?, company, image, company, 1).unwrap();
    }
    Ok(())
    }).await??;
    for company in ["a", "b", "empty"] {
        let response =
            router(db.clone()).oneshot(request(Some(company), "Basic dGVzdDpzZWNyZXQ=")).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let entries: Vec<vw_api_app_identity::CustomIconEntry> =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await?)?;
        assert_eq!(entries.len(), usize::from(company != "empty"));
        if company != "empty" {
            assert_eq!(entries[0].id, company);
        }
    }
    Ok(())
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
