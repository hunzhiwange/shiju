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
