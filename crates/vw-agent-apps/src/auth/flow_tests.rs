use super::{
    credential_file::{AccountCredential, CredentialFileStore},
    flow::{AuthAccountScope, AuthDevicePoll, AuthFlowTiming, AuthFlows},
};
use std::{
    sync::{
        Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthLoginMethod,
        AuthService, AuthStartRequest,
    },
    id::TenantScope,
};

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "host",
        service: AuthService::CodexOauth,
    }
}

fn request(target: &str) -> AuthStartRequest {
    AuthStartRequest {
        method: AuthLoginMethod::DeviceCode,
        github_domain: None,
        region: None,
        target_account_id: Some(target.into()),
    }
}

fn timing(now: Instant) -> AuthFlowTiming {
    AuthFlowTiming {
        expires_at: TimestampMs(1_800_000_000_000),
        deadline: now + Duration::from_secs(30),
        retry_after_seconds: 5,
    }
}

fn pending(flow: AuthFlow) -> AuthFlowId {
    let AuthFlow::Pending { flow_id, retry_after_seconds: 5, .. } = flow else {
        panic!("expected a pending flow with the configured interval");
    };
    flow_id
}

fn account(id: &str) -> AuthAccount {
    AuthAccount {
        account_id: id.into(),
        service: AuthService::CodexOauth,
        label: "Fixture account".into(),
        credential_state: AuthCredentialState::Ready,
        is_default: false,
        authenticated_at: Some(TimestampMs(1_799_999_999_000)),
        login_expires_at: None,
        github_domain: None,
        region: None,
    }
}

#[test]
fn auth6_flow_commits_expected_scoped_result() {
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let now = Instant::now();
    let started = flows.start(scope(&tenant), request("target"), timing(now));
    assert!(matches!(started, Ok(AuthFlow::Pending { .. })), "start result: {started:?}");
    let id = pending(started.unwrap());
    let second = pending(flows.start(scope(&tenant), request("target"), timing(now)).unwrap());
    assert_ne!(id, second);
    assert!(matches!(flows.snapshot(scope(&tenant), &id, || now), Ok(AuthFlow::Pending { .. })));

    let root = tempfile::Builder::new()
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .tempdir()
        .unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let commits = AtomicUsize::new(0);
    let ready = Barrier::new(2);
    let attempt = || {
        ready.wait();
        flows.complete(
            scope(&tenant),
            &id,
            || now,
            Ok(account("target")),
            |bound, started, approved| {
                assert_eq!(bound.tenant, &tenant);
                assert_eq!(bound.host, "host");
                assert_eq!(bound.principal, "oidc:owner");
                assert_eq!(bound.service, AuthService::CodexOauth);
                assert_eq!(started.target_account_id.as_deref(), Some("target"));
                let credential = AccountCredential::CodexAuth(serde_json::json!({
                    "tokens": {
                        "access_token": "fixture-private-access",
                        "refresh_token": "fixture-private-refresh",
                        "id_token": "fixture-private-id",
                        "account_id": "fixture-external-account"
                    }
                }));
                commits.fetch_add(1, Ordering::Relaxed);
                store
                    .for_owner(bound.principal)
                    .and_then(|owner| {
                        owner.save(
                            bound.tenant,
                            bound.host,
                            AppId::Codex,
                            &approved.account_id,
                            &credential,
                        )
                    })
                    .map_err(|_| AuthErrorCode::StorageUnavailable)
            },
        )
    };
    let (a, b) = std::thread::scope(|threads| {
        let concurrent = threads.spawn(attempt);
        (attempt(), concurrent.join().unwrap())
    });
    assert_eq!(commits.load(Ordering::Relaxed), 1);
    let approved = AuthFlow::Approved {
        flow_id: id.clone(),
        expires_at: timing(now).expires_at,
        account: account("target"),
    };
    assert!(matches!(
        (&a, &b),
        (Ok(_), Err(AuthErrorCode::Conflict)) | (Err(AuthErrorCode::Conflict), Ok(_))
    ));
    assert_eq!(a.or(b), Ok(approved.clone()));
    assert_eq!(flows.snapshot(scope(&tenant), &id, || now), Ok(approved.clone()));
    assert!(matches!(
        flows.snapshot(scope(&tenant), &second, || now),
        Ok(AuthFlow::Pending { .. })
    ));
    assert!(!serde_json::to_string(&approved).unwrap().contains("fixture-private"));

    let owner = store.for_owner("oidc:owner").unwrap();
    let AccountCredential::CodexAuth(saved) =
        owner.read(&tenant, "host", AppId::Codex, "target").unwrap()
    else {
        panic!("expected the scoped Codex credential");
    };
    assert_eq!(saved["tokens"]["account_id"], "fixture-external-account");
    assert!(
        store
            .for_owner("oidc:other")
            .unwrap()
            .read(&tenant, "host", AppId::Codex, "target")
            .is_err()
    );
}

