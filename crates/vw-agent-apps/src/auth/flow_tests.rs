use super::{
    credential_file::{AccountCredential, CredentialFileStore},
    flow::{AuthAccountScope, AuthFlowTiming, AuthFlows},
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
