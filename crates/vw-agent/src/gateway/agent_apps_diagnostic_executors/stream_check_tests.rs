use std::{os::unix::fs::DirBuilderExt, time::Duration};

use super::StreamProbe;
use crate::{
    agent_apps::auth_accounts::AuthAccountStore,
    app_providers::AppProviderStore,
    gateway::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticProbeError},
    global_database::GlobalDatabase,
};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::auth::copilot::CopilotAuth;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailureCode, DiagnosticOutcome, DiagnosticResult,
        DiagnosticScope,
    },
    app_providers::claude::{ClaudeConfig, ClaudeProfile},
    id::ProjectScope,
};

use crate::model_relay::stream_observer::tests::samples;

#[tokio::test]
async fn diagnostic_stream_check_requires_valid_completion() {
    let directory = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::new(directory.path().join("global.sqlite")).unwrap();
    let providers = AppProviderStore::new(database.clone()).unwrap();
    let credentials = directory.path().join("credentials");
    std::fs::DirBuilder::new().mode(0o700).create(&credentials).unwrap();
    let accounts = AuthAccountStore::new(database.clone(), &credentials).unwrap();
    let copilot = CopilotAuth::new().unwrap();
    let owner = DiagnosticJobOwner {
        scope: DiagnosticScope {
            project: ProjectScope::for_default_tenant("project".into()).unwrap(),
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    };
    for (format, path, valid) in samples() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        providers
            .claude_save_with_metadata(
                owner.scope.project.tenant_scope(),
                format,
                &ClaudeProfile {
                    name: "Fixture".into(),
                    model: Some("fixture".into()),
                    config: ClaudeConfig::ThirdParty {
                        base_url: format!("http://{}/v1", listener.local_addr().unwrap()),
                        api_key: "fixture-key".into(),
                        api_key_header: false,
                    },
                },
                None,
                true,
                Some(json!({"meta":{"apiFormat":format}})),
            )
            .unwrap();
        // One valid increment without the protocol's terminal event.
        let truncated = valid.rsplit_once("data:").unwrap().0.to_owned();
        let empty_output = valid.replace("\"OK\"", "\"\"");
        use DiagnosticFailureCode::{
            InvalidResponse, Transport, Unauthorized, UnexpectedHttpStatus,
        };
        let cases = vec![
            (200, valid, 0, None),
            (200, String::new(), 0, Some(InvalidResponse)),
            (200, ": heartbeat\n\n".into(), 0, Some(InvalidResponse)),
            (200, "data: {broken}\n\n".into(), 0, Some(InvalidResponse)),
            (200, truncated.clone(), 0, Some(InvalidResponse)),
            (200, empty_output, 0, Some(InvalidResponse)),
            (401, String::new(), 0, Some(Unauthorized)),
            (404, String::new(), 0, Some(UnexpectedHttpStatus)),
            (200, truncated, 100, Some(Transport)),
        ];
        let expected_cases: Vec<_> =
            cases.iter().map(|(status, _, _, expected)| (*status, *expected)).collect();
        let server = tokio::spawn(async move {
            for (status, body, extra_length, _) in cases {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut headers = Vec::new();
                while !headers.ends_with(b"\r\n\r\n") {
                    headers.push(socket.read_u8().await.unwrap());
                    assert!(headers.len() < 8192);
                }
                let headers = String::from_utf8(headers).unwrap().to_ascii_lowercase();
                assert!(
                    headers
                        .starts_with(&format!("post {} http/1.1\r\n", path.to_ascii_lowercase()))
                );
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                let mut bytes = vec![0; length];
                socket.read_exact(&mut bytes).await.unwrap();
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                assert!(request.get("tools").is_none());
                assert!(request.to_string().contains("Reply with OK."));
                assert!(request.get("stream").is_none_or(|value| value == true));
                socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()+extra_length).as_bytes()).await.unwrap();
                for frame in body.split_inclusive("\n\n") {
                    socket.write_all(frame.as_bytes()).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
            }
            assert!(timeout(Duration::from_millis(50), listener.accept()).await.is_err());
        });
        for (status, expected) in expected_cases {
            let probe = StreamProbe::prepare(
                &database,
                &accounts,
                owner.clone(),
                format.into(),
                "fixture".into(),
            )
            .unwrap();
            let result = probe
                .run_with_source(
                    &accounts,
                    &copilot,
                    CancellationToken::new(),
                    DiagnosticEvidenceSource::ControlledFixture,
                )
                .await
                .ok();
            let Some(DiagnosticResult::Streaming { report, may_incur_cost: true, .. }) = result
            else {
                panic!("stream probe must return a streaming report");
            };
            assert_eq!(report.source, DiagnosticEvidenceSource::ControlledFixture);
            if expected.is_none() {
                let DiagnosticOutcome::Succeeded { evidence } = report.outcome else {
                    panic!(
                        "valid increment and legal completion must succeed: {format}: {:?}",
                        report.outcome
                    );
                };
                assert_eq!(evidence.actual_model_id.as_ref(), "fixture-v1");
                assert_eq!(evidence.http_status, 200);
                assert!(evidence.max_event_gap_ms.is_some());
            } else {
                assert!(
                    matches!(report.outcome, DiagnosticOutcome::Failed {error, http_status:Some(actual)} if Some(error.code) == expected && actual == status),
                    "HTTP200 alone must never pass: {format}"
                );
            }
        }
        let cancel = CancellationToken::new();
        cancel.cancel();
        let probe = StreamProbe::prepare(
            &database,
            &accounts,
            owner.clone(),
            format.into(),
            "fixture".into(),
        )
        .unwrap();
        assert!(matches!(
            probe.run(&accounts, &copilot, cancel).await,
            Err(DiagnosticProbeError::Cancelled)
        ));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn diagnostic_stream_check_cancellation_closes_four_protocol_connections() {
    let directory = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::new(directory.path().join("global.sqlite")).unwrap();
    let providers = AppProviderStore::new(database.clone()).unwrap();
    let credentials = directory.path().join("credentials");
    std::fs::DirBuilder::new().mode(0o700).create(&credentials).unwrap();
    let accounts = AuthAccountStore::new(database.clone(), &credentials).unwrap();
    let copilot = CopilotAuth::new().unwrap();
    let owner = DiagnosticJobOwner {
        scope: DiagnosticScope {
            project: ProjectScope::for_default_tenant("project".into()).unwrap(),
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    };
    for (format, _, valid) in samples() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        providers
            .claude_save_with_metadata(
                owner.scope.project.tenant_scope(),
                format,
                &ClaudeProfile {
                    name: "Fixture".into(),
                    model: Some("fixture".into()),
                    config: ClaudeConfig::ThirdParty {
                        base_url: format!("http://{}/v1", listener.local_addr().unwrap()),
                        api_key: "fixture-key".into(),
                        api_key_header: false,
                    },
                },
                None,
                true,
                Some(json!({"meta":{"apiFormat":format}})),
            )
            .unwrap();
        let (sent, received) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                headers.push(socket.read_u8().await.unwrap());
                assert!(headers.len() < 8192);
            }
            let headers = String::from_utf8(headers).unwrap().to_ascii_lowercase();
            let length: usize = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .unwrap()
                .parse()
                .unwrap();
            socket.read_exact(&mut vec![0; length]).await.unwrap();
            let partial = valid.rsplit_once("data:").unwrap().0;
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{partial}",valid.len()).as_bytes()).await.unwrap();
            sent.send(()).unwrap();
            let mut byte = [0];
            assert_connection_closed(
                timeout(Duration::from_secs(2), socket.read(&mut byte))
                    .await
                    .expect("cancel must close upstream"),
            );
            assert!(
                timeout(Duration::from_millis(50), listener.accept()).await.is_err(),
                "cancel must never retry"
            );
        });
        let probe = StreamProbe::prepare(
            &database,
            &accounts,
            owner.clone(),
            format.into(),
            "fixture".into(),
        )
        .unwrap();
        let cancel = CancellationToken::new();
        let interrupt = async {
            received.await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
            cancel.cancel();
        };
        let (result, ()) = tokio::join!(probe.run(&accounts, &copilot, cancel.clone()), interrupt);
        assert!(matches!(result, Err(DiagnosticProbeError::Cancelled)), "{format}");
        server.await.unwrap();
    }
}

