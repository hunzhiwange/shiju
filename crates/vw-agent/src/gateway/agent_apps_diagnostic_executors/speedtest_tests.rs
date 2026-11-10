use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::{Instant, sleep, timeout},
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticEndpointSource, DiagnosticJobSnapshot, DiagnosticJobState, DiagnosticScope,
        SpeedEvidence,
    },
    id::{ProjectScope, TaskId},
};

use super::*;

const WAIT: Duration = Duration::from_secs(5);

fn target(id: &str, url: String) -> (DiagnosticEndpointId, EndpointCandidate) {
    (
        DiagnosticEndpointId(id.into()),
        EndpointCandidate { url, sources: vec![DiagnosticEndpointSource::SavedConfig] },
    )
}

async fn observe(
    jobs: &DiagnosticJobs,
    owner: &DiagnosticJobOwner,
    id: &TaskId,
    ready: impl Fn(&DiagnosticJobSnapshot) -> bool,
) -> DiagnosticJobSnapshot {
    let start = Instant::now();
    loop {
        let snapshot = jobs.snapshot(owner, id).unwrap();
        if ready(&snapshot) {
            return snapshot;
        }
        assert!(start.elapsed() < WAIT, "unexpected job snapshot: {snapshot:?}");
        sleep(Duration::from_millis(5)).await;
    }
}

fn report<'a>(result: &'a DiagnosticResult, id: &str) -> &'a DiagnosticReport<SpeedEvidence> {
    let DiagnosticResult::Speed { endpoint_id, report } = result else {
        panic!("expected a speed result: {result:?}");
    };
    assert_eq!(endpoint_id.0, id);
    assert_eq!(report.source, DiagnosticEvidenceSource::ControlledFixture);
    report
}

async fn get(listener: &TcpListener, path: &str) -> TcpStream {
    timeout(WAIT, async {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(&mut socket);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, format!("GET {path} HTTP/1.1\r\n"));
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            assert!(!line.to_ascii_lowercase().starts_with("authorization:"));
        }
        socket
    })
    .await
    .expect("expected an HTTP GET")
}

