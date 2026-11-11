use super::{JsonTarget, VerifiedRouteBinding};
use crate::app_routing::{RoutingListener, SharedTransport};
use axum::{Json, Router, http::StatusCode, routing::post};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{net::TcpListener, sync::mpsc};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn aggregation_t11_aggregation_routing_preserves_contract() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/upstream", upstream.local_addr().unwrap());
    let (record, mut requests) = mpsc::unbounded_channel();
    let router = Router::new().route(
        "/upstream",
        post(move |Json(body): Json<Value>| async move {
            record.send(body).unwrap();
            Json(json!({"id": "default-provider"}))
        }),
    );
    let upstream_task = tokio::spawn(async move { axum::serve(upstream, router).await.unwrap() });

    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let binding = VerifiedRouteBinding::new(
        ProjectScope::new(tenant, "project".into()).unwrap(),
        "host".into(),
        AppId::Codex,
        "account".into(),
        1,
    );
    let token = listener
        .auth
        .issue_test_token(
            binding,
            JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: url.parse().unwrap(),
                authorization: "Bearer fixture".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            },
        )
        .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    let send = |model: &str| {
        client
            .post(format!("http://{address}/v1/chat/completions"))
            .bearer_auth(&token)
            .json(&json!({"model": model}))
            .send()
    };

    let response = send("shared-model").await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap(), json!({"id": "default-provider"}));
    assert_eq!(requests.try_recv().unwrap(), json!({"model": "shared-model"}));

    let inactive = send("ccs-member/shared-model").await.unwrap().status();
    let malformed = send("ccs-member").await.unwrap().status();
    let extra_request = requests.try_recv().ok();

    listener.stop().await.unwrap();
    upstream_task.abort();
    assert!(upstream_task.await.unwrap_err().is_cancelled());
    assert_eq!(inactive, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(malformed, StatusCode::BAD_REQUEST);
    assert!(extra_request.is_none(), "reserved IDs must never reach the default provider");
}

