use crate::AppRoutingError;
use gateway_endpoint::GatewayEndpoint;
use gateway_http::GatewayTransport;
use vw_api_transport::id::TenantScope;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, os::unix::fs::PermissionsExt, sync::Arc};
use app_routing_services::RoutingListener;
use agent_database::GlobalDatabase;
use crate::server::lifecycle_api::{RoutingPaths, configure_with_policy};
pub(crate) const HOST: &str = "routing-mac";
pub(crate) const MANAGER: &str = "fixture-routing-manager-token-00001";
pub(crate) const READER: &str = "fixture-routing-reader-token-000002";
pub(crate) const MEMBER: &str = "fixture-routing-member-token-000003";

fn grant(token: &str, read: bool, manage: bool) -> Value {
    json!({
        "id": token,
        "token_sha256": hex::encode(Sha256::digest(token)),
        "grants": [{
            "platform_id": "p", "company_id": "owner", "host_id": HOST,
            "routing_read": read, "routing_manage": manage
        }]
    })
}

pub(crate) fn write_policy(path: &Path, home: &Path, port: u16) {
    let host = |id| {
        json!({
            "id": id,
            "online": true,
            "binding": "local",
            "home": home,
            "search_paths": [],
            "applications": home.join("Applications"),
            "credential_root": home.join("credentials"),
        })
    };
    let mut owner = host(HOST);
    owner["routing_startup"] = json!({"enabled": false, "listen_port": port});
    let policy = json!({
        "hosts": [owner, host("other-mac")],
        "principals": [
            grant(MANAGER, true, true), grant(READER, true, false), grant(MEMBER, false, false)
        ],
    });
    fs::write(path, policy.to_string()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

pub(crate) fn client(port: u16, company: &str) -> GatewayTransport {
    GatewayTransport::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", company).unwrap()),
    )
    .unwrap()
}

pub(crate) fn assert_error(error: AppRoutingError, status: u16, code: &str) {
    assert!(
        matches!(&error, AppRoutingError::Http { status: actual, body: Some(body) }
            if *actual == status && body.error.code == code),
        "expected HTTP {status} {code}, got {error:?}"
    );
}

pub(crate) fn write_attached_app(home: &Path) -> (std::path::PathBuf, String) {
    use vw_agent_apps::providers::live::commit::{ExecutionHostId, ProviderTransactions};

    let root = home.join("credentials");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let _store = ProviderTransactions::new(&root, ExecutionHostId::new(HOST).unwrap()).unwrap();
    let host_state = fs::read_dir(root.join("provider-live"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("codex.state.json");
    let attached = json!({
        "mode": {"mode": "proxy", "attached": true, "proxy_route": "fixture-target", "contract": null},
        "mode_tenant": null,
        "pointer": null
    }).to_string();
    fs::write(&host_state, &attached).unwrap();
    fs::set_permissions(&host_state, fs::Permissions::from_mode(0o600)).unwrap();
    (host_state, attached)
}


// The fixture uses the real persistence and aggregation executors with explicit
// paths; no process-global home or policy is changed by a domain test.
pub(crate) struct Lifecycle(pub(crate) agent_gateway::aggregation_lifecycle::AggregationLifecycle);
struct Publication;
impl agent_gateway::publication_context::PublicationHost for Publication {
    fn revision(&self, c: &agent_gateway::publication_context::PublicationContext, provider: Option<&str>) -> Result<String, axum::http::StatusCode> {
        use gateway_aggregation::aggregation_read_snapshot::{load, revision};
        let snapshot = load(&c.database, &c.project, &c.host, c.app, &c.home, &c.expected, provider)?;
        revision(&c.project, &c.host, c.app, &c.principal, &c.expected, &snapshot)
    }
    fn publish_aggregation(&self, c: &agent_gateway::publication_context::PublicationContext, default: &str, policy: vw_agent_apps::aggregation::codex_publish::CatalogPolicy, row: &Value) -> Result<bool, axum::http::StatusCode> {
        c.publish_aggregation(default, policy, row)
    }
}
impl app_routing_services::lifecycle_host::AggregationLifecycleHost for Lifecycle {
    fn resume(&self, routing: Arc<RoutingListener>) -> Result<(), axum::http::StatusCode> {
        self.0.resume(&Publication, routing)
    }
    fn detach(&self) -> app_routing_services::lifecycle_host::DetachResult {
        self.0.detach(&Publication)
    }
}
pub(crate) struct GatewayProcess(tokio::task::JoinHandle<()>, Arc<RoutingListener>);
impl Drop for GatewayProcess {
    fn drop(&mut self) {
        self.0.abort();
        let routing = self.1.clone();
        tokio::spawn(async move { let _ = routing.stop().await; });
    }
}
pub(crate) async fn spawn_gateway(home: &Path, policy: &Path) -> (GatewayProcess, u16) {
    let routing = Arc::new(RoutingListener::default());
    let database = GlobalDatabase::for_home(home).unwrap();
    let app = axum::Router::new().nest("/v1", crate::server::router().merge(crate::server::agent_apps_high_availability::router()));
    let app = configure_with_policy(app, &routing, policy.to_owned(), database.clone(),
        RoutingPaths { home: home.to_owned(), config: home.join(".config/shiju"), state: home.join(".local/state/shiju") },
        |database, policy, tenant, host| Lifecycle(agent_gateway::aggregation_lifecycle::AggregationLifecycle { database, policy, tenant, host }),
    ).await.layer(axum::Extension(database)).layer(axum::middleware::from_fn(tenant));
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    let task = tokio::spawn(async move { axum::serve(socket, app).await.unwrap(); });
    (GatewayProcess(task, routing), port)
}
async fn tenant(mut request: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    let platform = request.headers()["x-platform-id"].to_str().unwrap();
    let company = request.headers()["x-company-id"].to_str().unwrap();
    let tenant = TenantScope::new(platform, company).unwrap();
    request.extensions_mut().insert(tenant);
    next.run(request).await
}
