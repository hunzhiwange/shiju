use super::app_routing::lifecycle_api_tests::{HOST, MANAGER, spawn_gateway, write_policy};
use crate::{
    app_providers::AppProviderStore,
    gateway::agent_apps_high_availability_store::FailoverQueueStore,
    global_database::GlobalDatabase,
};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use std::{fs, time::Duration};
use vw_api_types::{
    agent_apps::AppId::Codex,
    agent_apps_high_availability::{FailoverQueueScope, FailoverQueueVersion},
    app_providers::{AppProviderWrite, ProviderProtocol, codex_catalog::CodexProfile},
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn ha_api_queue_read_enforces_scope() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let mut grants: Value = serde_json::from_slice(&fs::read(&policy)?)?;
    grants["principals"][0]["grants"][0]["routing_apps"] = json!([
        {"project_id": "project", "app_id": "codex"},
        {"project_id": "project", "app_id": "pi"}
    ]);
    fs::write(&policy, grants.to_string())?;

    let db = GlobalDatabase::for_home(home.path())?;
    let queue = FailoverQueueStore::new(db.clone())?;
    let providers = AppProviderStore::new(db).map_err(anyhow::Error::msg)?;
    let tenant = TenantScope::new("p", "owner")?;
    let other = TenantScope::new("p", "other")?;
    let scope = FailoverQueueScope {
        project: ProjectScope::new(tenant.clone(), "project".into())?,
        target_host_id: HOST.into(),
        app_id: Codex,
    };
    let provider = AppProviderWrite {
        name: "Custom".into(),
        protocol: ProviderProtocol::OpenaiResponses,
        api_key: "fixture-api-key".into(),
        base_url: "https://fixture.example.test/v1".into(),
        model: "fixture-model".into(),
        request_options: None,
    };
    for (owner, id) in [
        (&tenant, "queued-a"),
        (&tenant, "queued-b"),
        (&tenant, "candidate"),
        (&other, "foreign-tenant"),
    ] {
        providers.save(owner, Codex, id, &provider).unwrap();
    }
    providers
        .codex_save(
            &tenant,
            "official",
            &CodexProfile {
                name: "Official".into(),
                config: "model = \"fixture-model\"".into(),
                api_key: None,
                preserve_login: false,
            },
            None,
            None,
        )
        .unwrap();
    let snapshot = providers.failover_providers(&scope.project, Codex).unwrap();
    let saved = queue.update(&scope, FailoverQueueVersion(0), |members, enabled| {
        for id in ["queued-b", "queued-a"] {
            members.add(id, &snapshot).unwrap();
        }
        *enabled = true;
        Ok(())
    })?;

    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let client = Client::builder().no_proxy().timeout(Duration::from_secs(5)).build()?;
    let request = |resource, company, project, host, app, token| {
        let request = client
            .get(format!("http://127.0.0.1:{port}/v1/agent-apps/high-availability/{resource}"))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .query(&[("projectId", project), ("targetHostId", host), ("appId", app)]);
        match token {
            Some(token) => request.header("x-agent-apps-token", token),
            None => request,
        }
    };
    for resource in ["queue", "candidates"] {
        for (company, project, host, app, token, expected) in [
            ("owner", "project", HOST, "codex", None, StatusCode::UNAUTHORIZED),
            (
                "owner",
                "project",
                HOST,
                "codex",
                Some("invalid-fixture-token-000000000000"),
                StatusCode::UNAUTHORIZED,
            ),
            ("other", "project", HOST, "codex", Some(MANAGER), StatusCode::FORBIDDEN),
            ("owner", "foreign", HOST, "codex", Some(MANAGER), StatusCode::FORBIDDEN),
            ("owner", "project", "other-mac", "codex", Some(MANAGER), StatusCode::FORBIDDEN),
            ("owner", "project", HOST, "claude", Some(MANAGER), StatusCode::FORBIDDEN),
            ("owner", "project", HOST, "unknown", Some(MANAGER), StatusCode::BAD_REQUEST),
            ("owner", "project", HOST, "pi", Some(MANAGER), StatusCode::CONFLICT),
        ] {
            let response = request(resource, company, project, host, app, token).send().await?;
            assert_eq!(response.status(), expected, "{resource} {company}/{project}/{host}/{app}");
        }
        let response =
            request(resource, "owner", "project", HOST, "codex", Some(MANAGER)).send().await?;
        assert_eq!(response.status(), StatusCode::OK, "{resource}");
        assert_eq!(response.headers()["cache-control"], "no-store");
        let payload = response.json::<Value>().await?;
        let public_entry = |id| json!({"providerId": id, "name": "Custom"});
        let expected = if resource == "queue" {
            json!({
                "scope": scope, "version": 1, "enabled": true,
                "entries": [public_entry("queued-b"), public_entry("queued-a")],
            })
        } else {
            json!({"scope": scope, "candidates": [public_entry("candidate")]})
        };
        assert_eq!(payload, expected);
    }
    assert_eq!(queue.get(&scope)?, saved);
    Ok(())
}

