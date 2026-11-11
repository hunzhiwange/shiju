use std::{io::Write, os::unix::fs::PermissionsExt, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use futures_util::{StreamExt, stream};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{
    io::AsyncReadExt,
    sync::{Notify, broadcast},
    time::timeout,
};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailureCode, DiagnosticJobIdentity,
        DiagnosticJobSnapshot, DiagnosticJobState, DiagnosticOutcome, DiagnosticResult,
        DiagnosticScope,
    },
    id::ProjectScope,
    mcp_management::{
        McpCatalogRecord, McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpFailureCode,
        McpRecordId, McpTransport,
    },
};

use super::{
    McpCatalogStore,
    catalog_create_tests::{http_draft, with_database},
    network_probe::{McpHandshakeStartError, start_with_source},
};
use crate::{
    gateway::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticJobs},
    global_database::{GlobalDatabase, run_sync_database},
};

const TOKEN: &str = "fixture-member-token-for-mcp-probe";
const OTHER_TOKEN: &str = "fixture-other-token-for-mcp-probe";

struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    catalog: McpCatalogStore,
    jobs: DiagnosticJobs,
    owner: DiagnosticJobOwner,
}

impl Fixture {
    fn new(db: GlobalDatabase) -> anyhow::Result<Self> {
        let root = tempfile::tempdir()?;
        let project =
            ProjectScope::new(TenantScope::new("platform", "company")?, "project".into())?;
        Ok(Self {
            catalog: McpCatalogStore::new(db.clone()).unwrap(),
            jobs: DiagnosticJobs::open(root.path()).unwrap(),
            db,
            root,
            owner: DiagnosticJobOwner {
                principal: "fixture-member".into(),
                scope: DiagnosticScope {
                    project,
                    target_host_id: "local-fixture".into(),
                    app_id: AppId::Claude,
                },
            },
        })
    }

    fn catalog_scope(&self) -> McpCatalogScope {
        McpCatalogScope::Project { scope: self.owner.scope.project.clone() }
    }

    fn policy_path(&self) -> PathBuf {
        self.root.path().join("policy.json")
    }

    fn permit(record: &McpCatalogRecord, url: &str) -> Value {
        json!({
            "project_id": "project", "app_id": AppId::Claude, "record_id": record.record.id,
            "connection_revision": record.connection_revision,
            "endpoint_sha256": hex::encode(Sha256::digest(reqwest_012::Url::parse(url).unwrap().as_str()))
        })
    }

    fn policy(&self, permits: &[Value]) -> anyhow::Result<()> {
        let grant =
            json!({"platform_id":"platform", "company_id":"company", "host_id":"local-fixture"});
        let mut mcp_grant = grant.clone();
        mcp_grant["mcp_probe"] = json!(permits);
        let policy = json!({
            "principals": [
                {"id":self.owner.principal,"token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[mcp_grant]},
                {"id":"fixture-other","token_sha256":hex::encode(Sha256::digest(OTHER_TOKEN)),"grants":[grant]}
            ],
            "hosts":[{"id":"local-fixture","online":true,"binding":"local","home":self.root.path(),
                "applications":self.root.path(),"search_paths":[],"credential_root":self.root.path()}]
        });
        let mut file = tempfile::NamedTempFile::new_in(self.root.path())?;
        file.as_file().set_permissions(std::fs::Permissions::from_mode(0o600))?;
        serde_json::to_writer(&mut file, &policy)?;
        file.flush()?;
        file.persist(self.policy_path())?;
        Ok(())
    }

    fn create(&self, name: &str, url: &str, sse: bool) -> anyhow::Result<McpCatalogRecord> {
        let mut draft = http_draft(name, url);
        if sse {
            draft.connection =
                McpConnectionDefinition::Sse { url: url.into(), headers: Default::default() };
        }
        Ok(run_sync_database(|| Ok(self.catalog.create(&self.catalog_scope(), draft)))?.unwrap())
    }

    fn start(
        &self,
        token: &'static str,
        scope: DiagnosticScope,
        records: &[&McpCatalogRecord],
    ) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
        let headers = HeaderMap::from_iter([(
            axum::http::HeaderName::from_static("x-agent-apps-token"),
            HeaderValue::from_static(token),
        )]);
        start_with_source(
            &self.jobs,
            self.db.clone(),
            Some(self.policy_path()),
            &headers,
            scope,
            records.iter().map(|record| record.record.id.clone()).collect::<Vec<McpRecordId>>(),
            DiagnosticEvidenceSource::ControlledFixture,
        )
    }

