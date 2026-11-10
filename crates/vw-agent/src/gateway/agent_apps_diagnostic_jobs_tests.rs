use std::{
    future::pending,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{Router, extract::State, http::StatusCode, routing::get};
use tokio::{
    net::TcpListener,
    sync::{Semaphore, mpsc, oneshot},
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode, DiagnosticJobSnapshot,
        DiagnosticJobState, DiagnosticOutcome, DiagnosticPhase, DiagnosticProbeKind,
        DiagnosticReport, DiagnosticResult, DiagnosticScope, DiagnosticTiming,
        ReachabilityEvidence,
    },
    id::{ProjectScope, TaskId},
};

use super::{DiagnosticJobError, DiagnosticJobOwner, DiagnosticJobs, DiagnosticProbeError};

fn owner() -> DiagnosticJobOwner {
    DiagnosticJobOwner {
        scope: DiagnosticScope {
            project: ProjectScope::for_default_tenant("project".into()).unwrap(),
            target_host_id: "execution-host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    }
}

fn result(http_status: u16) -> DiagnosticResult {
    DiagnosticResult::Reachability {
        provider_id: "provider".into(),
        report: DiagnosticReport {
            source: DiagnosticEvidenceSource::ControlledFixture,
            timings: vec![],
            outcome: DiagnosticOutcome::Succeeded {
                evidence: ReachabilityEvidence { http_status, slow: false },
            },
        },
    }
}

async fn observe(
    jobs: &DiagnosticJobs,
    owner: &DiagnosticJobOwner,
    id: &TaskId,
    ready: impl Fn(&DiagnosticJobSnapshot) -> bool,
) -> DiagnosticJobSnapshot {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        let snapshot = jobs.snapshot(owner, id).unwrap();
        if ready(&snapshot) {
            return snapshot;
        }
        tokio::task::yield_now().await;
    }
    panic!("diagnostic never reached expected state: {:?}", jobs.snapshot(owner, id));
}

#[tokio::test(start_paused = true)]
async fn diagnostic_jobs_progress_and_retention_are_bounded() {
    let directory = tempfile::tempdir().unwrap();
    let jobs = DiagnosticJobs::open(directory.path()).unwrap();
    assert!(matches!(DiagnosticJobs::open(directory.path()), Err(DiagnosticJobError::Unavailable)));
    let owner = owner();
    let (entered, mut entries) = mpsc::unbounded_channel();
    let gate = Arc::new(Semaphore::new(0));
    let app = Router::new()
        .route(
            "/",
            get(
                async |State((entered, gate)): State<(
                    mpsc::UnboundedSender<()>,
                    Arc<Semaphore>,
                )>| {
                    entered.send(()).unwrap();
                    gate.acquire().await.unwrap().forget();
                    StatusCode::UNAUTHORIZED
                },
            ),
        )
        .with_state((entered, gate.clone()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let url = format!("http://{address}/");
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let kind = DiagnosticProbeKind::Reachability;
    let submit = |count| {
        jobs.start(
            owner.clone(),
            kind,
            (0..count)
                .map(|_| {
                    let (client, url) = (client.clone(), url.clone());
                    move |_| async move {
                        let response = client.get(url).send().await.unwrap();
                        Ok(result(response.status().as_u16()))
                    }
                })
                .collect(),
        )
    };

    let first = submit(3).unwrap();
    let queued = jobs.snapshot(&owner, &first.task_id).expect("accepted task ID must resolve");
    assert_eq!(queued.job, first);
    assert_ne!(first.task_id.as_ref(), first.correlation_id);
    assert_eq!(queued.state, DiagnosticJobState::Queued {});
    assert_eq!(queued.progress.completed, 0);
    assert_eq!(queued.progress.total, Some(3));

    for _ in 0..3 {
        entries.recv().await.unwrap();
    }
    let waiting = submit(1).unwrap();
    tokio::task::yield_now().await;
    assert_eq!(
        jobs.snapshot(&owner, &waiting.task_id).unwrap().state,
        DiagnosticJobState::Queued {}
    );
    assert!(entries.try_recv().is_err(), "only three probes can enter across jobs");

    let mut foreign = owner.clone();
    foreign.scope.target_host_id = "different-host".into();
    assert_eq!(jobs.snapshot(&foreign, &first.task_id), Err(DiagnosticJobError::NotFound));
    foreign = owner.clone();
    foreign.principal = "different-member".into();
    assert_eq!(jobs.snapshot(&foreign, &first.task_id), Err(DiagnosticJobError::NotFound));

    gate.add_permits(1);
    let partial = observe(&jobs, &owner, &first.task_id, |job| job.progress.completed == 1).await;
    assert_eq!(partial.state, DiagnosticJobState::Running {});
    assert_eq!(partial.results.unwrap(), vec![result(401)]);
    entries.recv().await.unwrap();

    gate.add_permits(3);
    let completed =
        observe(&jobs, &owner, &first.task_id, |job| job.state == DiagnosticJobState::Succeeded {})
            .await;
    assert_eq!(completed.progress.completed, 3);
    assert_eq!(completed.results.unwrap(), vec![result(401); 3]);
    observe(&jobs, &owner, &waiting.task_id, |job| job.state == DiagnosticJobState::Succeeded {})
        .await;

    assert_eq!(submit(0), Err(DiagnosticJobError::InvalidInput));
    assert_eq!(submit(33), Err(DiagnosticJobError::InvalidInput));

    let failure = DiagnosticFailure {
        phase: DiagnosticPhase::ResponseHeaders,
        code: DiagnosticFailureCode::Transport,
    };
    let failed = jobs
        .start(
            owner.clone(),
            kind,
            vec![move |_| async move { Err(DiagnosticProbeError::Failed(failure)) }],
        )
        .unwrap();
    observe(&jobs, &owner, &failed.task_id, |job| {
        job.state == DiagnosticJobState::Failed { error: failure }
    })
    .await;

    let oversized = jobs
        .start(
            owner.clone(),
            kind,
            vec![|_| async {
                let DiagnosticResult::Reachability { provider_id, mut report } = result(401) else {
                    unreachable!()
                };
                report.timings =
                    vec![DiagnosticTiming { phase: DiagnosticPhase::Connect, elapsed_ms: 1 }; 9];
                Ok(DiagnosticResult::Reachability { provider_id, report })
            }],
        )
        .unwrap();
    let oversized = observe(&jobs, &owner, &oversized.task_id, |job| {
        matches!(job.state, DiagnosticJobState::Failed { .. })
    })
    .await;
    assert_eq!(oversized.progress.completed, 1);
    assert_eq!(oversized.results, Some(vec![]));

    let cancelled = jobs
        .start(owner.clone(), kind, vec![|_| async { Err(DiagnosticProbeError::Cancelled) }])
        .unwrap();
    observe(&jobs, &owner, &cancelled.task_id, |job| job.state == DiagnosticJobState::Cancelled {})
        .await;

    let metadata = std::fs::read_to_string(directory.path().join("diagnostics.json")).unwrap();
    assert!(metadata.contains(&first.correlation_id));
    assert!(!metadata.contains("results"));
    assert!(!metadata.contains("httpStatus"));

    tokio::time::advance(Duration::from_secs(600)).await;
    assert_eq!(jobs.snapshot(&owner, &first.task_id), Err(DiagnosticJobError::NotFound));

    let mut active = Vec::new();
    for _ in 0..128 {
        active.push(submit(1).unwrap());
    }
    assert_eq!(submit(1), Err(DiagnosticJobError::CapacityExceeded));
    assert_eq!(
        jobs.snapshot(&owner, &active[0].task_id).unwrap().state,
        DiagnosticJobState::Queued {}
    );
    assert!(entries.try_recv().is_err(), "rejected submission cannot execute");
    gate.add_permits(active.len());
    for job in &active {
        observe(&jobs, &owner, &job.task_id, |snapshot| {
            snapshot.state == DiagnosticJobState::Succeeded {}
        })
        .await;
    }
    for _ in 0..active.len() {
        entries.recv().await.unwrap();
    }
    assert!(entries.try_recv().is_err(), "exactly the accepted probes sent HTTP requests");
    let replacement = submit(1).unwrap();
    assert_eq!(
        active
            .iter()
            .filter(|job| jobs.snapshot(&owner, &job.task_id) == Err(DiagnosticJobError::NotFound))
            .count(),
        1
    );
    gate.add_permits(1);
    observe(&jobs, &owner, &replacement.task_id, |job| {
        job.state == DiagnosticJobState::Succeeded {}
    })
    .await;
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());

    let restart_directory = tempfile::tempdir().unwrap();
    let (task, owner, restart_directory) = tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let task = runtime.block_on(async {
            let jobs = DiagnosticJobs::open(restart_directory.path()).unwrap();
            let (started, observed) = oneshot::channel();
            let task = jobs
                .start(
                    owner.clone(),
                    kind,
                    vec![move |_| async move {
                        started.send(()).unwrap();
                        pending::<Result<DiagnosticResult, DiagnosticProbeError>>().await
                    }],
                )
                .unwrap();
            observed.await.unwrap();
            assert_eq!(
                jobs.snapshot(&owner, &task.task_id).unwrap().state,
                DiagnosticJobState::Running {}
            );
            task
        });
        drop(runtime);
        (task, owner, restart_directory)
    })
    .await
    .unwrap();
    let recovered = DiagnosticJobs::open(restart_directory.path()).unwrap();
    let interrupted = recovered.snapshot(&owner, &task.task_id).unwrap();
    assert_eq!(interrupted.job, task);
    assert_eq!(interrupted.state, DiagnosticJobState::Interrupted {});
    assert!(interrupted.results.is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn diagnostic_cancel_is_terminal_and_releases_capacity() {
    use tokio::{
        io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
        process::Command,
        time::timeout,
    };
    use tokio_util::sync::CancellationToken;

    let directory = tempfile::tempdir().unwrap();
    let jobs = DiagnosticJobs::open(directory.path()).unwrap();
    let owner = owner();
    let kind = DiagnosticProbeKind::Reachability;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (entered, mut requests) = mpsc::unbounded_channel();
    let (closed, mut disconnects) = mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let (entered, closed) = (entered.clone(), closed.clone());
            connections.spawn(async move {
                let mut socket = BufReader::new(socket);
                let mut line = String::new();
                socket.read_line(&mut line).await.unwrap();
                let path = line.split_whitespace().nth(1).unwrap().to_owned();
                loop {
                    line.clear();
                    assert_ne!(socket.read_line(&mut line).await.unwrap(), 0);
                    if line == "\r\n" {
                        break;
                    }
                }
                entered.send(path.clone()).unwrap();
                if path == "/active" {
                    assert_eq!(
                        socket.read_u8().await.unwrap_err().kind(),
                        std::io::ErrorKind::UnexpectedEof
                    );
                    closed.send(()).unwrap();
                } else {
                    socket
                        .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                        .await
                        .unwrap();
                }
            });
        }
    });

    let client = reqwest::Client::builder().no_proxy().http1_only().build().unwrap();
    let cleanup = Arc::new(Semaphore::new(0));
    let (reaped, mut exits) = mpsc::unbounded_channel();
    let submit = |path: &str, count| {
        jobs.start(
            owner.clone(),
            kind,
            (0..count)
                .map(|_| {
                    let (client, url) = (client.clone(), format!("{url}{path}"));
                    let (cleanup, reaped) = (cleanup.clone(), reaped.clone());
                    let holds_resources = path == "/active";
                    move |cancel: CancellationToken| async move {
                        if !holds_resources {
                            let response = client.get(url).send().await.unwrap();
                            return Ok(result(response.status().as_u16()));
                        }
                        let mut child = Command::new("/bin/sleep")
                            .arg("60")
                            .kill_on_drop(true)
                            .spawn()
                            .unwrap();
                        tokio::select! {
                            () = cancel.cancelled() => {}
                            _ = client.get(url).send() => panic!("fixture should wait for cancellation"),
                        }
                        assert!(child.try_wait().unwrap().is_none());
                        cleanup.acquire().await.unwrap().forget();
                        child.kill().await.unwrap();
                        reaped.send(child.wait().await.unwrap()).unwrap();
                        // A successful result racing with cancellation must be discarded.
                        Ok(result(204))
                    }
                })
                .collect(),
        )
        .unwrap()
    };

    let active = submit("/active", 4);
    for _ in 0..3 {
        assert_eq!(
            timeout(Duration::from_secs(10), requests.recv()).await.unwrap().unwrap(),
            "/active"
        );
    }
    let queued = submit("/queued", 1);
    tokio::task::yield_now().await;
    assert_eq!(
        jobs.snapshot(&owner, &queued.task_id).unwrap().state,
        DiagnosticJobState::Queued {}
    );
    let mut foreign = owner.clone();
    foreign.scope.target_host_id = "other-host".into();
    assert_eq!(jobs.cancel(&foreign, &active.task_id), Err(DiagnosticJobError::NotFound));
    foreign = owner.clone();
    foreign.principal = "other-member".into();
    assert_eq!(jobs.cancel(&foreign, &queued.task_id), Err(DiagnosticJobError::NotFound));

    assert_ne!(
        jobs.cancel(&owner, &queued.task_id).unwrap().state,
        DiagnosticJobState::Queued {},
        "cancelling a queued job must stop its permit wait"
    );
    let queued_done = observe(&jobs, &owner, &queued.task_id, |job| {
        job.state == DiagnosticJobState::Cancelled {}
    })
    .await;
    assert_eq!(queued_done.progress.completed, 0);
    assert_eq!(jobs.cancel(&owner, &queued.task_id).unwrap(), queued_done);

    let replacement = submit("/replacement", 1);
    assert_eq!(
        jobs.cancel(&owner, &active.task_id).unwrap().state,
        DiagnosticJobState::Cancelling {}
    );
    for _ in 0..3 {
        timeout(Duration::from_secs(10), disconnects.recv()).await.unwrap().unwrap();
    }
    assert_eq!(
        jobs.cancel(&owner, &active.task_id).unwrap().state,
        DiagnosticJobState::Cancelling {}
    );
    assert_eq!(
        jobs.snapshot(&owner, &replacement.task_id).unwrap().state,
        DiagnosticJobState::Queued {}
    );
    assert!(requests.try_recv().is_err(), "no queued request can run before cleanup");
    assert!(exits.try_recv().is_err(), "children are still being held for cleanup");

    cleanup.add_permits(3);
    for _ in 0..3 {
        assert!(!timeout(Duration::from_secs(10), exits.recv()).await.unwrap().unwrap().success());
    }
    let cancelled = observe(&jobs, &owner, &active.task_id, |job| {
        job.state == DiagnosticJobState::Cancelled {}
    })
    .await;
    assert_eq!(cancelled.progress.completed, 0);
    assert_eq!(cancelled.results, Some(vec![]));
    assert_eq!(jobs.cancel(&owner, &active.task_id).unwrap(), cancelled);

    let finished = observe(&jobs, &owner, &replacement.task_id, |job| {
        job.state == DiagnosticJobState::Succeeded {}
    })
    .await;
    assert_eq!(finished.results, Some(vec![result(204)]));
    assert_eq!(jobs.cancel(&owner, &replacement.task_id).unwrap(), finished);
    assert_eq!(requests.try_recv().unwrap(), "/replacement");
    assert!(requests.try_recv().is_err(), "cancelled queued probes never send HTTP");

    let checkpoint = std::fs::read(directory.path().join("diagnostics.json")).unwrap();
    let persisted: Vec<serde_json::Value> = serde_json::from_slice(&checkpoint).unwrap();
    assert!(persisted.iter().any(|job| {
        job["identity"]["taskId"] == active.task_id.as_ref()
            && job["state"]["status"] == "cancelled"
            && job["progress"]["completed"] == 0
    }));
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}
