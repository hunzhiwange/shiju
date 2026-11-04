//! HTTP regression coverage using production authentication, tenant and connection middleware.
use crate::app::agent::gateway::{self, AppState, GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::project;
use crate::app::agent::{
    config::Config, memory::NoneMemory, providers::Provider, security::pairing::PairingGuard,
};
use base64::Engine;
use parking_lot::Mutex;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use vw_api_types::{
    file::{LocalImportRequest, MoveFileRequest},
    id::{ProjectScope, TenantScope},
};
use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};
struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

// 状态 fixture 关闭外部依赖，只保留路由装配所需的最小共享组件。
fn state() -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(true, &["drag-test-key".into()])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

struct Server {
    addr: SocketAddr,
    handle: tokio::task::JoinHandle<Result<(), gateway::ApiError>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
async fn server(bootstrap: bool) -> Server {
    server_on(bootstrap, "127.0.0.1").await
}
async fn server_on(bootstrap: bool, host: &str) -> Server {
    if bootstrap {
        let state = state();
        let router = gateway::bootstrap::client_router(
            state.clone(),
            axum::Router::new(),
            axum::Router::new(),
        )
        .with_state(state);
        let listener = tokio::net::TcpListener::bind((host, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .map_err(|error| gateway::ApiError::internal(error.to_string()))
        });
        Server { addr, handle }
    } else {
        let (addr, task) =
            gateway::start(gateway::ServeOptions { hostname: host.into(), port: 0, cors: vec![] })
                .await
                .unwrap();
        Server { addr, handle: task }
    }
}
struct Fixture {
    scope: ProjectScope,
    root: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self::in_tenant(TenantScope::new("10001", "100010005").unwrap())
    }
    fn in_tenant(tenant: TenantScope) -> Self {
        let root = tempfile::tempdir().unwrap();
        let id = format!("drag-{}", uuid::Uuid::new_v4());
        let scope = ProjectScope::new(tenant.clone(), id.clone().into()).unwrap();
        project::runtime_store()
            .unwrap()
            .create(
                &tenant,
                &project::Info {
                    id,
                    worktree: root.path().to_str().unwrap().into(),
                    vcs: None,
                    name: None,
                    icon: None,
                    commands: None,
                    time: project::TimeInfo { created: 1, updated: 1, initialized: None },
                    sandboxes: vec![],
                },
            )
            .unwrap();
        Self { scope, root }
    }
    fn client(&self, server: &Server) -> GatewayClient {
        GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", server.addr.port())
                .with_tenant_scope(self.scope.tenant_scope().clone())
                .with_auth(GatewayAuth { skey: Some("drag-test-key".into()) }),
        )
        .unwrap()
    }
    fn import(&self, source_paths: Vec<String>) -> LocalImportRequest {
        LocalImportRequest {
            project_id: self.scope.project_id().clone(),
            worktree_id: None,
            target_directory: "".into(),
            source_paths,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        project::runtime_store().unwrap().delete(&self.scope).unwrap();
    }
}