#[test]
fn auth6_flow_rejects_invalid_or_stale_input() {
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let now = Instant::now();
    assert_eq!(
        flows.start(scope(&tenant), request("../target"), timing(now)),
        Err(AuthErrorCode::InvalidInput)
    );
    assert_eq!(
        flows.start(
            AuthAccountScope { service: AuthService::XaiOauth, ..scope(&tenant) },
            AuthStartRequest {
                method: AuthLoginMethod::Browser,
                target_account_id: None,
                ..request("target")
            },
            timing(now)
        ),
        Err(AuthErrorCode::Unsupported)
    );
    let id = pending(flows.start(scope(&tenant), request("target"), timing(now)).unwrap());
    let never_write = |_: AuthAccountScope<'_>, _: &AuthStartRequest, _: &AuthAccount| {
        panic!("invalid or stale completion must not enter the commit operation")
    };
    let other_platform = TenantScope::new("other", "company").unwrap();
    let other_company = TenantScope::new("platform", "other").unwrap();
    for other in [
        scope(&other_platform),
        scope(&other_company),
        AuthAccountScope { principal: "oidc:other", ..scope(&tenant) },
        AuthAccountScope { host: "other", ..scope(&tenant) },
        AuthAccountScope { service: AuthService::KimiOauth, ..scope(&tenant) },
    ] {
        assert_eq!(flows.snapshot(other, &id, || now), Err(AuthErrorCode::NotFound));
        assert_eq!(
            flows.complete(other, &id, || now, Ok(account("target")), never_write),
            Err(AuthErrorCode::NotFound)
        );
    }
    let wrong_target =
        flows.complete(scope(&tenant), &id, || now, Ok(account("different")), never_write);
    assert!(
        matches!(&wrong_target, Ok(AuthFlow::Failed { error, .. }) if error.code == AuthErrorCode::IdentityMismatch)
    );
    assert_eq!(flows.snapshot(scope(&tenant), &id, || now), wrong_target);
    assert_eq!(
        flows.complete(scope(&tenant), &id, || now, Ok(account("target")), never_write),
        Err(AuthErrorCode::Conflict)
    );

    let id = pending(flows.start(scope(&tenant), request("target"), timing(now)).unwrap());
    let deadline = timing(now).deadline;
    assert_eq!(
        flows.complete(scope(&tenant), &id, || deadline, Ok(account("target")), never_write),
        Err(AuthErrorCode::Conflict)
    );
    assert!(matches!(flows.snapshot(scope(&tenant), &id, || now), Ok(AuthFlow::Expired { .. })));

    let id = pending(flows.start(scope(&tenant), request("target"), timing(now)).unwrap());
    assert_eq!(flows.cancel(scope(&tenant), &id, || now), Ok(()));
    assert_eq!(flows.cancel(scope(&tenant), &id, || now), Ok(()));
    assert!(matches!(flows.snapshot(scope(&tenant), &id, || now), Ok(AuthFlow::Cancelled { .. })));
    assert_eq!(
        flows.complete(scope(&tenant), &id, || now, Ok(account("target")), never_write),
        Err(AuthErrorCode::Conflict)
    );

    let id = pending(flows.start(scope(&tenant), request("target"), timing(now)).unwrap());
    let root = tempfile::Builder::new()
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .tempdir()
        .unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let failed = flows.complete(
        scope(&tenant),
        &id,
        || now,
        Ok(account("target")),
        |bound, _, approved| {
            store
                .for_owner(bound.principal)
                .and_then(|owner| {
                    owner.save(
                        bound.tenant,
                        bound.host,
                        AppId::Codex,
                        &approved.account_id,
                        &AccountCredential::CodexAuth(
                            serde_json::json!({"unexpected": "fixture-private"}),
                        ),
                    )
                })
                .map_err(|_| AuthErrorCode::StorageUnavailable)
        },
    );
    assert!(
        matches!(&failed, Ok(AuthFlow::Failed { error, .. }) if error.code == AuthErrorCode::StorageUnavailable)
    );
    assert!(!serde_json::to_string(&failed.unwrap()).unwrap().contains("fixture-private"));
    assert_eq!(
        flows.complete(scope(&tenant), &id, || now, Ok(account("target")), never_write),
        Err(AuthErrorCode::Conflict)
    );
}