#[tokio::test]
async fn ha_api_queue_write_rejects_conflicts() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let mut grants: Value = serde_json::from_slice(&fs::read(&policy)?)?;
    grants["principals"][0]["grants"][0]["routing_apps"] = json!([
        {"project_id": "project", "app_id": "codex"}
    ]);
    grants["principals"][0]["grants"][0]["provider_config_write"] = json!(true);
    fs::write(&policy, grants.to_string())?;
    let db = GlobalDatabase::for_home(home.path())?;
    let tenant = TenantScope::new("p", "owner")?;
    let scope = FailoverQueueScope {
        project: ProjectScope::new(tenant.clone(), "project".into())?,
        target_host_id: HOST.into(),
        app_id: Codex,
    };
    let queue = FailoverQueueStore::new(db.clone())?;
    let providers = AppProviderStore::new(db).map_err(anyhow::Error::msg)?;
    for id in ["first", "second", "third"] {
        providers
            .save(
                &tenant,
                Codex,
                id,
                &AppProviderWrite {
                    name: id.into(),
                    protocol: ProviderProtocol::OpenaiResponses,
                    api_key: "fixture-key".into(),
                    base_url: "https://fixture.example.test/v1".into(),
                    model: "fixture".into(),
                    request_options: None,
                },
            )
            .map_err(anyhow::Error::msg)?;
    }
    providers
        .codex_save(
            &tenant,
            "official",
            &CodexProfile {
                name: "Official".into(),
                config: "model = \"fixture\"".into(),
                api_key: None,
                preserve_login: false,
            },
            None,
            None,
        )
        .map_err(anyhow::Error::msg)?;
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let client = Client::builder().no_proxy().timeout(Duration::from_secs(10)).build()?;
    let request = |method, resource: &str, body: Value, token: Option<&str>| {
        let request = client
            .request(
                method,
                format!("http://127.0.0.1:{port}/v1/agent-apps/high-availability/{resource}"),
            )
            .header("x-platform-id", "p")
            .header("x-company-id", "owner")
            .query(&[("projectId", "project"), ("targetHostId", HOST), ("appId", "codex")])
            .json(&body);
        match token {
            Some(token) => request.header("x-agent-apps-token", token),
            None => request,
        }
    };
    let read = client
        .get(format!("http://127.0.0.1:{port}/v1/agent-apps/high-availability/queue"))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner")
        .header("x-agent-apps-token", MANAGER)
        .query(&[("projectId", "project"), ("targetHostId", HOST), ("appId", "codex")])
        .send()
        .await?;
    assert_eq!(
        read.status(),
        StatusCode::OK,
        "same principal and scope must read successfully before testing writes"
    );
    assert_eq!(read.json::<Value>().await?["version"], 0);
    use reqwest::Method;
    let member = |version, id| json!({"expectedVersion":version,"providerId":id});
    for token in [None, Some("invalid-fixture-token-000000000000")] {
        let response =
            request(Method::POST, "queue/members", member(0, "first"), token).send().await?;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "queue writes require authentication"
        );
        assert_eq!(queue.get(&scope)?.version.0, 0);
    }
    let response =
        request(Method::POST, "queue/members", member(0, "first"), Some(MANAGER)).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.json::<Value>().await?["version"], 1);
    let first = queue.get(&scope)?;
    for (company, project, host, app) in [
        ("other", "project", HOST, "codex"),
        ("owner", "foreign", HOST, "codex"),
        ("owner", "project", "other-mac", "codex"),
        ("owner", "project", HOST, "claude"),
    ] {
        let response = client
            .post(format!("http://127.0.0.1:{port}/v1/agent-apps/high-availability/queue/members"))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .header("x-agent-apps-token", MANAGER)
            .query(&[("projectId", project), ("targetHostId", host), ("appId", app)])
            .json(&member(1, "second"))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(queue.get(&scope)?, first);
    }
    // A replay is idempotent only at the current revision.
    let response =
        request(Method::POST, "queue/members", member(1, "first"), Some(MANAGER)).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await?["version"], 1);
    assert_eq!(queue.get(&scope)?, first);
    for (body, expected) in [
        (member(0, "second"), StatusCode::CONFLICT),
        (member(1, "official"), StatusCode::CONFLICT),
        (member(1, "missing"), StatusCode::NOT_FOUND),
        (json!({"expectedVersion":1,"providerId":"first","sql":"delete"}), StatusCode::BAD_REQUEST),
    ] {
        let response = request(Method::POST, "queue/members", body, Some(MANAGER)).send().await?;
        assert_eq!(response.status(), expected);
        assert_eq!(queue.get(&scope)?, first);
    }
    // Existing grants are reloaded: routing alone cannot edit providers, and
    // provider editing alone cannot change routing configuration.
    let original = fs::read(&policy)?;
    for field in ["provider_config_write", "routing_manage"] {
        let mut denied = grants.clone();
        denied["principals"][0]["grants"][0][field] = json!(false);
        fs::write(&policy, denied.to_string())?;
        for (method, resource, body) in [
            (Method::POST, "queue/members", member(1, "second")),
            (Method::DELETE, "queue/members", member(1, "first")),
            (Method::PUT, "queue/order", json!({"expectedVersion":1,"providerIds":["first"]})),
            (Method::PUT, "enabled", json!({"expectedVersion":1,"enabled":false})),
        ] {
            let response = request(method, resource, body, Some(MANAGER)).send().await?;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{field}/{resource}");
            assert_eq!(queue.get(&scope)?, first);
        }
    }
    fs::write(&policy, original)?;
    // Two actual HTTP requests race the same persisted compare-and-swap revision.
    let (left, right) = tokio::join!(
        request(Method::POST, "queue/members", member(1, "second"), Some(MANAGER)).send(),
        request(Method::POST, "queue/members", member(1, "third"), Some(MANAGER)).send()
    );
    let left = left?;
    let right = right?;
    let statuses = [left.status(), right.status()];
    assert_eq!(statuses.iter().filter(|s| **s == StatusCode::OK).count(), 1);
    assert_eq!(statuses.iter().filter(|s| **s == StatusCode::CONFLICT).count(), 1);
    let conflict = if left.status() == StatusCode::CONFLICT { left } else { right };
    assert_eq!(conflict.json::<Value>().await?["error"]["code"], "version_conflict");
    let saved = queue.get(&scope)?;
    assert_eq!(saved.version.0, 2);
    assert_eq!(saved.members.len(), 2);
    let response = request(
        Method::PUT,
        "queue/order",
        json!({"expectedVersion":2,"providerIds":["first","first"]}),
        Some(MANAGER),
    )
    .send()
    .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(queue.get(&scope)?, saved);
    let mut reversed = saved.members.clone();
    reversed.reverse();
    let response = request(
        Method::PUT,
        "queue/order",
        json!({"expectedVersion":2,"providerIds":reversed}),
        Some(MANAGER),
    )
    .send()
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response.json::<Value>().await?;
    assert_eq!(payload["version"], 3);
    assert_eq!(payload["entries"][0]["providerId"], reversed[0]);
    assert_eq!(queue.get(&scope)?.members, reversed);
    let before = queue.get(&scope)?;
    let response =
        request(Method::PUT, "enabled", json!({"expectedVersion":3,"enabled":true}), Some(MANAGER))
            .send()
            .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT, "unpublished mode cannot enable failover");
    assert_eq!(queue.get(&scope)?, before, "failed enablement must not change queue or flag");
    let response = request(
        Method::PUT,
        "enabled",
        json!({"expectedVersion":3,"enabled":false}),
        Some(MANAGER),
    )
    .send()
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(queue.get(&scope)?, before);
    let response = request(Method::DELETE, "queue/members", member(3, &reversed[0]), Some(MANAGER))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await?["version"], 4);
    assert_eq!(queue.get(&scope)?.members, ["first"]);
    Ok(())
}

