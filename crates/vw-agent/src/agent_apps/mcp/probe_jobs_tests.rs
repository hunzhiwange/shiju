use std::{os::unix::fs::DirBuilderExt, sync::Arc, time::Duration};

use axum::{Json, extract::State};
use serde_json::{Value, json};
use tokio::{io::AsyncReadExt, sync::Notify, time::timeout};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticJobIdentity, DiagnosticJobSnapshot, DiagnosticJobState, DiagnosticScope,
    },
    id::ProjectScope,
    mcp_management::{
        McpCatalogRecord, McpCatalogScope, McpConnectionDefinition, McpConnectionOutcome,
        McpFailureCode,
    },
};

use super::{
    McpCatalogStore,
    catalog_create_tests::{http_draft, with_database},
    probe_jobs::{McpProbeBatch, McpProbeTarget, start_connection_checks},
};
use crate::{
    agent_apps::auth_accounts::AuthAccountStore,
    gateway::agent_apps_diagnostic_jobs::{DiagnosticJobError, DiagnosticJobOwner, DiagnosticJobs},
    global_database::{GlobalDatabase, run_sync_database},
};

#[test]
fn cc_mcp_t31_in_flight_connection_edit_discards_old_success() -> anyhow::Result<()> {
    with_database(|database| {
        let workspace = tempfile::tempdir()?;
        let account_dir = workspace.path().join("accounts");
        std::fs::DirBuilder::new().mode(0o700).create(&account_dir)?;
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let accounts = AuthAccountStore::new(database, &account_dir).map_err(anyhow::Error::msg)?;
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let url = format!("http://{}/mcp", listener.local_addr()?);
            let ready = std::sync::Arc::new(Notify::new());
            let release = std::sync::Arc::new(Notify::new());
            let router = axum::Router::new()
                .route("/mcp", axum::routing::post(check_http))
                .with_state((ready.clone(), release.clone()));
            let server = tokio::spawn(async move { axum::serve(listener, router).await });
            let saved =
                run_sync_database(|| Ok(catalog.create(&scope, http_draft("fixture", &url))))?
                    .unwrap();

            let check = catalog.check_connection(
                &scope,
                &saved.record,
                "local-fixture",
                workspace.path(),
                &accounts,
                |_, _| None,
            );
            let edit = async {
                ready.notified().await;
                run_sync_database(|| {
                    Ok(catalog.update_connection(
                        &scope,
                        &saved.record,
                        McpConnectionDefinition::StreamableHttp {
                            url: format!("{url}?updated=1"),
                            headers: Default::default(),
                        },
                    ))
                })?
                .unwrap();
                release.notify_one();
                Ok::<_, anyhow::Error>(())
            };
            let (outcome, edit) =
                timeout(Duration::from_secs(10), async { tokio::join!(check, edit) }).await?;
            edit?;
            server.abort();
            assert_eq!(
                outcome,
                McpConnectionOutcome::Failed { code: McpFailureCode::StaleRevision },
                "an in-flight success must not apply to an edited connection",
            );
            Ok(())
        })
    })
}

type HttpSignals = (std::sync::Arc<Notify>, std::sync::Arc<Notify>);