#[tokio::test]
async fn auth6_cancel_commits_expected_scoped_result() {
    use tokio::{
        io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
        sync::oneshot,
        time::timeout,
    };

    timeout(Duration::from_secs(10), async {
        let flows = AuthFlows::default();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let now = Instant::now();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let callback_addr = listener.local_addr().unwrap();
        let listener = std::sync::Arc::new(std::sync::Mutex::new(Some(listener)));
        let id = pending(
            flows
                .start_with_cleanup(
                    scope(&tenant),
                    AuthStartRequest { method: AuthLoginMethod::Browser, ..request("target") },
                    timing(now),
                    {
                        let listener = listener.clone();
                        move || {
                            drop(listener.lock().unwrap().take());
                            Ok(())
                        }
                    },
                )
                .unwrap(),
        );
        // Only the upstream token endpoint is substituted. Hold its response
        // until cancellation has returned, after the exchange is in flight.
        let token_endpoint = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let token_url = format!("http://{}/oauth/token", token_endpoint.local_addr().unwrap());
        let redirect_uri = format!("http://{callback_addr}/auth/callback");
        let body: String = reqwest::Url::parse_with_params(
            "https://fixture.invalid/",
            [
                ("grant_type", "authorization_code"),
                ("code", "fixture-code"),
                ("redirect_uri", redirect_uri.as_str()),
                ("client_id", "fixture-client"),
                ("code_verifier", "fixture-verifier"),
            ],
        )
        .unwrap()
        .query()
        .unwrap()
        .into();
        let (arrived, request_seen) = oneshot::channel();
        let (resume, cancelled) = oneshot::channel();
        let server = async {
            let (stream, _) = token_endpoint.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert_eq!(line, "POST /oauth/token HTTP/1.1\r\n");
            let mut headers = Vec::new();
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                headers.push(line.trim().to_ascii_lowercase());
            }
            assert!(headers.contains(&"content-type: application/x-www-form-urlencoded".into()));
            assert!(headers.contains(&format!("content-length: {}", body.len())));
            let mut request_body = vec![0; body.len()];
            stream.read_exact(&mut request_body).await.unwrap();
            assert_eq!(request_body, body.as_bytes());
            arrived.send(()).unwrap();
            cancelled.await.unwrap();
            let tokens = r#"{"access_token":"fixture-private-access","refresh_token":"fixture-private-refresh","id_token":"fixture-private-id","expires_in":3600}"#;
            stream
                .get_mut()
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{tokens}",
                        tokens.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        };
        let root = tempfile::Builder::new()
            .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
            .tempdir()
            .unwrap();
        let store = CredentialFileStore::new(root.path()).unwrap();
        let metadata = root.path().join("account.json");
        let exchange = async {
            let tokens = reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .post(token_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body.clone())
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            assert_eq!(tokens["expires_in"], 3600);
            flows.complete(
                scope(&tenant),
                &id,
                || now,
                Ok(account("target")),
                |bound, _, approved| {
                    let credential = AccountCredential::CodexAuth(serde_json::json!({
                        "tokens": {
                            "access_token": tokens["access_token"],
                            "refresh_token": tokens["refresh_token"],
                            "id_token": tokens["id_token"],
                            "account_id": "fixture-external-account",
                        }
                    }));
                    store
                        .for_owner(bound.principal)
                        .and_then(|owner| {
                            owner.save(
                                bound.tenant,
                                bound.host,
                                AppId::Codex,
                                &approved.account_id,
                                &credential,
                            )
                        })
                        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
                    std::fs::write(&metadata, serde_json::to_vec(approved).unwrap())
                        .map_err(|_| AuthErrorCode::StorageUnavailable)
                },
            )
        };
        let cancel = async {
            request_seen.await.unwrap();
            assert!(tokio::net::TcpStream::connect(callback_addr).await.is_ok());
            assert_eq!(flows.cancel(scope(&tenant), &id, || now), Ok(()));
            let closed = tokio::net::TcpStream::connect(callback_addr).await;
            resume.send(()).unwrap();
            closed
        };
        let ((), completed, closed) = tokio::join!(server, exchange, cancel);
        assert_eq!(
            completed,
            Err(AuthErrorCode::Conflict),
            "late token exchange must not enter the scoped commit"
        );
        assert!(matches!(
            closed,
            Err(ref error) if error.kind() == std::io::ErrorKind::ConnectionRefused
        ), "successful cancellation must close the callback listener");
        assert_eq!(
            flows.snapshot(scope(&tenant), &id, || now),
            Ok(AuthFlow::Cancelled { flow_id: id, expires_at: timing(now).expires_at })
        );
        assert!(!metadata.exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    })
    .await
    .expect("token exchange and cancellation should finish");
}

