use super::{
    RouteCommitError, RoutingListener, SharedTransport, VerifiedRouteBinding,
    aggregation_routing::{AggregationModel, AggregationPublication, AggregationTarget},
    dispatch::JsonTarget,
    failover_routing::FailoverPublication,
};
use crate::{
    gateway::agent_apps_high_availability_store::FailoverQueueStore,
    global_database::GlobalDatabase,
};
use axum::{Json, Router, http::StatusCode, routing::post};
use serde_json::{Value, json};
use std::{ffi::OsStr, time::Duration};
use tokio::{net::TcpListener, sync::mpsc};
use vw_agent_apps::providers::{
    failover::{ProviderAuth, ProviderSnapshot},
    live::{
        commit::{AppState, ExecutionHostId, Mode, ModeState, PendingTarget, ProviderTransactions},
        stage::FileChange,
        write_guard::{FilePrivacy, lock_app},
    },
    mode_state::StackState,
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_high_availability::{FailoverQueueScope, FailoverQueueVersion},
    app_providers::AppProviderSummary,
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn aggregation_t12_listener_preserves_queue_and_resumes_ordered_selection() {
    use std::os::unix::fs::PermissionsExt;
    let (record, mut requests) = mpsc::unbounded_channel();
    let mut upstreams = Vec::new();
    for id in ["alpha", "beta"] {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/upstream", upstream.local_addr().unwrap());
        let record = record.clone();
        let router = Router::new().route(
            "/upstream",
            post(move |Json(body): Json<Value>| async move {
                record.send((id, body.clone())).unwrap();
                let status = if body["model"] == "unavailable" {
                    StatusCode::BAD_GATEWAY
                } else {
                    StatusCode::OK
                };
                (status, Json(json!({"provider": id})))
            }),
        );
        let task = tokio::spawn(async move { axum::serve(upstream, router).await.unwrap() });
        upstreams.push((url, task));
    }
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let queue_store =
        FailoverQueueStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
    let scope = FailoverQueueScope {
        project: project.clone(),
        target_host_id: "host".into(),
        app_id: AppId::Codex,
    };
    let providers = || -> Vec<ProviderSnapshot> {
        ["alpha", "beta"]
            .map(|id| ProviderSnapshot {
                provider: AppProviderSummary {
                    id: id.into(),
                    app_id: AppId::Codex,
                    name: id.into(),
                    configured: true,
                },
                auth: ProviderAuth::ThirdParty,
            })
            .into()
    };
    let saved = queue_store
        .update(&scope, FailoverQueueVersion(0), |queue, enabled| {
            for id in ["beta", "alpha"] {
                queue.add(id, &providers()).map_err(|error| anyhow::anyhow!("{error:?}"))?;
            }
            *enabled = true;
            Ok(())
        })
        .unwrap();
    let binding = || {
        VerifiedRouteBinding::new(project.clone(), "host".into(), AppId::Codex, "account".into(), 1)
    };
    let target = |index: usize, id: &str, revision: &str| AggregationTarget {
        binding: binding(),
        revision: revision.into(),
        provider_id: id.into(),
        target: JsonTarget {
            url: upstreams[index].0.parse().unwrap(),
            authorization: "Bearer fixture".parse().unwrap(),
            transport: SharedTransport::new().unwrap(),
            request_options: Default::default(),
        },
    };
    let mut state = AppState {
        mode: Some(ModeState {
            mode: Mode::Proxy,
            attached: true,
            proxy_route: Some("alpha".into()),
            contract: None,
        }),
        mode_tenant: Some(tenant.clone()),
        stack: StackState {
            enabled: true,
            members: vec!["alpha".into(), "beta".into()],
            keys: [("alpha".into(), "alpha".into()), ("beta".into(), "beta".into())].into(),
        },
        stack_tenant: Some(tenant.clone()),
        ..Default::default()
    };
    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let native = tempfile::tempdir().unwrap();
    let mut guard = lock_app(AppId::Codex, native.path()).unwrap();
    let transactions =
        ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let mut commit = |token: &str, state: &AppState| {
        let patch = |_: Option<&[u8]>| Ok(token.as_bytes().to_vec());
        transactions
            .commit_checked(
                &tenant,
                &mut guard,
                &[FileChange {
                    name: OsStr::new("client.json"),
                    privacy: FilePrivacy::Private,
                    patch: &patch,
                }],
                PendingTarget {
                    mode: state.mode.clone(),
                    stack: Some(state.stack.clone()),
                    ..Default::default()
                },
                || Ok(()),
            )
            .map_err(|_| RouteCommitError { status: StatusCode::CONFLICT, recovery_pending: true })
    };
    let mut aggregate_token = String::new();
    listener
        .publish_aggregation(
            AggregationPublication {
                binding: binding(),
                revision: "aggregate-1".into(),
                state: state.clone(),
                models: vec![AggregationModel {
                    id: "ccs-beta/unavailable".into(),
                    provider_id: "beta".into(),
                    upstream_model: "unavailable".into(),
                }],
                default: target(0, "alpha", "aggregate-1"),
                members: vec![target(1, "beta", "aggregate-1")],
            },
            None,
            |_, token| {
                aggregate_token = token.into();
                commit(token, &state)
            },
        )
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
    for (model, expected) in [("unavailable", "alpha"), ("ccs-beta/unavailable", "beta")] {
        assert_eq!(send(&aggregate_token, model).await.unwrap().status(), StatusCode::BAD_GATEWAY);
        assert_eq!(requests.try_recv().unwrap().0, expected);
        assert!(requests.try_recv().is_err(), "aggregation failure must not visit another target");
        assert_eq!(queue_store.get(&scope).unwrap(), saved);
    }
    let plain = |state: &AppState| FailoverPublication {
        binding: binding(),
        revision: "route-1".into(),
        state: state.clone(),
        providers: providers(),
        default: target(0, "alpha", "route-1"),
        members: vec![target(1, "beta", "route-1")],
    };
    assert_eq!(
        listener
            .publish_failover_route(
                plain(&state),
                &queue_store,
                Some("aggregate-1"),
                |_, _, _, _| panic!("aggregation reached commit")
            )
            .err(),
        Some(StatusCode::CONFLICT)
    );
    state.stack.enabled = false;
    // A result started in ordinary routing has no aggregation revision. It
    // cannot replace the aggregation that became active while it was in flight.
    for expected in [None, Some("stale")] {
        assert_eq!(
            listener
                .publish_failover_route(plain(&state), &queue_store, expected, |_, _, _, _| panic!(
                    "stale transition reached commit"
                ))
                .err(),
            Some(StatusCode::CONFLICT)
        );
    }
    let mut plain_token = String::new();
    listener
        .publish_failover_route(
            plain(&state),
            &queue_store,
            Some("aggregate-1"),
            |_, token, queue, provider| {
                assert_eq!(queue, &saved);
                assert_eq!(provider, "beta");
                plain_token = token.into();
                let mut selected = state.clone();
                selected.mode.as_mut().unwrap().proxy_route = Some(provider.into());
                commit(token, &selected)
            },
        )
        .unwrap();
    assert_eq!(
        send(&aggregate_token, "unavailable").await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let response = send(&plain_token, "shared-model").await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap(), json!({"provider": "beta"}));
    assert_eq!(requests.try_recv().unwrap(), ("beta", json!({"model": "shared-model"})));
    assert_eq!(queue_store.get(&scope).unwrap(), saved);
    state.mode.as_mut().unwrap().proxy_route = Some("beta".into());
    assert!(transactions.state(AppId::Codex).unwrap() == state);
    assert_eq!(std::fs::read_to_string(native.path().join("client.json")).unwrap(), plain_token);
    listener.stop().await.unwrap();
    for (_, task) in upstreams {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }
}

#[tokio::test]
async fn aggregation_t12_queue_publication_rejects_unauthorized_scope() {
    use crate::gateway::api::handlers::app_routing::{RoutingControl, configure_with_policy};
    use axum::{
        Extension,
        body::Body,
        http::{HeaderMap, Request},
        routing::get,
    };
    use sha2::{Digest, Sha256};
    use std::{os::unix::fs::PermissionsExt, sync::Arc};
    use tower::ServiceExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("policy.json");
    let policy = json!({
        "hosts": [{"id":"host", "online":true, "binding":"local", "home":root.path(),
            "search_paths":[], "applications":root.path().join("Applications"), "credential_root":root.path()}],
        "principals": [{"id":"manager", "token_sha256":hex::encode(Sha256::digest("fixture-queue-publication-manager-token")),
            "grants":[{"platform_id":"platform", "company_id":"company", "host_id":"host",
                "routing_read":true, "routing_manage":true, "provider_config_write":true,
                "routing_apps":[{"project_id":"project", "app_id":"codex"}]}]}]
    });
    std::fs::write(&path, policy.to_string()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let listener = Arc::new(RoutingListener::default());
    let (send, mut receive) = mpsc::unbounded_channel();
    let router = configure_with_policy(
        Router::new().route(
            "/",
            get(move |Extension(control): Extension<RoutingControl>| async move {
                send.send(control).unwrap();
            }),
        ),
        &listener,
        path,
    )
    .await;
    assert_eq!(router.oneshot(Request::new(Body::empty())).await.unwrap().status(), StatusCode::OK);
    let control = receive.try_recv().unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let mut headers = HeaderMap::new();
    headers
        .insert("x-agent-apps-token", "fixture-queue-publication-manager-token".parse().unwrap());
    for (tenant, project, host) in [
        (TenantScope::new("platform", "other").unwrap(), "project", "host"),
        (tenant.clone(), "other", "host"),
        (tenant.clone(), "project", "other"),
    ] {
        let scope = ProjectScope::new(tenant, project.into()).unwrap();
        assert_eq!(
            control
                .publish_failover_route(
                    &headers,
                    &scope,
                    host,
                    None,
                    |_, _| panic!("unauthorized preparation"),
                    |_, _, _, _, _| panic!("unauthorized commit")
                )
                .err(),
            Some(StatusCode::FORBIDDEN)
        );
    }
}