async fn check_http(
    State((ready, release)): State<HttpSignals>,
    Json(request): Json<Value>,
) -> axum::response::Response {
    use axum::{http::StatusCode, response::IntoResponse as _};
    let id = &request["id"];
    let result = match request["method"].as_str() {
        Some("initialize") => json!({
            "protocolVersion": "2025-03-26", "capabilities": {"tools": {}},
            "serverInfo": {"name": "fixture", "version": "1"},
        }),
        Some("notifications/initialized") => return StatusCode::ACCEPTED.into_response(),
        Some("tools/list") => {
            ready.notify_one();
            release.notified().await;
            json!({"tools": []})
        }
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    Json(json!({"jsonrpc": "2.0", "id": id, "result": result})).into_response()
}

#[test]
fn cc_mcp_t31_cancel_stops_only_owned_active_and_queued_probes() -> anyhow::Result<()> {
    with_database(|database| {
        let fixture = BatchFixture::new(database)?;
        let survivor = fixture.stdio("survivor", "discovery");
        let replacement = fixture.stdio("replacement", "complete");
        let first = fixture.stdio("initializing", "handshake");
        let second = fixture.stdio("discovering", "discovery");
        let queued = fixture.stdio("queued", "handshake");
        let never_registered = fixture.stdio("before-worker", "handshake");

        fixture.runtime()?.block_on(async {
            let survivor_job = fixture.start(1, [&survivor])?;
            fixture.wait_for_stdio(&survivor).await?;
            let cancelled_job = fixture.start(2, [&first, &second, &queued])?;
            let before_worker_job = fixture.start(3, [&never_registered])?;
            fixture.jobs.cancel(&fixture.owner, &before_worker_job.task_id).unwrap();
            let (first_pid, first_child) = fixture.wait_for_stdio(&first).await?;
            let (second_pid, second_child) = fixture.wait_for_stdio(&second).await?;
            assert!(!fixture.marker(&queued, "pid").exists());
            assert!(!fixture.marker(&never_registered, "pid").exists());

            let mut stranger = fixture.owner.clone();
            stranger.principal = "oidc:unrelated".into();
            assert_eq!(
                fixture.jobs.cancel(&stranger, &cancelled_job.task_id),
                Err(DiagnosticJobError::NotFound)
            );
            assert_eq!(fixture.snapshot(&cancelled_job).state, DiagnosticJobState::Running {});

            fixture.jobs.cancel(&fixture.owner, &cancelled_job.task_id).unwrap();
            let cancelled = fixture.terminal(&cancelled_job).await?;
            assert_eq!(cancelled.state, DiagnosticJobState::Cancelled {});
            assert_eq!(cancelled.progress.completed, 0);
            assert!(cancelled.results.unwrap().is_empty());
            assert_eq!(
                fixture.terminal(&before_worker_job).await?.state,
                DiagnosticJobState::Cancelled {}
            );
            assert!(!fixture.marker(&queued, "pid").exists());
            assert!(!fixture.marker(&never_registered, "pid").exists());
            assert!(!process_exists(first_pid), "cancelled handshake leader was not reaped");
            assert!(!process_exists(second_pid), "cancelled discovery leader was not reaped");
            wait_until(|| {
                (!process_exists(first_child) && !process_exists(second_child)).then_some(())
            })
            .await?;

            // Two cancelled probes must release their permits after process cleanup.
            let replacement_job = fixture.start(4, [&replacement])?;
            assert_eq!(
                fixture.terminal(&replacement_job).await?.state,
                DiagnosticJobState::Succeeded {}
            );
            let (survivor_pid, survivor_child) = fixture.wait_for_stdio(&survivor).await?;
            assert!(process_exists(survivor_pid) && process_exists(survivor_child));
            assert_eq!(fixture.snapshot(&survivor_job).state, DiagnosticJobState::Running {});
            fixture.jobs.cancel(&fixture.owner, &survivor_job.task_id).unwrap();
            assert_eq!(
                fixture.terminal(&survivor_job).await?.state,
                DiagnosticJobState::Cancelled {}
            );
            Ok(())
        })
    })
}

#[test]
fn cc_mcp_t31_only_current_task_generation_and_connection_revision_apply() -> anyhow::Result<()> {
    with_database(|database| {
        let fixture = BatchFixture::new(database)?;
        let record = fixture.stdio("results", "complete");
        fixture.runtime()?.block_on(async {
            let old_job = fixture.start(7, [&record])?;
            let old_result = fixture.terminal(&old_job).await?;
            assert_eq!(old_result.state, DiagnosticJobState::Succeeded {});
            let current =
                |snapshot: &DiagnosticJobSnapshot, job: &DiagnosticJobIdentity, generation| {
                    fixture
                        .catalog
                        .current_connection_check_items(
                            &fixture.scope,
                            snapshot,
                            &job.task_id,
                            generation,
                        )
                        .unwrap()
                };
            assert_eq!(
                current(&old_result, &old_job, 7)[0].outcome,
                McpConnectionOutcome::Connected { tool_count: Some(0) },
            );

            // Saving the same connection advances the record revision only.
            let same = run_sync_database(|| {
                Ok(fixture.catalog.update_connection(
                    &fixture.scope,
                    &record.record,
                    stdio_definition("results", "complete"),
                ))
            })?
            .unwrap();
            assert!(same.record.revision > record.record.revision);
            assert_eq!(current(&old_result, &old_job, 7).len(), 1);
            let new_job = fixture.start(8, [&same])?;
            let new_result = fixture.terminal(&new_job).await?;
            assert_eq!(current(&new_result, &new_job, 8).len(), 1);
            assert!(current(&old_result, &new_job, 7).is_empty());
            assert!(current(&old_result, &old_job, 8).is_empty());

            run_sync_database(|| {
                Ok(fixture.catalog.update_connection(
                    &fixture.scope,
                    &same.record,
                    stdio_definition("results", "discovery"),
                ))
            })?
            .unwrap();
            assert!(current(&new_result, &new_job, 8).is_empty());
            Ok(())
        })
    })
}

#[test]
fn diagnostic_mcp_stdio_cancellation_reaps_process_tree() -> anyhow::Result<()> {
    // Exercise the same authorized catalog path without an optional cancellation token:
    // timeout must retain ownership of the entire process group as well.
    with_database(|database| {
        let fixture = BatchFixture::new(database)?;
        let record = fixture.stdio("standalone-deadline", "handshake");
        fixture.runtime()?.block_on(async {
            let check = fixture.catalog.check_connection(
                &fixture.scope,
                &record.record,
                "local-fixture",
                fixture.workspace.path(),
                &fixture.accounts,
                |_, _| None,
            );
            let observe = async {
                let pids = fixture.wait_for_stdio(&record).await?;
                tokio::time::pause();
                tokio::time::advance(Duration::from_secs(31)).await;
                tokio::time::resume();
                Ok::<_, anyhow::Error>(pids)
            };
            let (outcome, pids) = tokio::join!(check, observe);
            let (pid, child) = pids?;
            assert_eq!(outcome, McpConnectionOutcome::Failed { code: McpFailureCode::TimedOut });
            let exited =
                wait_until(|| (!process_exists(pid) && !process_exists(child)).then_some(())).await;
            if exited.is_err() {
                // Clean up the deliberately exposed leak even in the red run.
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                    libc::kill(child, libc::SIGKILL);
                }
            }
            assert!(exited.is_ok(), "timed-out stdio probe left a live process tree");
            Ok(())
        })
    })?;
    cc_mcp_t31_cancel_stops_only_owned_active_and_queued_probes()
}