#[test]
fn auth6_cancel_rejects_invalid_or_stale_input() {
    use std::{
        process::Command,
        sync::{Arc, Mutex},
    };

    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = AuthAccountScope { service: AuthService::ClaudeOfficial, ..scope(&tenant) };
    let now = Instant::now();
    let child = Arc::new(Mutex::new(Command::new("/bin/sleep").arg("30").spawn().unwrap()));
    let attempts = Arc::new(AtomicUsize::new(0));
    let id = pending(
        flows
            .start_with_cleanup(
                owner,
                AuthStartRequest {
                    method: AuthLoginMethod::CopyLink,
                    target_account_id: None,
                    ..request("target")
                },
                timing(now),
                {
                    let (child, attempts) = (Arc::clone(&child), Arc::clone(&attempts));
                    move || {
                        if attempts.fetch_add(1, Ordering::Relaxed) == 0 {
                            return Err(AuthErrorCode::ExecutionHostUnavailable);
                        }
                        let mut child = child.lock().unwrap();
                        let unavailable = |_| AuthErrorCode::ExecutionHostUnavailable;
                        if child.try_wait().map_err(unavailable)?.is_none() {
                            child.kill().map_err(unavailable)?;
                            child.wait().map_err(unavailable)?;
                        }
                        Ok(())
                    }
                },
            )
            .unwrap(),
    );
    let other = AuthAccountScope { principal: "oidc:other", ..owner };
    let unauthorized = flows.cancel(other, &id, || now);
    let attempts_after_unauthorized = attempts.load(Ordering::Relaxed);
    let first = flows.cancel(owner, &id, || now);
    let pending_after_failure = flows.snapshot(owner, &id, || now);
    let mut wrote = false;
    let stale = flows.complete(
        owner,
        &id,
        || now,
        Ok(AuthAccount { service: owner.service, ..account("target") }),
        |_, _, _| {
            wrote = true;
            Ok(())
        },
    );
    // Cleanup may be retried after the original authorization deadline.
    let deadline = timing(now).deadline;
    let retry = flows.cancel(owner, &id, || deadline);
    let repeated = flows.cancel(owner, &id, || deadline);
    let mut child = child.lock().unwrap();
    let reaped = child.try_wait().unwrap().is_some();
    if !reaped {
        child.kill().unwrap();
        child.wait().unwrap();
    }

    assert_eq!(unauthorized, Err(AuthErrorCode::NotFound));
    assert_eq!(attempts_after_unauthorized, 0);
    assert_eq!(
        first,
        Err(AuthErrorCode::ExecutionHostUnavailable),
        "cancellation must report a host cleanup failure"
    );
    assert!(matches!(pending_after_failure, Ok(AuthFlow::Pending { .. })));
    assert_eq!(stale, Err(AuthErrorCode::Conflict));
    assert!(!wrote);
    assert_eq!(retry, Ok(()));
    assert_eq!(repeated, Ok(()));
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
    assert!(reaped, "successful cancellation must reap the local CLI process");
    assert!(matches!(flows.snapshot(owner, &id, || deadline), Ok(AuthFlow::Cancelled { .. })));
}

