use super::{AgentAppsSettingsError, GatewayClient, test_support};
use serde_json::{Value, json};
use vw_api_types::{
    TaskId, TenantScope, agent_apps::AppId, agent_apps_diagnostics::*, id::ProjectScope,
};

fn scope() -> DiagnosticScopeQuery {
    DiagnosticScopeQuery {
        project_id: "project /?中文".into(),
        target_host_id: "host /?中文".into(),
        app_id: AppId::Claude,
    }
}

fn identity(kind: DiagnosticProbeKind) -> DiagnosticJobIdentity {
    let scope = scope();
    DiagnosticJobIdentity {
        task_id: "job /?中文%".into(),
        correlation_id: "server-correlation".into(),
        scope: DiagnosticScope {
            project: ProjectScope::new(TenantScope::default(), scope.project_id).unwrap(),
            target_host_id: scope.target_host_id,
            app_id: scope.app_id,
        },
        kind,
    }
}

fn snapshot(job: &DiagnosticJobIdentity, state: DiagnosticJobState) -> DiagnosticJobSnapshot {
    DiagnosticJobSnapshot {
        job: job.clone(),
        state,
        progress: DiagnosticProgress { completed: 1, total: Some(2) },
        results: Some(vec![DiagnosticResult::Reachability {
            provider_id: "provider".into(),
            report: DiagnosticReport {
                source: DiagnosticEvidenceSource::ControlledFixture,
                timings: vec![DiagnosticTiming { phase: DiagnosticPhase::Connect, elapsed_ms: 17 }],
                outcome: DiagnosticOutcome::Failed {
                    error: DiagnosticFailure {
                        phase: DiagnosticPhase::Connect,
                        code: DiagnosticFailureCode::Timeout,
                    },
                    http_status: None,
                },
            },
        }]),
    }
}

#[tokio::test]
async fn diagnostic_sdk_preserves_job_identity() {
    for (request, kind) in [
        (
            DiagnosticStartRequest::Reachability { provider_id: "provider".into() },
            DiagnosticProbeKind::Reachability,
        ),
        (
            DiagnosticStartRequest::Speed {
                provider_id: "provider".into(),
                endpoint_ids: vec![DiagnosticEndpointId("opaque-endpoint".into())],
            },
            DiagnosticProbeKind::Speed,
        ),
        (
            DiagnosticStartRequest::Model {
                provider_id: "provider".into(),
                model_id: "model".into(),
            },
            DiagnosticProbeKind::Model,
        ),
        (
            DiagnosticStartRequest::Streaming {
                provider_id: "provider".into(),
                model_id: "model".into(),
            },
            DiagnosticProbeKind::Streaming,
        ),
        (
            DiagnosticStartRequest::Mcp {
                server_ids: vec![vw_api_types::mcp_management::McpRecordId("server".into())],
            },
            DiagnosticProbeKind::Mcp,
        ),
    ] {
        let job = identity(kind);
        let cancelling = snapshot(&job, DiagnosticJobState::Cancelling {});
        let cancelled = snapshot(&job, DiagnosticJobState::Cancelled {});
        let server = test_support::server(vec![
            (202, serde_json::to_value(&job).unwrap()),
            (200, serde_json::to_value(&cancelling).unwrap()),
            (200, serde_json::to_value(&cancelled).unwrap()),
            (200, serde_json::to_value(&cancelled).unwrap()),
        ]);
        let endpoint = server
            .client()
            .endpoint()
            .clone()
            .with_auth(crate::GatewayAuth { skey: Some("fixture-gateway-auth".into()) });
        let client = GatewayClient::new(endpoint).unwrap();
        assert_eq!(
            client.diagnostic_start(&scope(), &request, "fixture-member").await.unwrap(),
            job
        );
        assert_eq!(
            client.diagnostic_cancel(&scope(), &job.task_id, "fixture-member").await.unwrap(),
            cancelling
        );
        assert_eq!(
            client.diagnostic_status(&scope(), &job.task_id, "fixture-member").await.unwrap(),
            cancelled
        );
        assert_eq!(
            client.diagnostic_cancel(&scope(), &job.task_id, "fixture-member").await.unwrap(),
            cancelled
        );
        for (method, suffix, body) in [
            ("POST", "", serde_json::to_value(&request).unwrap()),
            ("POST", "/job%20%2F%3F%E4%B8%AD%E6%96%87%25/cancel", Value::Null),
            ("GET", "/job%20%2F%3F%E4%B8%AD%E6%96%87%25", Value::Null),
            ("POST", "/job%20%2F%3F%E4%B8%AD%E6%96%87%25/cancel", Value::Null),
        ] {
            let recorded = server.take_request();
            assert_eq!(recorded.method, method);
            assert_eq!(
                recorded.path,
                format!(
                    "/v1/agent-apps/diagnostics/jobs{suffix}?projectId=project+%2F%3F%E4%B8%AD%E6%96%87&targetHostId=host+%2F%3F%E4%B8%AD%E6%96%87&appId=claude"
                )
            );
            assert_eq!(recorded.body, body);
            assert_eq!(recorded.header("x-agent-apps-token"), Some("fixture-member"));
            assert_eq!(recorded.header("authorization"), Some("Bearer fixture-gateway-auth"));
            assert_eq!(
                recorded.header("x-platform-id"),
                Some(client.endpoint().tenant_scope().platform_id().as_ref())
            );
            assert_eq!(
                recorded.header("x-company-id"),
                Some(client.endpoint().tenant_scope().company_id().as_ref())
            );
        }
        server.join();
    }
    for status in [401, 403, 404, 429, 503] {
        let body = json!({"error":{"code":format!("fixture-{status}"),"message":"safe error","details":{"correlationId":"error-correlation"}}});
        let server = test_support::server(vec![(status, body.clone()); 3]);
        let client = server.client();
        let expected = AgentAppsSettingsError::Http { status, body };
        let request = DiagnosticStartRequest::Reachability { provider_id: "provider".into() };
        assert_eq!(
            client.diagnostic_start(&scope(), &request, "fixture-member").await.unwrap_err(),
            expected
        );
        assert_eq!(
            client
                .diagnostic_status(&scope(), &"missing".into(), "fixture-member")
                .await
                .unwrap_err(),
            expected
        );
        assert_eq!(
            client
                .diagnostic_cancel(&scope(), &"missing".into(), "fixture-member")
                .await
                .unwrap_err(),
            expected
        );
        server.join();
    }
    // A real local HTTP peer observes whether a redirect or lost acceptance replays a submission.
    assert_start_is_not_replayed(true).await;
    assert_start_is_not_replayed(false).await;
}

