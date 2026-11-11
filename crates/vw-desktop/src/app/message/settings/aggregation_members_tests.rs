use super::*;
fn selection() -> Selection {
    Selection {
        scope: Scope {
            context: crate::app::state::AgentAppsContext {
                gateway: "http://fixture".into(),
                tenant: Default::default(),
                project: None,
            },
            host: "host".into(),
            app: AppId::ClaudeDesktop,
        },
        project: "project".into(),
        catalog_generation: 1,
    }
}
fn snapshot() -> Snapshot {
    Snapshot {
        state: serde_json::from_value(serde_json::json!({
            "revision":"r1","mode":"aggregation","attached":true,
            "defaultProviderId":"one","directProviderId":null,
            "members":[{"providerId":"one","name":"默认供应商","models":[]},
                       {"providerId":"two","name":"成员","models":[]}],
            "notices":[],"recovery":"none"
        }))
        .unwrap(),
        candidates: vec![],
    }
}
fn ready() -> Form {
    let mut form = Form::new(selection(), 1);
    assert!(form.finish(&selection(), 1, Ok(snapshot())));
    form
}
#[test]
fn aggregation_t57_aggregation_members_preserves_contract() {
    let action = Action::Member { provider_id: "one".into(), enabled: false };
    let mut form = ready();
    assert!(form.begin_action(&selection(), &action).is_none(), "default cannot be removed");
    assert!(!form.loading);
    let action = Action::Member { provider_id: "two".into(), enabled: false };
    assert_eq!(form.begin_action(&selection(), &action).as_deref(), Some("r1"));
    assert!(form.begin_action(&selection(), &action).is_none(), "duplicate write blocked");
}
fn server(
    responses: Vec<(u16, serde_json::Value)>,
) -> (GatewayClient, std::thread::JoinHandle<Vec<(String, serde_json::Value)>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let mut requests = vec![];
        for (status, response) in responses {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5))
                    }
                    Err(e) => panic!("accept: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut bytes = vec![];
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                bytes.push(b[0]);
            }
            let head = String::from_utf8(bytes).unwrap();
            let len = head
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|n| n.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; len];
            stream.read_exact(&mut body).unwrap();
            requests.push((head, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null)));
            let body = response.to_string();
            write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", port)
        .with_auth(vw_gateway_client::GatewayAuth { skey: Some("gateway-key".into()) });
    (GatewayClient::new(endpoint).unwrap(), handle)
}
fn catalog() -> serde_json::Value {
    serde_json::json!({"appId":"claude-desktop","rows":[],"direct":{"status":"unconfigured"},"inUse":{"status":"unconfigured"},"nativeRead":null})
}
#[tokio::test]
async fn aggregation_t57_sdk_mutations_read_back_server_authority() {
    for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
        for action in [
            Action::Member { provider_id: "供应商 /?".into(), enabled: true },
            Action::Member { provider_id: "供应商 /?".into(), enabled: false },
            Action::Default { provider_id: "供应商 /?".into() },
        ] {
            let mut selected = selection();
            selected.scope.app = app;
            let mut observed = snapshot().state;
            observed.revision = "server-readback".into();
            let (client, handle) = server(vec![
                (200, serde_json::to_value(snapshot().state).unwrap()),
                (200, serde_json::to_value(&observed).unwrap()),
                (200, catalog()),
            ]);
            let outcome = mutate(
                client,
                selected,
                "management-grant".into(),
                action.clone(),
                "opaque-revision".into(),
            )
            .await;
            assert_eq!(outcome.result.unwrap().state, observed);
            assert!(outcome.write_error.is_none());
            let requests = handle.join().unwrap();
            assert_eq!(requests.len(), 3);
            let app = serde_json::to_value(app).unwrap();
            let app = app.as_str().unwrap();
            let (head, body) = &requests[0];
            assert!(head.starts_with(&format!("PUT /v1/agent-apps/{app}/aggregation/")));
            assert_eq!(body["targetHostId"], "host");
            assert_eq!(body["projectId"], "project");
            assert_eq!(body["expectedRevision"], "opaque-revision");
            match action {
                Action::Member { enabled, .. } => {
                    assert!(head.contains("/members/%E4%BE%9B%E5%BA%94%E5%95%86%20%2F%3F "));
                    assert_eq!(body["enabled"], enabled);
                }
                Action::Default { .. } => {
                    assert!(head.contains("/default "));
                    assert_eq!(body["providerId"], "供应商 /?");
                }
            }
            assert!(head.to_lowercase().contains("authorization: bearer management-grant\r\n"));
            assert!(requests[1].0.starts_with("GET "));
            assert!(requests[1].0.contains("projectId=project"));
            assert!(requests[2].0.starts_with("GET "));
        }
    }
}
#[tokio::test]
async fn aggregation_t57_failures_do_not_claim_success_and_conflicts_refresh_once() {
    for status in [403, 409, 422, 502, 503, 504] {
        let mut responses = vec![(
            status,
            serde_json::json!({"error":{"code":"fixture","message":"private-payload"}}),
        )];
        let mut fresh = snapshot().state;
        fresh.revision = "fresh".into();
        if status == 409 {
            responses.extend([(200, serde_json::to_value(&fresh).unwrap()), (200, catalog())]);
        }
        let (client, handle) = server(responses);
        let outcome = mutate(
            client,
            selection(),
            "grant".into(),
            Action::Default { provider_id: "two".into() },
            "r1".into(),
        )
        .await;
        let mut form = ready();
        assert!(
            form.begin_action(&selection(), &Action::Default { provider_id: "two".into() })
                .is_some()
        );
        assert!(form.finish_mutation(&selection(), 1, outcome));
        assert!(!form.loading);
        assert!(form.error.is_some());
        assert!(!form.error.unwrap().contains("private-payload"));
        if status == 409 {
            assert_eq!(form.snapshot.unwrap().state.revision, "fresh");
        } else {
            assert!(form.snapshot.is_none());
        }
        let requests = handle.join().unwrap();
        assert_eq!(requests.iter().filter(|r| r.0.starts_with("PUT ")).count(), 1);
        assert_eq!(requests.len(), if status == 409 { 3 } else { 1 });
    }
}
#[test]
fn aggregation_t57_scope_and_generation_discard_late_mutations() {
    let action = Action::Default { provider_id: "two".into() };
    let old = selection();
    for index in 0..5 {
        let mut changed = old.clone();
        match index {
            0 => changed.project = "other".into(),
            1 => changed.scope.host = "other".into(),
            2 => changed.scope.app = AppId::Codex,
            3 => changed.catalog_generation += 1,
            _ => changed.scope.context.gateway = "other".into(),
        }
        let mut form = ready();
        assert!(form.begin_action(&changed, &action).is_none());
        assert!(form.snapshot.is_none());
        assert!(!form.finish_mutation(
            &old,
            1,
            MutationOutcome { result: Ok(snapshot()), write_error: None }
        ));
    }
    let mut form = ready();
    assert!(form.begin_action(&old, &action).is_some());
    form.generation = 2;
    assert!(!form.finish_mutation(
        &old,
        1,
        MutationOutcome { result: Ok(snapshot()), write_error: None }
    ));
    assert!(form.finish_mutation(
        &old,
        2,
        MutationOutcome { result: Ok(snapshot()), write_error: None }
    ));
    assert!(!form.finish_mutation(
        &old,
        2,
        MutationOutcome { result: Ok(snapshot()), write_error: None }
    ));
}