#[tokio::test]
async fn auth6_expiry_commits_expected_scoped_result() {
    use std::cell::Cell;

    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = scope(&tenant);
    let before_start = Instant::now();
    let mut expiry = timing(before_start);
    expiry.deadline += Duration::from_secs(120);
    let id = pending(flows.start(owner, request("target"), expiry).unwrap());
    let registered = Instant::now();
    let first_due = registered + Duration::from_secs(5);
    let now = Cell::new(registered);
    let exchanges = Cell::new(0);
    let no_write = |_: AuthAccountScope<'_>, _: &AuthStartRequest, _: &AuthAccount, ()| {
        panic!("a pending or expired exchange must not enter the commit operation")
    };
    let early_poll = async |at| {
        flows
            .poll_device(
                owner,
                &id,
                || at,
                || async { panic!("an early or concurrent poll must not send an exchange") },
                no_write,
            )
            .await
    };
    let expect_pending = |seconds| {
        Ok(AuthFlow::Pending {
            flow_id: id.clone(),
            expires_at: timing(before_start).expires_at,
            retry_after_seconds: seconds,
        })
    };

    assert_eq!(
        early_poll(now.get()).await,
        expect_pending(5),
        "first poll must wait for the interval"
    );
    for (sent, received, outcome, interval) in [
        (0, 0, AuthDevicePoll::Pending, 5),
        (5, 7, AuthDevicePoll::SlowDown, 10),
        (17, 18, AuthDevicePoll::SlowDown, 15),
        (33, 33, AuthDevicePoll::Pending, 15),
    ] {
        now.set(first_due + Duration::from_secs(sent));
        let polled = flows
            .poll_device(
                owner,
                &id,
                || now.get(),
                || async {
                    exchanges.set(exchanges.get() + 1);
                    if sent == 0 {
                        // Even a due request cannot run while one is in flight.
                        assert_eq!(
                            early_poll(now.get() + Duration::from_secs(5)).await,
                            expect_pending(5)
                        );
                    }
                    now.set(first_due + Duration::from_secs(received));
                    outcome
                },
                no_write,
            )
            .await;
        assert_eq!(polled, expect_pending(interval));
        now.set(now.get() + Duration::from_secs(interval - 1));
        assert_eq!(early_poll(now.get()).await, expect_pending(interval));
    }
    assert_eq!(exchanges.get(), 4);
    now.set(first_due + Duration::from_secs(48));
    assert_eq!(
        flows
            .poll_device(owner, &id, || now.get(), || async { AuthDevicePoll::Expired }, no_write)
            .await,
        Ok(AuthFlow::Expired { flow_id: id.clone(), expires_at: timing(before_start).expires_at })
    );
    assert!(matches!(flows.snapshot(owner, &id, || now.get()), Ok(AuthFlow::Expired { .. })));
}