#[tokio::test]
async fn ha_api_queue_write_rejects_conflicts_enabled() -> anyhow::Result<()> {
    use crate::gateway::{
        agent_apps_high_availability_store::{FailoverQueueState, NAMESPACE, config_key},
        settings_store::GatewaySettingsStore,
    };
    use std::os::unix::fs::PermissionsExt;
    use vw_api_types::{
        agent_apps::AppId::Claude,
        app_providers::claude::{ClaudeConfig, ClaudeProfile},
    };
    let home = tempfile::tempdir()?;
    let root = home.path().join("credentials");
    fs::create_dir(&root)?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    fs::create_dir(home.path().join(".claude"))?;
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let mut grants: Value = serde_json::from_slice(&fs::read(&policy)?)?;
    grants["principals"][0]["grants"][0]["routing_apps"] = json!([
        {"project_id":"project","app_id":"claude"}
    ]);
    grants["principals"][0]["grants"][0]["provider_config_write"] = json!(true);
    fs::write(&policy, grants.to_string())?;
    let db = GlobalDatabase::for_home(home.path())?;
    let tenant = TenantScope::new("p", "owner")?;
    let scope = FailoverQueueScope {
        project: ProjectScope::new(tenant.clone(), "project".into())?,
        target_host_id: HOST.into(),
        app_id: Claude,
    };
    AppProviderStore::new(db.clone())
        .map_err(anyhow::Error::msg)?
        .claude_save(
            &tenant,
            "current",
            &ClaudeProfile {
                name: "Current".into(),
                config: ClaudeConfig::ThirdParty {
                    base_url: "https://fixture.example.test".into(),
                    api_key: "fixture-key".into(),
                    api_key_header: true,
                },
                model: Some("fixture".into()),
            },
            None,
            true,
        )
        .map_err(anyhow::Error::msg)?;
    // Establish the same real direct-provider transaction used by the native
    // mode owner before asking the HTTP API to publish a routing mode.
    {
        use vw_agent_apps::providers::live::{
            commit::{ExecutionHostId, PendingTarget, ProviderTransactions},
            write_guard::lock_app,
        };
        let transactions =
            ProviderTransactions::new(&root, ExecutionHostId::new(HOST).unwrap()).unwrap();
        let mut guard = lock_app(Claude, &home.path().join(".claude")).unwrap();
        transactions
            .commit(
                &tenant,
                &mut guard,
                &[],
                PendingTarget { pointer: Some("current".into()), ..Default::default() },
            )
            .unwrap();
    }
    let queue = FailoverQueueStore::new(db.clone())?;
    let settings = GatewaySettingsStore::new(db)?;
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let client = Client::builder().no_proxy().timeout(Duration::from_secs(15)).build()?;
    let request = |method, path: &str, body: Value| {
        client
            .request(method, format!("http://127.0.0.1:{port}/v1/{path}"))
            .header("x-platform-id", "p")
            .header("x-company-id", "owner")
            .header("x-agent-apps-token", MANAGER)
            .json(&body)
    };
    use reqwest::Method;
    let enabled = |version, flag| {
        request(
            Method::PUT,
            "agent-apps/high-availability/enabled",
            json!({"expectedVersion":version,"enabled":flag}),
        )
        .query(&[("projectId", "project"), ("targetHostId", HOST), ("appId", "claude")])
    };
    assert_eq!(enabled(0, true).send().await?.status(), StatusCode::CONFLICT);
    assert_eq!(queue.get(&scope)?.version.0, 0);
    assert_eq!(
        request(Method::POST, &format!("app-routing/hosts/{HOST}/start"), json!({}))
            .send()
            .await?
            .status(),
        StatusCode::OK
    );
    let applied = request(Method::POST, "app-providers/claude/mode/apply", json!({
        "targetHostId":HOST, "projectId":"project", "action":{"kind":"proxy","providerId":"current"}
    })).send().await?;
    let status = applied.status();
    let body = applied.text().await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let native = home.path().join(".claude/settings.json");
    let configuration = fs::read(&native)?;
    // Fail after both tentative fields changed: no partial state may persist.
    let exhausted = FailoverQueueState {
        version: FailoverQueueVersion(u64::MAX),
        enabled: false,
        members: vec![],
    };
    settings.upsert(
        &tenant,
        NAMESPACE,
        &config_key(&scope)?,
        &serde_json::to_string(&exhausted)?,
    )?;
    assert_eq!(enabled(u64::MAX, true).send().await?.status(), StatusCode::CONFLICT);
    assert_eq!(queue.get(&scope)?, exhausted);
    settings.delete(&tenant, NAMESPACE, &config_key(&scope)?)?;
    let response = enabled(0, true).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response.json::<Value>().await?;
    assert_eq!(payload["version"], 1);
    assert_eq!(payload["enabled"], true);
    assert_eq!(payload["entries"], json!([{"providerId":"current","name":"Current"}]));
    let saved = queue.get(&scope)?;
    assert!(saved.enabled);
    assert_eq!(saved.members, ["current"]);
    assert_eq!(enabled(0, false).send().await?.status(), StatusCode::CONFLICT);
    assert_eq!(queue.get(&scope)?, saved);
    assert_eq!(enabled(1, false).send().await?.status(), StatusCode::OK);
    let disabled = queue.get(&scope)?;
    assert!(!disabled.enabled);
    assert_eq!(disabled.version.0, 2);
    assert_eq!(disabled.members, saved.members);
    assert_eq!(fs::read(native)?, configuration);
    Ok(())
}