#[tokio::test]
async fn file_drag_integration_both_listeners_preserve_data_and_reject_conflicts() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        let fixture = Fixture::new();
        let client = fixture.client(&server);
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(source.path().join("中文/empty/nested")).unwrap();
        std::fs::write(source.path().join("中文/.binary"), [0, 255, 17]).unwrap();
        std::fs::write(source.path().join("file.txt"), "original").unwrap();
        let paths = vec![
            source.path().join("中文").to_str().unwrap().into(),
            source.path().join("file.txt").to_str().unwrap().into(),
        ];
        let request = fixture.import(paths);
        let imported = client.file_import_local(&request).await.unwrap();
        assert_eq!(imported.items.len(), 2);
        assert!(imported.items.iter().all(|item| item.ok));
        assert!(fixture.root.path().join("中文/empty/nested").is_dir());
        assert_eq!(std::fs::read(fixture.root.path().join("中文/.binary")).unwrap(), [0, 255, 17]);
        std::fs::write(source.path().join("file.txt"), "changed source").unwrap();
        assert!(
            client.file_import_local(&request).await.unwrap().items.iter().all(|item| !item.ok)
        );
        assert_eq!(
            std::fs::read_to_string(fixture.root.path().join("file.txt")).unwrap(),
            "original"
        );
        let movement = MoveFileRequest {
            project_id: request.project_id.clone(),
            worktree_id: None,
            from_path: "中文".into(),
            to_path: "nested/中文".into(),
            overwrite: false,
        };
        client.file_move(&movement).await.unwrap();
        assert!(!fixture.root.path().join("中文").exists());
        assert!(fixture.root.path().join("nested/中文/empty/nested").is_dir());
        assert_eq!(
            std::fs::read(fixture.root.path().join("nested/中文/.binary")).unwrap(),
            [0, 255, 17]
        );
        assert_eq!(std::fs::read(source.path().join("中文/.binary")).unwrap(), [0, 255, 17]);
        let foreign = tempfile::tempdir().unwrap();
        let mut invalid = movement.clone();
        invalid.worktree_id = Some(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(foreign.path().to_str().unwrap())
                .into(),
        );
        invalid.from_path = "file.txt".into();
        invalid.to_path = "outside.txt".into();
        assert!(client.file_move(&invalid).await.is_err());
        let mut invalid_import = request.clone();
        invalid_import.worktree_id = invalid.worktree_id;
        assert!(client.file_import_local(&invalid_import).await.is_err());
        assert_eq!(std::fs::read_dir(foreign.path()).unwrap().count(), 0);
        assert_eq!(
            std::fs::read_to_string(fixture.root.path().join("file.txt")).unwrap(),
            "original"
        );
    }
}

#[tokio::test]
async fn file_drag_integration_auth_tenant_origin_rejections_have_no_disk_effects() {
    let server = server(true).await;
    let fixture = Fixture::new();
    let source = tempfile::tempdir().unwrap();
    let path = source.path().join("secret.txt");
    std::fs::write(&path, "retained").unwrap();
    let request = fixture.import(vec![path.to_str().unwrap().into()]);
    let http = reqwest::Client::new();
    for (auth, tenant, origin, expected) in [
        (false, true, None, 401),
        (true, false, None, 400),
        (true, true, Some("https://evil.example"), 403),
    ] {
        let mut req = http
            .post(format!("http://{}/v1/files/import-local", server.addr))
            .json(&request)
            .header("x-forwarded-for", "127.0.0.1")
            .header("x-forwarded-host", "localhost");
        if auth {
            req = req.bearer_auth("drag-test-key");
        }
        if tenant {
            req = req.header("x-platform-id", "10001").header("x-company-id", "100010005");
        }
        if let Some(origin) = origin {
            req = req.header("origin", origin);
        }
        assert_eq!(req.send().await.unwrap().status().as_u16(), expected);
        assert_eq!(std::fs::read_dir(fixture.root.path()).unwrap().count(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "retained");
    }
    let wrong_tenant = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", server.addr.port())
            .with_tenant_scope(TenantScope::new("10002", "100020005").unwrap())
            .with_auth(GatewayAuth { skey: Some("drag-test-key".into()) }),
    )
    .unwrap();
    assert!(wrong_tenant.file_import_local(&request).await.is_err());
    assert_eq!(std::fs::read_dir(fixture.root.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn file_drag_integration_recursive_link_failure_keeps_source_and_other_items() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        let fixture = Fixture::new();
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("linked")).unwrap();
        std::fs::write(source.path().join("good"), [255, 0]).unwrap();
        std::os::unix::fs::symlink(source.path().join("good"), source.path().join("linked/link"))
            .unwrap();
        let result = fixture
            .client(&server)
            .file_import_local(&fixture.import(vec![
                source.path().join("linked").to_str().unwrap().into(),
                source.path().join("good").to_str().unwrap().into(),
            ]))
            .await
            .unwrap();
        assert!(!result.items[0].ok);
        assert!(result.items[1].ok);
        assert!(!fixture.root.path().join("linked").exists());
        assert_eq!(std::fs::read_dir(fixture.root.path()).unwrap().count(), 1);
        assert_eq!(std::fs::read(fixture.root.path().join("good")).unwrap(), [255, 0]);
        assert!(
            source.path().join("linked/link").symlink_metadata().unwrap().file_type().is_symlink()
        );
    }
}