async fn assert_start_is_not_replayed(redirect: bool) {
    use std::{
        io::{BufRead, Read, Write},
        net::TcpListener,
        time::{Duration, Instant},
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(4);
        let mut first = None;
        let mut count = 0;
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
                    let mut reader = std::io::BufReader::new(&mut stream);
                    let mut length = 0;
                    loop {
                        let mut line = String::new();
                        assert!(reader.read_line(&mut line).unwrap() > 0);
                        if line == "\r\n" {
                            break;
                        }
                        if let Some(value) =
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                        {
                            length = value.trim().parse::<usize>().unwrap();
                        }
                    }
                    let mut body = vec![0; length];
                    reader.read_exact(&mut body).unwrap();
                    assert_eq!(
                        serde_json::from_slice::<Value>(&body).unwrap(),
                        json!({"kind":"reachability","providerId":"provider"})
                    );
                    count += 1;
                    first.get_or_insert(Instant::now());
                    if redirect && count == 1 {
                        write!(stream, "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{port}/replayed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                    } else if redirect {
                        write!(stream, "HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                    }
                    // Without a redirect, drop the connection after reading the complete POST.
                    // The caller cannot know whether the host accepted it.
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if first.is_some_and(|at: Instant| at.elapsed() > Duration::from_millis(350)) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("{error}"),
            }
        }
        count
    });
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    let result = client
        .diagnostic_start(
            &scope(),
            &DiagnosticStartRequest::Reachability { provider_id: "provider".into() },
            "fixture-member",
        )
        .await;
    let count = handle.join().unwrap();
    assert_eq!(count, 1, "diagnostic submission must not replay on redirect or failed acceptance");
    if redirect {
        assert_eq!(
            result.unwrap_err(),
            AgentAppsSettingsError::Http { status: 307, body: json!("") }
        );
    } else {
        assert!(matches!(result, Err(AgentAppsSettingsError::Transport(_))));
    }
}