#[tokio::test]
async fn ha_api_health_and_policy_are_scope_bound() -> anyhow::Result<()> {
    use crate::gateway::agent_apps_failover_runtime;
    use reqwest::Method;
    let home = tempfile::tempdir()?;
    let policy_file = home.path().join("access.json");
    write_policy(&policy_file, home.path(), 0);
    let mut grants: Value = serde_json::from_slice(&fs::read(&policy_file)?)?;
    for principal in [0, 1] {
        grants["principals"][principal]["grants"][0]["routing_apps"] =
            json!([{"project_id":"project", "app_id":"codex"}]);
    }
    grants["principals"][0]["grants"][0]["provider_config_write"] = json!(true);
    fs::write(&policy_file, grants.to_string())?;
    let database = GlobalDatabase::for_home(home.path())?;
    let tenant = TenantScope::new("p", "owner")?;
    let scope = FailoverQueueScope {
        project: ProjectScope::new(tenant.clone(), "project".into())?,
        target_host_id: HOST.into(), app_id: Codex,
    };
    AppProviderStore::new(database.clone()).map_err(anyhow::Error::msg)?.save(
        &tenant, Codex, "provider", &AppProviderWrite {
            name: "Custom".into(), protocol: ProviderProtocol::OpenaiResponses,
            api_key: "fixture-key".into(), base_url: "https://fixture.example.test/v1".into(),
            model: "fixture".into(), request_options: None,
        },
    ).map_err(anyhow::Error::msg)?;
    let original = agent_apps_failover_runtime::policy(database.clone(), &scope)?;
    let (_gateway, port) = spawn_gateway(home.path(), &policy_file).await;
    let client = Client::builder().no_proxy().timeout(Duration::from_secs(10)).build()?;
    let request = |method, resource: &str, host: &str, token: &str| {
        let mut request = client.request(method,
            format!("http://127.0.0.1:{port}/v1/agent-apps/high-availability/{resource}"))
            .header("x-platform-id", "p").header("x-company-id", "owner")
            .header("x-agent-apps-token", token)
            .query(&[("projectId", "project"), ("targetHostId", host), ("appId", "codex")]);
        if resource.starts_with("health") {
            request = request.query(&[("providerId", "provider")]);
        }
        request
    };
    let reader = "fixture-routing-reader-token-000002";
    for resource in ["health", "policy", "route"] {
        let response = request(Method::GET, resource, HOST, reader).send().await?;
        assert_eq!(response.status(), StatusCode::OK, "{resource}");
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = response.json::<Value>().await?;
        if resource == "health" {
            assert_eq!(body["state"], "unknown");
            assert_eq!(body["providerId"], "provider");
            assert_eq!(body["scope"], json!(scope));
        } else if resource == "route" {
            assert!(body["actualProviderId"].is_null(), "reads must not create a route");
        } else {
            assert_eq!(body, json!(original));
        }
        for token in [MANAGER, "invalid-fixture-token-000000000000"] {
            let response = request(Method::GET, resource, "other-mac", token).send().await?;
            assert!(matches!(response.status(), StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED));
        }
    }
    let mut updated = original;
    updated.circuit_breaker.failure_threshold = 7;
    updated.retry_timeout.first_byte_timeout_secs = 30;
    for token in [reader, "invalid-fixture-token-000000000000"] {
        let response = request(Method::PUT, "policy", HOST, token).json(&updated).send().await?;
        assert!(matches!(response.status(), StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED));
        assert_eq!(agent_apps_failover_runtime::policy(database.clone(), &scope)?, original);
        let response = request(Method::POST, "health/reset", HOST, token).send().await?;
        assert!(matches!(response.status(), StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED));
    }
    for (method, resource) in [(Method::PUT, "policy"), (Method::POST, "health/reset")] {
        let response = request(method, resource, "other-mac", MANAGER).json(&updated).send().await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(agent_apps_failover_runtime::policy(database.clone(), &scope)?, original);
    }
    let mut invalid = updated;
    invalid.circuit_breaker.failure_threshold = 0;
    assert_eq!(request(Method::PUT, "policy", HOST, MANAGER).json(&invalid)
        .send().await?.status(), StatusCode::BAD_REQUEST);
    assert_eq!(agent_apps_failover_runtime::policy(database.clone(), &scope)?, original);
    let response = request(Method::PUT, "policy", HOST, MANAGER).json(&updated).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await?, json!(updated));
    assert_eq!(agent_apps_failover_runtime::policy(database.clone(), &scope)?, updated);
    let response = request(Method::POST, "health/reset", HOST, MANAGER).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await?["state"], "unknown");
    let mut foreign = scope.clone();
    foreign.target_host_id = "other-mac".into();
    assert_eq!(agent_apps_failover_runtime::policy(database, &foreign)?, original);
    Ok(())
}