// A default route can point at a VPN address that cannot accept local TCP traffic.
// Enumerate assigned addresses and probe them instead of assuming that route is usable.
#[cfg(unix)]
fn assigned_non_loopback_addresses() -> Vec<std::net::Ipv4Addr> {
    let mut head = std::ptr::null_mut();
    // SAFETY: getifaddrs initializes head on success; every node and address remains
    // valid until freeifaddrs, and sockaddr_in is read only after checking AF_INET.
    unsafe {
        assert_eq!(libc::getifaddrs(&mut head), 0);
        let mut current = head;
        let mut addresses = Vec::new();
        while !current.is_null() {
            let address = (*current).ifa_addr;
            if !address.is_null() && i32::from((*address).sa_family) == libc::AF_INET {
                let address = &*address.cast::<libc::sockaddr_in>();
                let ip = std::net::Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes());
                if !ip.is_loopback() && !ip.is_unspecified() {
                    addresses.push(ip);
                }
            }
            current = (*current).ifa_next;
        }
        libc::freeifaddrs(head);
        addresses
    }
}

#[cfg(unix)]
#[tokio::test]
async fn file_drag_integration_non_loopback_cannot_spoof_forwarded_headers() {
    for bootstrap in [true, false] {
        let server = server_on(bootstrap, "0.0.0.0").await;
        let mut reachable = None;
        for ip in assigned_non_loopback_addresses() {
            if let Ok(Ok(connection)) = tokio::time::timeout(
                Duration::from_millis(500),
                tokio::net::TcpStream::connect((ip, server.addr.port())),
            )
            .await
            {
                if !connection.local_addr().unwrap().ip().is_loopback() {
                    reachable = Some(ip);
                    break;
                }
            }
        }
        let local_ip =
            reachable.expect("a locally reachable non-loopback IPv4 interface is required");
        let fixture = Fixture::new();
        let source = tempfile::tempdir().unwrap();
        let path = source.path().join("retained.txt");
        std::fs::write(&path, "retained").unwrap();
        let response = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://{local_ip}:{}/v1/files/import-local", server.addr.port()))
            .bearer_auth("drag-test-key")
            .header("x-platform-id", "10001")
            .header("x-company-id", "100010005")
            .header("x-forwarded-for", "127.0.0.1")
            .header("x-forwarded-host", "localhost")
            .json(&fixture.import(vec![path.to_str().unwrap().into()]))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
        assert_eq!(std::fs::read_dir(fixture.root.path()).unwrap().count(), 0);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "retained");
    }
}

#[tokio::test]
async fn file_drag_integration_default_tenant_cannot_be_changed_from_another_tenant() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        let fixture = Fixture::in_tenant(TenantScope::default());
        std::fs::write(fixture.root.path().join("keep.txt"), "original").unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = source.path().join("import.txt");
        std::fs::write(&path, "retained").unwrap();
        let other = GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", server.addr.port())
                .with_tenant_scope(TenantScope::new("10002", "100020005").unwrap())
                .with_auth(GatewayAuth { skey: Some("drag-test-key".into()) }),
        )
        .unwrap();
        assert!(
            other
                .file_import_local(&fixture.import(vec![path.to_str().unwrap().into()]))
                .await
                .is_err()
        );
        let movement = MoveFileRequest {
            project_id: fixture.scope.project_id().clone(),
            worktree_id: None,
            from_path: "keep.txt".into(),
            to_path: "moved.txt".into(),
            overwrite: false,
        };
        assert!(other.file_move(&movement).await.is_err());
        if bootstrap {
            let response = reqwest::Client::new()
                .post(format!("http://{}/v1/files/move", server.addr))
                .header("x-platform-id", "10002")
                .header("x-company-id", "100020005")
                .json(&movement)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
        }
        assert_eq!(
            std::fs::read_to_string(fixture.root.path().join("keep.txt")).unwrap(),
            "original"
        );
        assert_eq!(std::fs::read_dir(fixture.root.path()).unwrap().count(), 1);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "retained");
    }
}