#[test]
fn aggregation_t57_candidates_are_intents_and_noop_writes_are_blocked() {
    let mut form = ready();
    for action in [
        Action::Member { provider_id: "unknown".into(), enabled: true },
        Action::Default { provider_id: "one".into() },
        Action::Member { provider_id: "two".into(), enabled: true },
    ] {
        assert!(form.begin_action(&selection(), &action).is_none());
        assert!(!form.loading);
    }
    form.snapshot.as_mut().unwrap().candidates.push(
        vw_gateway_client::vw_api_types::app_providers::AppProviderSummary {
            id: "candidate".into(),
            app_id: AppId::ClaudeDesktop,
            name: "候选供应商".into(),
            configured: true,
        },
    );
    assert_eq!(
        form.begin_action(
            &selection(),
            &Action::Member { provider_id: "candidate".into(), enabled: true }
        )
        .as_deref(),
        Some("r1")
    );
    assert_eq!(
        form.snapshot.as_ref().unwrap().state.members.len(),
        2,
        "intent never changes local membership optimistically"
    );
}

#[tokio::test]
async fn aggregation_t57_readback_failure_requires_refresh_without_replaying_write() {
    let (client, handle) = server(vec![
        (200, serde_json::to_value(snapshot().state).unwrap()),
        (503, serde_json::json!({"error":{"code":"offline"}})),
    ]);
    let outcome = mutate(
        client,
        selection(),
        "grant".into(),
        Action::Default { provider_id: "two".into() },
        "r1".into(),
    )
    .await;
    let mut form = ready();
    form.begin_action(&selection(), &Action::Default { provider_id: "two".into() }).unwrap();
    assert!(form.finish_mutation(&selection(), 1, outcome));
    assert!(form.snapshot.is_none());
    assert!(form.error.is_some());
    assert!(
        form.begin_action(&selection(), &Action::Default { provider_id: "two".into() }).is_none()
    );
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].0.starts_with("PUT "));
    assert!(requests[1].0.starts_with("GET "));
}