#[test]
fn cc_mcp_t31_timed_out_stdio_is_reaped_before_job_completion() -> anyhow::Result<()> {
    with_database(|database| {
        let fixture = BatchFixture::new(database)?;
        let record = fixture.stdio("deadline", "handshake");
        fixture.runtime()?.block_on(async {
            let job = fixture.start(1, [&record])?;
            let (pid, child) = fixture.wait_for_stdio(&record).await?;
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(31)).await;
            tokio::time::resume();
            let snapshot = fixture.terminal(&job).await?;
            assert_eq!(snapshot.state, DiagnosticJobState::Succeeded {});
            let results = fixture
                .catalog
                .current_connection_check_items(&fixture.scope, &snapshot, &job.task_id, 1)
                .unwrap();
            assert_eq!(
                results[0].outcome,
                McpConnectionOutcome::Failed { code: McpFailureCode::TimedOut }
            );
            assert!(!process_exists(pid));
            wait_until(|| (!process_exists(child)).then_some(())).await?;
            Ok(())
        })
    })
}

#[test]
fn cc_mcp_t31_cancel_drops_a_stalled_http_initialization_request() -> anyhow::Result<()> {
    with_database(|database| {
        let fixture = BatchFixture::new(database)?;
        fixture.runtime()?.block_on(async {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let url = format!("http://{}/mcp", listener.local_addr()?);
            let record = run_sync_database(|| {
                Ok(fixture.catalog.create(&fixture.scope, http_draft("stalled-http", &url)))
            })?
            .unwrap();
            let started = Arc::new(Notify::new());
            let ready = started.clone();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await?;
                let mut request = [0; 4096];
                anyhow::ensure!(stream.read(&mut request).await? > 0, "empty HTTP request");
                ready.notify_one();
                // A real HTTP connection whose server withholds the initialize response.
                while stream.read(&mut request).await? != 0 {}
                Ok::<_, anyhow::Error>(())
            });

            let job = fixture.start(1, [&record])?;
            timeout(Duration::from_secs(10), started.notified()).await?;
            fixture.jobs.cancel(&fixture.owner, &job.task_id).unwrap();
            assert_eq!(fixture.terminal(&job).await?.state, DiagnosticJobState::Cancelled {});
            timeout(Duration::from_secs(5), server).await.map_err(|_| {
                anyhow::anyhow!("cancelled HTTP initialization still has an open socket")
            })???;
            Ok(())
        })
    })
}

struct BatchFixture {
    catalog: Arc<McpCatalogStore>,
    accounts: Arc<AuthAccountStore>,
    workspace: tempfile::TempDir,
    jobs: DiagnosticJobs,
    owner: DiagnosticJobOwner,
    scope: McpCatalogScope,
}