#[tokio::test]
async fn diagnostic_stream_check_classifies_first_event_and_idle_timeouts() {
    use crate::model_relay::generation::GenerationProtocol;
    use vw_api_types::agent_apps_diagnostics::DiagnosticPhase;
    let (_, _, valid) = samples().remove(0);
    for (wire, expected_phase) in [
        (": heartbeat\n\n".to_owned(), DiagnosticPhase::FirstEvent),
        (valid.rsplit_once("data:").unwrap().0.to_owned(), DiagnosticPhase::Stream),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                headers.push(socket.read_u8().await.unwrap());
                assert!(headers.len() < 8192);
            }
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{wire}",wire.len()+100).as_bytes()).await.unwrap();
            let mut byte = [0];
            assert_connection_closed(
                timeout(Duration::from_secs(12), socket.read(&mut byte)).await.unwrap(),
            );
        });
        let mut response =
            reqwest::Client::builder().no_proxy().build().unwrap().get(url).send().await.unwrap();
        let mut phase = DiagnosticPhase::FirstEvent;
        let result = super::read_stream(
            &mut response,
            GenerationProtocol::Chat,
            tokio::time::Instant::now(),
            &mut phase,
        )
        .await;
        assert_eq!(result.unwrap_err(), DiagnosticFailureCode::Timeout);
        assert_eq!(phase, expected_phase);
        drop(response);
        server.await.unwrap();
    }
    // Buffered bytes cannot turn an already elapsed first-event deadline into success.
    let mut response = crate::model_relay::stream_observer::tests::source(valid).await;
    let mut phase = DiagnosticPhase::FirstEvent;
    let started = tokio::time::Instant::now() - Duration::from_secs(11);
    let result =
        super::read_stream(&mut response, GenerationProtocol::Chat, started, &mut phase).await;
    assert_eq!(result.unwrap_err(), DiagnosticFailureCode::Timeout);
    assert_eq!(phase, DiagnosticPhase::FirstEvent);
}

fn assert_connection_closed(result: std::io::Result<usize>) {
    match result {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) => {}
        other => panic!("upstream connection must be closed: {other:?}"),
    }
}