    async fn finish(&self, job: &DiagnosticJobIdentity) -> anyhow::Result<DiagnosticJobSnapshot> {
        Ok(timeout(Duration::from_secs(10), async {
            loop {
                let snapshot = self.jobs.snapshot(&self.owner, &job.task_id).unwrap();
                if !matches!(
                    snapshot.state,
                    DiagnosticJobState::Queued {}
                        | DiagnosticJobState::Running {}
                        | DiagnosticJobState::Cancelling {}
                ) {
                    return snapshot;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await?)
    }
}

#[derive(Clone)]
struct Server {
    methods: Arc<parking_lot::Mutex<Vec<String>>>,
    endpoint: String,
    events: broadcast::Sender<String>,
    hold_initialize: Option<(Arc<Notify>, Arc<Notify>)>,
    redirect: Option<String>,
    stateful: bool,
}

impl Server {
    fn new(endpoint: &str) -> Self {
        let (events, _) = broadcast::channel(8);
        Self {
            methods: Default::default(),
            endpoint: endpoint.into(),
            events,
            hold_initialize: None,
            redirect: None,
            stateful: false,
        }
    }

    fn serve(&self, listener: tokio::net::TcpListener) -> tokio::task::JoinHandle<()> {
        let routes = Router::new()
            .route("/http", post(http).delete(delete_session))
            .route("/sse", get(sse))
            .route("/messages", post(sse_post))
            .with_state(self.clone());
        tokio::spawn(async move { axum::serve(listener, routes).await.unwrap() })
    }

    async fn reply(&self, request: Value) -> Option<Value> {
        let method = request["method"].as_str().unwrap_or_default();
        self.methods.lock().push(method.to_owned());
        let result = match method {
            "initialize" => {
                if let Some((started, release)) = &self.hold_initialize {
                    started.notify_one();
                    release.notified().await;
                }
                json!({
                    "protocolVersion": "2025-03-26", "capabilities": {"tools": {}},
                    "serverInfo": {"name": "fixture", "version": "1"},
                })
            }
            "tools/list" => json!({"tools": []}),
            _ => return None,
        };
        Some(json!({"jsonrpc": "2.0", "id": request["id"], "result": result}))
    }
}

async fn http(State(server): State<Server>, Json(request): Json<Value>) -> Response {
    if let Some(destination) = &server.redirect {
        return (StatusCode::TEMPORARY_REDIRECT, [("location", destination.as_str())])
            .into_response();
    }
    let initialize = request["method"] == "initialize";
    match server.reply(request).await {
        Some(response) if initialize && server.stateful => {
            ([("mcp-session-id", "fixture-session")], Json(response)).into_response()
        }
        Some(response) => Json(response).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn delete_session(State(server): State<Server>, headers: HeaderMap) -> StatusCode {
    assert_eq!(headers.get("mcp-session-id").unwrap(), "fixture-session");
    server.methods.lock().push("DELETE".into());
    StatusCode::NO_CONTENT
}

async fn sse(State(server): State<Server>) -> Response {
    let events = server.events.subscribe();
    let first = Event::default().event("endpoint").data(server.endpoint);
    let messages = stream::unfold(events, |mut events| async {
        let payload = events.recv().await.ok()?;
        Some((
            Ok::<_, std::convert::Infallible>(Event::default().event("message").data(payload)),
            events,
        ))
    });
    Sse::new(stream::once(async { Ok(first) }).chain(messages)).into_response()
}

async fn sse_post(State(server): State<Server>, Json(request): Json<Value>) -> StatusCode {
    if let Some(reply) = server.reply(request).await {
        server.events.send(reply.to_string()).unwrap();
    }
    StatusCode::ACCEPTED
}

fn report(
    snapshot: &DiagnosticJobSnapshot,
) -> &vw_api_types::agent_apps_diagnostics::DiagnosticReport<
    vw_api_types::agent_apps_diagnostics::McpHandshakeEvidence,
> {
    let [DiagnosticResult::Mcp { report, .. }] = snapshot.results.as_deref().unwrap() else {
        panic!("expected one handshake result");
    };
    report
}

#[test]
fn diagnostic_mcp_http_handshake_is_scoped() -> anyhow::Result<()> {
    with_database(|database| {
        let fixture = Fixture::new(database)?;
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let base = format!("http://{}", listener.local_addr()?);
            let mut server = Server::new("/messages");
            server.stateful = true;
            let task = server.serve(listener);
            let http_url = format!("{base}/http?fixture=private-query");
            let sse_url = format!("{base}/sse");
            let http = fixture.create("http-fixture", &http_url, false)?;
            let sse = fixture.create("sse-fixture", &sse_url, true)?;
            let ungranted = fixture.create("ungranted", &format!("{base}/http"), false)?;

            let scope = fixture.owner.scope.clone();
            fixture.policy(&[Fixture::permit(&http, &format!("{base}/http"))])?;
            assert_eq!(fixture.start(TOKEN, scope.clone(), &[&http]), Err(McpHandshakeStartError::Record(McpFailureCode::PermissionDenied)));
            fixture.policy(&[Fixture::permit(&http, &http_url), Fixture::permit(&sse, &sse_url)])?;
            let forbidden = McpHandshakeStartError::Access(StatusCode::FORBIDDEN);
            assert_eq!(fixture.start(OTHER_TOKEN, scope.clone(), &[&http]), Err(forbidden));
            for wrong_scope in [
                DiagnosticScope { app_id: AppId::Codex, ..scope.clone() },
                DiagnosticScope { target_host_id: "foreign".into(), ..scope.clone() },
                DiagnosticScope { project: ProjectScope::new(scope.project.tenant_scope().clone(), "foreign".into())?, ..scope.clone() },
                DiagnosticScope { project: ProjectScope::new(TenantScope::new("platform", "foreign")?, "project".into())?, ..scope.clone() },
            ] {
                assert_eq!(fixture.start(TOKEN, wrong_scope, &[&http]), Err(McpHandshakeStartError::Access(StatusCode::FORBIDDEN)));
            }
            assert_eq!(fixture.start(TOKEN, scope.clone(), &[&http, &ungranted]), Err(McpHandshakeStartError::Access(StatusCode::FORBIDDEN)));
            assert!(server.methods.lock().is_empty());

            for (record, transport) in [(&http, McpTransport::StreamableHttp), (&sse, McpTransport::Sse)] {
                let job = fixture.start(TOKEN, scope.clone(), &[record]).unwrap();
                let result = fixture.finish(&job).await?;
                assert_eq!(result.state, DiagnosticJobState::Succeeded {});
                assert!(matches!(report(&result).outcome, DiagnosticOutcome::Succeeded { evidence } if evidence.transport == transport));
                assert_eq!(report(&result).source, DiagnosticEvidenceSource::ControlledFixture);
                let mut expected = vec!["initialize", "notifications/initialized"];
                if transport == McpTransport::StreamableHttp {
                    expected.push("DELETE");
                }
                assert_eq!(*server.methods.lock(), expected);
                server.methods.lock().clear();
                assert!(!serde_json::to_string(&result)?.contains("private-query"));
            }

            run_sync_database(|| Ok(fixture.catalog.delete(&fixture.catalog_scope(), &http.record)))?.unwrap();
            assert_eq!(fixture.start(TOKEN, scope.clone(), &[&http]), Err(McpHandshakeStartError::Record(McpFailureCode::NotFound)));
            let replacement = fixture.create("http-fixture", &format!("{base}/http"), false)?;
            assert_eq!(fixture.start(TOKEN, scope, &[&replacement]), Err(McpHandshakeStartError::Record(McpFailureCode::StaleRevision)));
            assert!(server.methods.lock().is_empty());
            task.abort();
            Ok(())
        })
    })
}

#[test]
fn diagnostic_mcp_http_handshake_is_scoped_cancels_active_connections_and_limits_batches()
-> anyhow::Result<()> {
    with_database(|database| {
        let fixture = Fixture::new(database)?;
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let url = format!("http://{}/http", listener.local_addr()?);
            let record = fixture.create("stalled", &url, false)?;
            fixture.policy(&[Fixture::permit(&record, &url)])?;
            let jobs = (0..4)
                .map(|_| fixture.start(TOKEN, fixture.owner.scope.clone(), &[&record]).unwrap())
                .collect::<Vec<_>>();
            let mut streams = Vec::new();
            for _ in 0..3 {
                let (mut socket, _) = timeout(Duration::from_secs(10), listener.accept()).await??;
                assert!(timeout(Duration::from_secs(5), socket.read(&mut [0; 4096])).await?? > 0);
                streams.push(socket);
            }
            assert!(timeout(Duration::from_millis(150), listener.accept()).await.is_err());
            for job in jobs.iter().rev() {
                fixture.jobs.cancel(&fixture.owner, &job.task_id).unwrap();
            }
            for job in &jobs {
                assert_eq!(fixture.finish(job).await?.state, DiagnosticJobState::Cancelled {});
            }
            for mut socket in streams {
                timeout(Duration::from_secs(5), async {
                    while socket.read(&mut [0; 4096]).await? != 0 {}
                    Ok::<_, std::io::Error>(())
                })
                .await??;
            }
            assert!(timeout(Duration::from_millis(150), listener.accept()).await.is_err());

            let timed = fixture.start(TOKEN, fixture.owner.scope.clone(), &[&record]).unwrap();
            let (mut socket, _) = timeout(Duration::from_secs(5), listener.accept()).await??;
            assert!(timeout(Duration::from_secs(5), socket.read(&mut [0; 4096])).await?? > 0);
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(29)).await;
            assert_eq!(fixture.jobs.snapshot(&fixture.owner, &timed.task_id).unwrap().state,
                DiagnosticJobState::Running {});
            tokio::time::advance(Duration::from_secs(1)).await;
            tokio::time::resume();
            assert!(matches!(report(&fixture.finish(&timed).await?).outcome,
                DiagnosticOutcome::Failed { error, .. } if error.code == DiagnosticFailureCode::Timeout));
            timeout(Duration::from_secs(5), async {
                while socket.read(&mut [0; 4096]).await? != 0 {}
                Ok::<_, std::io::Error>(())
            }).await??;
            Ok(())
        })
    })
}

#[test]
fn diagnostic_mcp_http_handshake_is_scoped_rejects_foreign_endpoints_and_revoked_results()
-> anyhow::Result<()> {
    with_database(|database| {
        let fixture = Fixture::new(database)?;
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(async {
            let foreign = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let foreign_url = format!("http://{}/foreign", foreign.local_addr()?);
            for (name, is_sse, destination) in [
                ("foreign-http", false, foreign_url.as_str()),
                ("same-origin-http", false, "/messages"),
                ("foreign-sse", true, foreign_url.as_str()),
            ] {
                let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
                let path = if is_sse { "sse" } else { "http" };
                let url = format!("http://{}/{path}?fixture=private-query", listener.local_addr()?);
                let mut server = Server::new(destination);
                if !is_sse {
                    server.redirect = Some(destination.to_owned());
                }
                let task = server.serve(listener);
                let record = fixture.create(name, &url, is_sse)?;
                fixture.policy(&[Fixture::permit(&record, &url)])?;
                let job = fixture.start(TOKEN, fixture.owner.scope.clone(), &[&record]).unwrap();
                let snapshot = fixture.finish(&job).await?;
                assert!(matches!(report(&snapshot).outcome, DiagnosticOutcome::Failed { error, .. } if error.code == DiagnosticFailureCode::Transport));
                assert!(!serde_json::to_string(&snapshot)?.contains("private-query"));
                assert!(server.methods.lock().is_empty());
                task.abort();
            }
            assert!(timeout(Duration::from_millis(150), foreign.accept()).await.is_err());

            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let url = format!("http://{}/http", listener.local_addr()?);
            let mut draft = http_draft("credential", &url);
            let reference = json!({"service":"xai_oauth","accountId":"account"});
            draft.connection = McpConnectionDefinition::StreamableHttp {
                url: url.clone(),
                headers: [("Authorization".into(), McpConfigValue::Credential {
                    reference: serde_json::from_value(reference.clone())?,
                })].into(),
            };
            let credential = run_sync_database(|| Ok(fixture.catalog.create(&fixture.catalog_scope(), draft)))?.unwrap();
            let mut permit = Fixture::permit(&credential, &url);
            permit["credentials"] = json!([reference]);
            fixture.policy(&[permit])?;
            assert_eq!(fixture.start(TOKEN, fixture.owner.scope.clone(), &[&credential]), Err(McpHandshakeStartError::Record(McpFailureCode::PermissionDenied)));
            assert!(timeout(Duration::from_millis(150), listener.accept()).await.is_err());

            let started = Arc::new(Notify::new());
            let release = Arc::new(Notify::new());
            let mut server = Server::new("/messages");
            server.hold_initialize = Some((started.clone(), release.clone()));
            let task = server.serve(listener);
            let record = fixture.create("revoked", &url, false)?;
            fixture.policy(&[Fixture::permit(&record, &url)])?;
            let job = fixture.start(TOKEN, fixture.owner.scope.clone(), &[&record]).unwrap();
            timeout(Duration::from_secs(5), started.notified()).await?;
            fixture.policy(&[])?;
            release.notify_one();
            assert!(matches!(report(&fixture.finish(&job).await?).outcome,
                DiagnosticOutcome::Failed { error, .. } if error.code == DiagnosticFailureCode::Unauthorized));
            assert_eq!(fixture.start(TOKEN, fixture.owner.scope.clone(), &[&record]), Err(McpHandshakeStartError::Access(StatusCode::FORBIDDEN)));
            task.abort();
            Ok(())
        })
    })
}