impl BatchFixture {
    fn new(database: GlobalDatabase) -> anyhow::Result<Self> {
        let workspace = tempfile::tempdir()?;
        let private = workspace.path().join("private");
        std::fs::DirBuilder::new().mode(0o700).create(&private)?;
        let tenant = TenantScope::new("platform", "company")?;
        Ok(Self {
            catalog: Arc::new(McpCatalogStore::new(database.clone()).unwrap()),
            accounts: Arc::new(
                AuthAccountStore::new(database, &private).map_err(anyhow::Error::msg)?,
            ),
            workspace,
            jobs: DiagnosticJobs::open(&private).unwrap(),
            owner: DiagnosticJobOwner {
                scope: DiagnosticScope {
                    project: ProjectScope::new(tenant.clone(), "project".into())?,
                    target_host_id: "local-fixture".into(),
                    app_id: AppId::Codex,
                },
                principal: "oidc:fixture".into(),
            },
            scope: McpCatalogScope::Tenant { scope: tenant },
        })
    }

    fn runtime(&self) -> std::io::Result<tokio::runtime::Runtime> {
        tokio::runtime::Builder::new_current_thread().enable_all().build()
    }

    fn stdio(&self, id: &str, mode: &str) -> McpCatalogRecord {
        let mut draft = http_draft(id, "unused");
        draft.connection = stdio_definition(id, mode);
        self.catalog.create(&self.scope, draft).unwrap()
    }

    fn start<'a>(
        &self,
        generation: u64,
        records: impl IntoIterator<Item = &'a McpCatalogRecord>,
    ) -> anyhow::Result<DiagnosticJobIdentity> {
        let targets = records
            .into_iter()
            .map(|record| McpProbeTarget {
                id: record.record.id.clone(),
                connection_revision: record.connection_revision,
            })
            .collect();
        start_connection_checks(
            &self.jobs,
            self.owner.clone(),
            self.catalog.clone(),
            self.accounts.clone(),
            self.workspace.path().into(),
            McpProbeBatch { scope: self.scope.clone(), generation, targets },
            |_, _, _| None,
        )
        .map_err(|error| anyhow::anyhow!("could not start fixture batch: {error:?}"))
    }

    fn snapshot(&self, job: &DiagnosticJobIdentity) -> DiagnosticJobSnapshot {
        self.jobs.snapshot(&self.owner, &job.task_id).unwrap()
    }

    async fn terminal(&self, job: &DiagnosticJobIdentity) -> anyhow::Result<DiagnosticJobSnapshot> {
        wait_until(|| {
            let snapshot = self.snapshot(job);
            match snapshot.state {
                DiagnosticJobState::Queued {}
                | DiagnosticJobState::Running {}
                | DiagnosticJobState::Cancelling {} => None,
                _ => Some(snapshot),
            }
        })
        .await
    }

    fn marker(&self, record: &McpCatalogRecord, suffix: &str) -> std::path::PathBuf {
        self.workspace.path().join(format!("{}.{suffix}", record.record.id.0))
    }

    async fn wait_for_stdio(&self, record: &McpCatalogRecord) -> anyhow::Result<(i32, i32)> {
        wait_until(|| {
            if !self.marker(record, "ready").exists() {
                return None;
            }
            let read_pid = |suffix| {
                std::fs::read_to_string(self.marker(record, suffix)).ok()?.trim().parse().ok()
            };
            Some((read_pid("pid")?, read_pid("child")?))
        })
        .await
    }
}

async fn wait_until<T>(mut observe: impl FnMut() -> Option<T>) -> anyhow::Result<T> {
    Ok(timeout(Duration::from_secs(10), async {
        loop {
            if let Some(value) = observe() {
                return value;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?)
}

fn process_exists(pid: i32) -> bool {
    assert!(pid > 0);
    // SAFETY: signal 0 checks the fixture PID's existence without changing its state.
    unsafe { libc::kill(pid, 0) == 0 }
}

fn stdio_definition(id: &str, mode: &str) -> McpConnectionDefinition {
    McpConnectionDefinition::Stdio {
        command: "/bin/sh".into(),
        args: vec!["-c".into(), BATCH_STDIO.into(), "fixture".into(), id.into(), mode.into()],
        env: Default::default(),
        cwd: None,
    }
}

const BATCH_STDIO: &str = r#"
    printf '%s\n' "$$" > "$1.pid"
    stall() {
        /bin/sleep 120 &
        printf '%s\n' "$!" > "$1.child"
        : > "$1.ready"
        wait
    }
    while IFS= read -r line; do
        id=${line#*\"id\":}
        id=${id%%,*}
        id=${id%%\}*}
        case "$line" in
            *'"method":"initialize"'*)
                if [ "$2" = handshake ]; then stall "$1"; fi
                printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}}\n' "$id"
                ;;
            *'"method":"tools/list"'*)
                if [ "$2" = discovery ]; then stall "$1"; fi
                printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[]}}\n' "$id"
                ;;
        esac
    done
"#;