#[tokio::test]
async fn aggregation_t11_published_revision_routes_and_revokes_exact_targets() {
    use crate::{
        app_routing::{
            RouteCommitError,
            aggregation_routing::{AggregationModel, AggregationPublication, AggregationTarget},
        },
        gateway::api::handlers::app_routing::{RoutingControl, configure_with_policy},
    };
    use axum::{
        Extension,
        body::Body,
        http::{HeaderMap, Request},
        routing::get,
    };
    use sha2::{Digest, Sha256};
    use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt, sync::Arc};
    use tower::ServiceExt;
    use vw_agent_apps::providers::{
        live::{
            commit::{
                AppState, ExecutionHostId, Mode, ModeState, PendingTarget, ProviderTransactions,
            },
            stage::FileChange,
            write_guard::{FilePrivacy, WriteError, lock_app},
        },
        mode_state::StackState,
    };
    use vw_api_types::app_providers::validation::{
        ProviderRequestOptions, ProviderRequestOverrides,
    };

    let (record, mut requests) = mpsc::unbounded_channel();
    let mut upstreams = Vec::new();
    for provider in ["alpha", "beta"] {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/upstream", upstream.local_addr().unwrap());
        let record = record.clone();
        let router = Router::new().route(
            "/upstream",
            post(move |headers: axum::http::HeaderMap, Json(body): Json<Value>| async move {
                record.send((provider, headers["authorization"].clone(), body.clone())).unwrap();
                let status = if body["model"] == "unavailable" {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    StatusCode::OK
                };
                (status, Json(json!({"id": provider})))
            }),
        );
        let task = tokio::spawn(async move { axum::serve(upstream, router).await.unwrap() });
        upstreams.push((url, task));
    }

    let listener = Arc::new(RoutingListener::default());
    let address = listener.start(0).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let binding = |tenant, project: &str, host: &str, app, account: &str, generation| {
        VerifiedRouteBinding::new(
            ProjectScope::new(tenant, project.into()).unwrap(),
            host.into(),
            app,
            account.into(),
            generation,
        )
    };
    let owned = |generation| {
        binding(tenant.clone(), "project", "host", AppId::Codex, "account", generation)
    };
    let snapshot = |revision: &str, generation| {
        let model = |provider_id: &str, upstream_model: &str| AggregationModel {
            id: format!("ccs-{provider_id}/{upstream_model}"),
            provider_id: provider_id.into(),
            upstream_model: upstream_model.into(),
        };
        let target = |index: usize, provider: &str| AggregationTarget {
            messages_api_key: false,
            binding: owned(generation),
            revision: revision.into(),
            provider_id: provider.into(),
            target: JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: upstreams[index].0.parse().unwrap(),
                authorization: format!("Bearer fixture-{provider}").parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: ProviderRequestOptions {
                    local_proxy_request_overrides: (provider == "beta").then(|| {
                        ProviderRequestOverrides {
                            body: json!({"model": "wrong-model"}).as_object().cloned(),
                            ..Default::default()
                        }
                    }),
                    ..Default::default()
                },
            },
        };
        AggregationPublication {
            binding: owned(generation),
            revision: revision.into(),
            state: AppState {
                mode: Some(ModeState {
                    mode: Mode::Proxy,
                    attached: true,
                    proxy_route: Some("alpha".into()),
                    contract: None,
                }),
                mode_tenant: Some(tenant.clone()),
                stack: StackState {
                    enabled: true,
                    members: ["alpha", "beta"].map(String::from).into(),
                    keys: ["alpha", "beta"].map(|key| (key.into(), key.into())).into(),
                },
                stack_tenant: Some(tenant.clone()),
                ..Default::default()
            },
            models: vec![
                model("alpha", "shared-model"),
                model("beta", "shared-model"),
                model("beta", "unavailable"),
            ],
            default: target(0, "alpha"),
            members: vec![target(1, "beta")],
        }
    };

    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let policy_path = root.path().join("access.json");
    let manager = "fixture-aggregation-manager-token";
    let policy = json!({
        "hosts": [{
            "id": "host", "online": true, "binding": "local",
            "home": root.path(), "search_paths": [], "applications": root.path().join("Applications"),
            "credential_root": root.path(),
        }],
        "principals": [{
            "id": "fixture-manager", "token_sha256": hex::encode(Sha256::digest(manager)),
            "grants": [{
                "platform_id": "platform", "company_id": "company", "host_id": "host",
                "routing_read": true, "routing_manage": true, "provider_config_write": true,
                "routing_apps": [{"project_id": "project", "app_id": "codex"}],
            }],
        }],
    });
    fs::write(&policy_path, policy.to_string()).unwrap();
    fs::set_permissions(&policy_path, fs::Permissions::from_mode(0o600)).unwrap();
    let (send_control, mut receive_control) = mpsc::unbounded_channel();
    let router = configure_with_policy(
        Router::new().route(
            "/",
            get(move |Extension(control): Extension<RoutingControl>| async move {
                send_control.send(control).unwrap();
            }),
        ),
        &listener,
        policy_path,
        crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home).unwrap(),
    )
    .await;
    assert_eq!(router.oneshot(Request::new(Body::empty())).await.unwrap().status(), StatusCode::OK);
    let control = receive_control.try_recv().unwrap();
    let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("x-agent-apps-token", manager.parse().unwrap());
    let foreign_tenant =
        ProjectScope::new(TenantScope::new("platform", "other").unwrap(), "project".into())
            .unwrap();
    let foreign_project = ProjectScope::new(tenant.clone(), "other".into()).unwrap();
    for (scope, host) in
        [(&foreign_tenant, "host"), (&foreign_project, "host"), (&project, "other")]
    {
        assert_eq!(
            control
                .publish_aggregation(
                    &headers,
                    scope,
                    host,
                    None,
                    |_, _| panic!("unauthorized caller reached preparation"),
                    |_, _, _| panic!("unauthorized caller reached commit"),
                )
                .err(),
            Some(StatusCode::FORBIDDEN)
        );
    }
    let publish =
        |candidate, expected, commit: &mut dyn FnMut(&str) -> Result<(), RouteCommitError>| {
            control.publish_aggregation(
                &headers,
                &project,
                "host",
                expected,
                |authorized_root, principal| {
                    assert_eq!(authorized_root, root.path());
                    assert_eq!(principal, "fixture-manager");
                    Ok(candidate)
                },
                |authorized_root, actual, token| {
                    assert_eq!(authorized_root, root.path());
                    assert_eq!(actual, address);
                    commit(token)
                },
            )
        };

    for foreign in [
        binding(
            TenantScope::new("platform", "other").unwrap(),
            "project",
            "host",
            AppId::Codex,
            "account",
            2,
        ),
        binding(tenant.clone(), "other", "host", AppId::Codex, "account", 2),
        binding(tenant.clone(), "project", "other", AppId::Codex, "account", 2),
        binding(tenant.clone(), "project", "host", AppId::Claude, "account", 2),
        binding(tenant.clone(), "project", "host", AppId::Codex, "other", 2),
        owned(1),
    ] {
        let mut candidate = snapshot("revision-1", 2);
        candidate.members[0].binding = foreign;
        assert_eq!(
            publish(candidate, None, &mut |_| panic!("invalid member reached commit")).err(),
            Some(StatusCode::FORBIDDEN)
        );
    }
    let mut stale_member = snapshot("revision-1", 2);
    stale_member.members[0].revision = "stale".into();
    assert_eq!(
        publish(stale_member, None, &mut |_| panic!("stale member reached commit")).err(),
        Some(StatusCode::CONFLICT)
    );

    let native = tempfile::tempdir().unwrap();
    let mut guard = lock_app(AppId::Codex, native.path()).unwrap();
    let transactions =
        ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let client_config = native.path().join("fixture.json");
    let mut commit = |token: &str, state: &AppState, preimage_matches: bool| {
        let change = |_: Option<&[u8]>| Ok(token.as_bytes().to_vec());
        transactions
            .commit_checked(
                &tenant,
                &mut guard,
                &[FileChange {
                    name: OsStr::new("fixture.json"),
                    privacy: FilePrivacy::Private,
                    patch: &change,
                }],
                PendingTarget {
                    mode: state.mode.clone(),
                    stack: Some(state.stack.clone()),
                    ..Default::default()
                },
                || if preimage_matches { Ok(()) } else { Err(WriteError::Conflict) },
            )
            .map_err(|_| RouteCommitError {
                status: StatusCode::CONFLICT,
                recovery_pending: !matches!(transactions.pending(AppId::Codex), Ok(None)),
            })
    };
    let first_publication = snapshot("revision-1", 2);
    let first_state = first_publication.state.clone();
    let mut first = String::new();
    let first_handle = publish(first_publication, None, &mut |token| {
        first = token.into();
        commit(token, &first_state, true)
    })
    .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    let send = |token: &str, model: &str| {
        client
            .post(format!("http://{address}/v1/chat/completions"))
            .bearer_auth(token)
            .json(&json!({"model": model}))
            .send()
    };

    for (id, provider, upstream_model, status) in [
        ("shared-model", "alpha", "shared-model", StatusCode::OK),
        ("ccs-alpha/shared-model", "alpha", "shared-model", StatusCode::OK),
        ("ccs-beta/shared-model", "beta", "shared-model", StatusCode::OK),
        ("ccs-beta/unavailable", "beta", "unavailable", StatusCode::BAD_GATEWAY),
    ] {
        let response = send(&first, id).await.unwrap();
        assert_eq!(response.status(), status);
        if status.is_success() {
            assert_eq!(response.json::<Value>().await.unwrap(), json!({"id": provider}));
        }
        let (actual, authorization, body) = requests.try_recv().unwrap();
        assert_eq!(actual, provider);
        assert_eq!(authorization.to_str().unwrap(), format!("Bearer fixture-{provider}"));
        assert_eq!(body, json!({"model": upstream_model}));
    }
    assert_eq!(send(&first, "ccs-beta/unpublished").await.unwrap().status(), StatusCode::NOT_FOUND);

    assert!(transactions.state(AppId::Codex).unwrap() == first_state);
    assert!(fs::read_to_string(&client_config).unwrap() == first);
    let mut rolled_back = String::new();
    assert_eq!(
        publish(snapshot("revision-2", 2), Some("revision-1"), &mut |token| {
            rolled_back = token.into();
            commit(token, &first_state, false)
        })
        .err(),
        Some(StatusCode::CONFLICT)
    );
    assert!(fs::read_to_string(&client_config).unwrap() == first);
    assert!(transactions.state(AppId::Codex).unwrap() == first_state);
    assert_eq!(
        send(&rolled_back, "ccs-beta/shared-model").await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(send(&first, "ccs-beta/shared-model").await.unwrap().status(), StatusCode::OK);
    assert_eq!(requests.try_recv().unwrap().0, "beta");

    for (expected, generation) in [(Some("stale"), 2), (Some("revision-1"), 1)] {
        assert_eq!(
            publish(snapshot("revision-2", generation), expected, &mut |_| {
                panic!("stale publication reached commit")
            })
            .err(),
            Some(StatusCode::CONFLICT)
        );
    }
    let mut next = String::new();
    let next_handle = publish(snapshot("revision-2", 3), Some("revision-1"), &mut |token| {
        next = token.into();
        commit(token, &first_state, true)
    })
    .unwrap();
    assert!(fs::read_to_string(client_config).unwrap() == next);
    listener.revoke_aggregation(&first_handle).unwrap();
    assert_eq!(
        send(&first, "ccs-beta/shared-model").await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(send(&next, "ccs-beta/shared-model").await.unwrap().status(), StatusCode::OK);
    assert_eq!(requests.try_recv().unwrap().0, "beta");
    listener.revoke_aggregation(&next_handle).unwrap();
    assert_eq!(
        send(&next, "ccs-beta/shared-model").await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let mut previous = String::new();
    let _previous_handle = publish(snapshot("revision-3", 3), None, &mut |token| {
        previous = token.into();
        commit(token, &first_state, true)
    })
    .unwrap();
    let mut pending = String::new();
    assert_eq!(
        publish(snapshot("revision-4", 3), Some("revision-3"), &mut |token| {
            pending = token.into();
            Err(RouteCommitError { status: StatusCode::CONFLICT, recovery_pending: true })
        })
        .err(),
        Some(StatusCode::CONFLICT)
    );
    for token in [previous, pending] {
        assert_eq!(
            send(&token, "ccs-beta/shared-model").await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert!(requests.try_recv().is_err());

    listener.stop().await.unwrap();
    for (_, task) in upstreams {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }
}