async fn write_headers(socket: &mut TcpStream, status: &str, length: u32) {
    socket
        .write_all(
            format!("HTTP/1.1 {status}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn diagnostic_speedtest_reports_each_endpoint() {
    let directory = tempfile::tempdir().unwrap();
    let jobs = DiagnosticJobs::open(directory.path()).unwrap();
    let owner = DiagnosticJobOwner {
        scope: DiagnosticScope {
            project: ProjectScope::for_default_tenant("project".into()).unwrap(),
            target_host_id: "execution-host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    };
    let submit = |targets| {
        start_with_source(
            &jobs,
            owner.clone(),
            targets,
            None,
            DiagnosticEvidenceSource::ControlledFixture,
        )
    };
    let slow = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fast = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let unreachable = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let slow_url = format!("http://{}/full/path", slow.local_addr().unwrap());
    let fast_url = format!("http://{}/fast", fast.local_addr().unwrap());
    let unreachable_url = format!("http://{}/unreachable", unreachable.local_addr().unwrap());
    let fast_fixture = tokio::spawn(async move {
        let mut warmup = get(&fast, "/fast").await;
        warmup
            .write_all(b"HTTP/1.1 302 Found\r\nLocation: /redirected\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        drop(warmup);
        let mut measured = get(&fast, "/fast").await;
        write_headers(&mut measured, "500 Internal Server Error", 16).await;
        measured // Leave the response body unfinished.
    });
    let error_fixture = tokio::spawn(async move {
        write_headers(&mut get(&unreachable, "/unreachable").await, "200 OK", 0).await;
        drop(get(&unreachable, "/unreachable").await);
    });
    let id = submit(vec![
        target("slow", slow_url.clone()),
        target("fast", fast_url),
        target("unreachable", unreachable_url),
        target("invalid", "file:///invalid".into()),
    ])
    .unwrap()
    .task_id;

    let partial = observe(&jobs, &owner, &id, |job| {
        job.results.as_ref().is_some_and(|results| {
            results.iter().any(|result| {
                matches!(
                    result,
                    DiagnosticResult::Speed { endpoint_id, .. } if endpoint_id.0 == "fast"
                )
            })
        })
    })
    .await;
    let partial_results = partial.results.unwrap();
    let fast_result = partial_results
        .iter()
        .find(|result| {
            matches!(
                result,
                DiagnosticResult::Speed { endpoint_id, .. } if endpoint_id.0 == "fast"
            )
        })
        .unwrap();
    let fast = report(fast_result, "fast");
    let DiagnosticOutcome::Succeeded { evidence } = fast.outcome else {
        panic!("timed GET should report HTTP 500 with unfinished body: {fast:?}");
    };
    assert_eq!(evidence.http_status, 500);
    assert_eq!(partial.progress.total, Some(4));
    assert!((1..4).contains(&partial.progress.completed));
    let fast_body = fast_fixture.await.unwrap();

    write_headers(&mut get(&slow, "/full/path").await, "204 No Content", 0).await;
    let mut measured = get(&slow, "/full/path").await;
    sleep(Duration::from_millis(40)).await;
    write_headers(&mut measured, "400 Bad Request", 0).await;
    error_fixture.await.unwrap();

    let completed =
        observe(&jobs, &owner, &id, |job| job.state == DiagnosticJobState::Succeeded {}).await;
    assert_eq!(completed.progress.completed, 4);
    let results = completed.results.unwrap();
    let slow_report = report(&results[0], "slow");
    let DiagnosticOutcome::Succeeded { evidence } = slow_report.outcome else {
        panic!("timed GET should report HTTP 400: {slow_report:?}");
    };
    assert_eq!(evidence.http_status, 400);
    assert!(evidence.latency_ms >= 35);
    assert_eq!(slow_report.timings.len(), 2);
    assert_eq!(slow_report.timings[0].phase, DiagnosticPhase::Warmup);
    assert_eq!(slow_report.timings[1].phase, DiagnosticPhase::ResponseHeaders);
    assert_eq!(slow_report.timings[1].elapsed_ms, evidence.latency_ms);
    assert_eq!(report(&results[1], "fast"), fast);
    for (index, id, phase, code) in [
        (2, "unreachable", DiagnosticPhase::ResponseHeaders, DiagnosticFailureCode::Transport),
        (3, "invalid", DiagnosticPhase::Connect, DiagnosticFailureCode::Unsupported),
    ] {
        assert_eq!(
            report(&results[index], id).outcome,
            DiagnosticOutcome::Failed {
                error: DiagnosticFailure { phase, code },
                http_status: None
            }
        );
    }
    drop(fast_body);

    assert_eq!(attempt_timeout(None), Duration::from_secs(8));
    assert_eq!(attempt_timeout(Some(0)), Duration::from_secs(2));
    assert_eq!(attempt_timeout(Some(60)), Duration::from_secs(30));

    let timed_out = start_with_source(
        &jobs,
        owner.clone(),
        vec![target("timeout", slow_url.clone())],
        Some(0),
        DiagnosticEvidenceSource::ControlledFixture,
    )
    .unwrap();
    let warmup = get(&slow, "/full/path").await;
    let measured = get(&slow, "/full/path").await;
    let snapshot = observe(&jobs, &owner, &timed_out.task_id, |job| {
        job.state == DiagnosticJobState::Succeeded {}
    })
    .await;
    let results = snapshot.results.unwrap();
    let timeout_report = report(&results[0], "timeout");
    assert_eq!(
        timeout_report.outcome,
        DiagnosticOutcome::Failed {
            error: DiagnosticFailure {
                phase: DiagnosticPhase::ResponseHeaders,
                code: DiagnosticFailureCode::Timeout,
            },
            http_status: None,
        }
    );
    assert!(timeout_report.timings.iter().all(|timing| timing.elapsed_ms >= 1900));
    drop((warmup, measured));

    assert_eq!(submit(Vec::new()), Err(DiagnosticJobError::InvalidInput));
    assert_eq!(
        submit((0..33).map(|i| target(&i.to_string(), slow_url.clone())).collect()),
        Err(DiagnosticJobError::InvalidInput)
    );
    let id =
        submit((0..4).map(|i| target(&i.to_string(), slow_url.clone())).collect()).unwrap().task_id;
    let mut active = Vec::new();
    for _ in 0..3 {
        active.push(get(&slow, "/full/path").await);
    }
    assert!(timeout(Duration::from_millis(50), slow.accept()).await.is_err());
    let mut warmup = active.pop().unwrap();
    write_headers(&mut warmup, "200 OK", 0).await;
    drop(warmup);
    active.push(get(&slow, "/full/path").await);
    jobs.cancel(&owner, &id).unwrap();
    let cancelled =
        observe(&jobs, &owner, &id, |job| job.state == DiagnosticJobState::Cancelled {}).await;
    assert_eq!(cancelled.progress.completed, 0);
    assert!(cancelled.results.unwrap().is_empty());
    for mut socket in active {
        let close = timeout(WAIT, socket.read(&mut [0u8; 1])).await.unwrap();
        assert!(
            matches!(close, Ok(0))
                || close.is_err_and(|error| error.kind() == std::io::ErrorKind::ConnectionReset)
        );
    }
    assert!(timeout(Duration::from_millis(50), slow.accept()).await.is_err());
}