#[tokio::test]
async fn diagnostic_sdk_preserves_job_identity_rejects_invalid_responses_and_ids() {
    let server = test_support::server_raw(vec![(202, "{}".into()), (502, "not json".into())]);
    assert!(matches!(
        server
            .client()
            .diagnostic_start(
                &scope(),
                &DiagnosticStartRequest::Mcp { server_ids: vec![] },
                "fixture"
            )
            .await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    assert_eq!(
        server.client().diagnostic_status(&scope(), &"job".into(), "fixture").await.unwrap_err(),
        AgentAppsSettingsError::Http { status: 502, body: json!("not json") }
    );
    server.join();
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    for id in ["", ".", ".."] {
        let id = TaskId::from(id);
        assert!(
            matches!(client.diagnostic_status(&scope(), &id, "fixture").await, Err(AgentAppsSettingsError::Transport(message)) if message == "invalid diagnostic job identity")
        );
        assert!(
            matches!(client.diagnostic_cancel(&scope(), &id, "fixture").await, Err(AgentAppsSettingsError::Transport(message)) if message == "invalid diagnostic job identity")
        );
    }
}

#[tokio::test]
async fn diagnostic_endpoints_sdk_uses_scoped_body() {
    use std::{
        io::{BufRead, Read, Write},
        net::TcpListener,
        time::{Duration, Instant},
    };
    let query = DiagnosticEndpointCandidatesQuery {
        provider_id: "provider /?中文".into(),
        draft_urls: vec!["https://draft.example/私有/".into()],
        preset_name: Some("preset /?中文".into()),
    };
    let expected = DiagnosticEndpointCandidatesResponse {
        candidates: vec![DiagnosticEndpointCandidate {
            id: DiagnosticEndpointId("host-issued-id".into()),
            display_url: "https://draft.example/私有".into(),
            sources: vec![
                DiagnosticEndpointSource::Draft,
                DiagnosticEndpointSource::SavedConfig,
                DiagnosticEndpointSource::Preset,
                DiagnosticEndpointSource::Custom,
            ],
        }],
    };
    for status in [200, 403] {
        let body = if status == 200 {
            serde_json::to_value(&expected).unwrap()
        } else {
            json!({"error":{"code":"forbidden","details":{"correlationId":"denied"}}})
        };
        let response_body = body.to_string();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        // Observe every connection, including unexpected save/probe/retry requests.
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut first = None;
            let mut requests = Vec::new();
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
                        let mut reader = std::io::BufReader::new(&mut stream);
                        let mut lines = Vec::new();
                        let mut length = 0;
                        loop {
                            let mut line = String::new();
                            assert!(reader.read_line(&mut line).unwrap() > 0);
                            if line == "\r\n" {
                                break;
                            }
                            if let Some(value) =
                                line.to_ascii_lowercase().strip_prefix("content-length:")
                            {
                                length = value.trim().parse::<usize>().unwrap();
                            }
                            lines.push(line.trim().to_string());
                        }
                        let mut bytes = vec![0; length];
                        reader.read_exact(&mut bytes).unwrap();
                        requests.push((lines, serde_json::from_slice::<Value>(&bytes).unwrap()));
                        first.get_or_insert(Instant::now());
                        write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}", response_body.len()).unwrap();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if first
                            .is_some_and(|at: Instant| at.elapsed() > Duration::from_millis(350))
                        {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            requests
        });
        let endpoint = crate::GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(crate::GatewayAuth { skey: Some("fixture-gateway-auth".into()) });
        let client = GatewayClient::new(endpoint).unwrap();
        let result =
            client.diagnostic_endpoint_candidates(&scope(), &query, "fixture-member").await;
        let requests = handle.join().unwrap();
        assert_eq!(requests.len(), 1, "candidate query must not probe, save, or retry");
        let (lines, recorded_body) = &requests[0];
        assert_eq!(
            lines[0],
            "POST /v1/agent-apps/diagnostics/endpoint-candidates/query?projectId=project+%2F%3F%E4%B8%AD%E6%96%87&targetHostId=host+%2F%3F%E4%B8%AD%E6%96%87&appId=claude HTTP/1.1",
            "candidate query must use its read-only endpoint with only scope in the URL"
        );
        assert_eq!(*recorded_body, serde_json::to_value(&query).unwrap());
        for (name, value) in [
            ("x-agent-apps-token", "fixture-member"),
            ("authorization", "Bearer fixture-gateway-auth"),
            ("x-platform-id", client.endpoint().tenant_scope().platform_id().as_ref()),
            ("x-company-id", client.endpoint().tenant_scope().company_id().as_ref()),
        ] {
            assert!(
                lines.iter().any(|line| line.split_once(':').is_some_and(|(key, actual)| key
                    .eq_ignore_ascii_case(name)
                    && actual.trim() == value)),
                "missing {name}"
            );
        }
        if status == 200 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert_eq!(result.unwrap_err(), AgentAppsSettingsError::Http { status, body });
        }
    }
}