#[tokio::test]
async fn auth6_expiry_rejects_invalid_or_stale_input() {
    use std::cell::Cell;
    use tokio::{
        io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };

    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = AuthAccountScope { service: AuthService::KimiOauth, ..scope(&tenant) };
    let before_start = Instant::now();
    let deadline = timing(before_start).deadline;
    let root = tempfile::tempdir().unwrap();
    let challenge = root.path().join("pending");
    let credential = root.path().join("credential.json");
    let metadata = root.path().join("account.json");
    std::fs::write(&challenge, b"fixture-device-code").unwrap();
    let mut first_cleanup = true;
    let id = pending(
        flows
            .start_with_cleanup(owner, request("target"), timing(before_start), {
                let challenge = challenge.clone();
                move || {
                    if std::mem::take(&mut first_cleanup) {
                        return Err(AuthErrorCode::ExecutionHostUnavailable);
                    }
                    std::fs::remove_file(&challenge)
                        .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)
                }
            })
            .unwrap(),
    );

    // No request from the client is needed to release the host resource.
    flows.reap_expired(deadline - Duration::from_millis(1)).unwrap();
    assert!(challenge.exists());
    assert_eq!(
        flows.reap_expired(deadline),
        Err(AuthErrorCode::ExecutionHostUnavailable),
        "expiry must attempt host cleanup even when no client polls"
    );
    assert!(matches!(flows.snapshot(owner, &id, || deadline), Ok(AuthFlow::Expired { .. })));
    assert!(challenge.exists());
    flows.reap_expired(deadline).unwrap();
    assert!(!challenge.exists(), "the next host sweep must retry cleanup");
    flows.reap_expired(deadline).unwrap();

    let late_id = pending(flows.start(owner, request("target"), timing(before_start)).unwrap());
    let wrong_owner = AuthAccountScope { principal: "oidc:other", ..owner };
    let now = Cell::new(Instant::now() + Duration::from_secs(5));
    let persist = |_: AuthAccountScope<'_>,
                   _: &AuthStartRequest,
                   approved: &AuthAccount,
                   tokens: serde_json::Value| {
        std::fs::write(&credential, serde_json::to_vec(&tokens).unwrap())
            .and_then(|()| std::fs::write(&metadata, serde_json::to_vec(approved).unwrap()))
            .map_err(|_| AuthErrorCode::StorageUnavailable)
    };
    assert_eq!(
        flows
            .poll_device(
                wrong_owner,
                &late_id,
                || now.get(),
                || async { panic!("a different owner must not start a device exchange") },
                persist
            )
            .await,
        Err(AuthErrorCode::NotFound)
    );
    let endpoint = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token_url = format!("http://{}/api/oauth/token", endpoint.local_addr().unwrap());
    let body = "client_id=17e5f671-d194-4dfb-9706-5516cb48c098&device_code=fixture-device-code&grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code";
    let server = async {
        let (stream, _) = endpoint.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "POST /api/oauth/token HTTP/1.1\r\n");
        let mut headers = Vec::new();
        loop {
            line.clear();
            assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            headers.push(line.trim().to_ascii_lowercase());
        }
        assert!(headers.contains(&"content-type: application/x-www-form-urlencoded".into()));
        assert!(headers.contains(&format!("content-length: {}", body.len())));
        let mut sent = vec![0; body.len()];
        stream.read_exact(&mut sent).await.unwrap();
        assert_eq!(sent, body.as_bytes());
        // The exchange started before expiry. Advance the host clock before
        // releasing the successful upstream response.
        now.set(deadline);
        let tokens = r#"{"access_token":"fixture-access","refresh_token":"fixture-refresh","expires_in":3600}"#;
        stream.get_mut().write_all(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{tokens}", tokens.len()
        ).as_bytes()).await.unwrap();
    };
    let poll = flows.poll_device(
        owner,
        &late_id,
        || now.get(),
        || async {
            let tokens = reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .post(token_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            assert_eq!(tokens["expires_in"], 3600);
            AuthDevicePoll::Complete(Ok((
                AuthAccount { service: owner.service, ..account("target") },
                tokens,
            )))
        },
        persist,
    );
    let ((), polled) =
        tokio::time::timeout(Duration::from_secs(10), async { tokio::join!(server, poll) })
            .await
            .expect("loopback exchange should finish");
    assert!(matches!(polled, Ok(AuthFlow::Expired { .. })));
    assert_eq!(
        std::fs::read_dir(root.path()).unwrap().count(),
        0,
        "a token response at the deadline must not persist credentials or metadata"
    );
}
