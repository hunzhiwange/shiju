use super::{
    agent_apps_high_availability_service::set_enabled,
    agent_apps_high_availability_store::{
        FailoverQueueState, FailoverQueueStore, NAMESPACE, config_key,
    },
    settings_store::GatewaySettingsStore,
};
use crate::{
    app_providers::AppProviderStore,
    app_routing::{RoutingListener, provider_publication},
    global_database::GlobalDatabase,
};
use axum::response::IntoResponse;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use vw_agent_apps::providers::live::{commit::*, write_guard::lock_app};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_high_availability::{FailoverQueueScope, FailoverQueueVersion},
    app_providers::{
        claude::{ClaudeConfig, ClaudeProfile},
        mode_apply::ModeApplyAction,
    },
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn ha_enable_requires_routing_mode_and_keeps_queue() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let root = home.path().join("credentials");
    fs::create_dir(&root)?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    let native = home.path().join(".claude");
    fs::create_dir(&native)?;
    let database = GlobalDatabase::for_home(home.path())?;
    let scope = FailoverQueueScope {
        project: ProjectScope::new(TenantScope::new("p", "owner")?, "project".into())?,
        target_host_id: "host".into(),
        app_id: AppId::Claude,
    };
    let tenant = scope.project.tenant_scope();
    let tx = ProviderTransactions::new(&root, ExecutionHostId::new("host").unwrap()).unwrap();
    let mut guard = lock_app(AppId::Claude, &native).unwrap();
    tx.commit(
        tenant,
        &mut guard,
        &[],
        PendingTarget { pointer: Some("direct".into()), ..Default::default() },
    )
    .unwrap();
    drop(guard);
    let routing = RoutingListener::default();
    let queues = FailoverQueueStore::new(database.clone())?;
    let before = queues.get(&scope)?;
    let update = |version, enabled| {
        set_enabled(database.clone(), &routing, home.path(), &root, &scope, version, enabled)
    };
    assert!(
        update(FailoverQueueVersion(0), true).is_err(),
        "direct or unknown routing mode must reject enablement"
    );
    assert_eq!(queues.get(&scope)?, before);

    let hits = Arc::new(AtomicUsize::new(0));
    let hit_count = hits.clone();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let endpoint = format!("http://{}", upstream.local_addr()?);
    let server = tokio::spawn(async move {
        axum::serve(upstream, axum::Router::new().route("/v1/messages", axum::routing::post(
            move |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<Value>| {
                let hits = hit_count.clone();
                async move {
                    assert_eq!(headers["x-api-key"], "fixture-key");
                    assert_eq!(headers["anthropic-version"], "2023-06-01");
                    assert_eq!(body["model"], "fixture");
                    hits.fetch_add(1, Ordering::SeqCst);
                    if body["stream"] == true {
                        let wire = crate::model_relay::stream_observer::tests::samples()
                            .into_iter().find(|(kind, _, _)| *kind == "anthropic").unwrap().2;
                        let chunks = wire.as_bytes().chunks(7)
                            .map(|chunk| Ok::<_, std::convert::Infallible>(axum::body::Bytes::copy_from_slice(chunk)))
                            .collect::<Vec<_>>();
                        ([(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                            axum::body::Body::from_stream(futures_util::stream::iter(chunks))).into_response()
                    } else {
                        axum::Json(json!({"type":"message","role":"assistant","content":[{"type":"text","text":"OK"}]})).into_response()
                    }
                }
            }
        ))).await.unwrap();
    });
    let store = AppProviderStore::new(database.clone()).map_err(anyhow::Error::msg)?;
    for id in ["direct", "route", "backup"] {
        store
            .claude_save(
                tenant,
                id,
                &ClaudeProfile {
                    name: id.into(),
                    config: ClaudeConfig::ThirdParty {
                        base_url: endpoint.clone(),
                        api_key: "fixture-key".into(),
                        api_key_header: true,
                    },
                    model: Some("fixture".into()),
                },
                None,
                true,
            )
            .map_err(anyhow::Error::msg)?;
    }
    let address = routing.start(0)?;
    let proxy = |id: &str| ModeApplyAction::Proxy { provider_id: id.into() };
    let apply = |action: &ModeApplyAction| {
        provider_publication::apply(database.clone(), &routing, home.path(), &root, &scope, action)
    };
    apply(&proxy("route")).expect("authorized saved provider must publish");
    let generation = routing.with_provider(&scope, |provider, generation| {
        assert_eq!(provider.provider_id, "route");
        assert!(generation > 0);
        Ok(generation)
    })?;
    let path = native.join("settings.json");
    let configuration = fs::read(&path)?;
    let saved: Value = serde_json::from_slice(&configuration)?;
    let token = saved["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap().to_owned();
    assert!(token.starts_with("sra_"));
    assert!(
        !String::from_utf8_lossy(&configuration).contains("fixture-key"),
        "client configuration must contain only its local routing credential"
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let request = |token: &str| {
        client
            .post(format!("http://{address}/v1/messages"))
            .header("x-api-key", token)
            .json(&json!({"model":"fixture","messages":[{"role":"user","content":"hello"}]}))
    };
    assert_eq!(request("PROXY_MANAGED").send().await?.status(), 401);
    assert_eq!(request(&token).send().await?.status(), 200);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    let streaming = client.post(format!("http://{address}/v1/messages"))
        .header("x-api-key", &token)
        .json(&json!({"model":"fixture","stream":true,"messages":[{"role":"user","content":"hello"}]}))
        .send().await?;
    assert_eq!(streaming.status(), 200);
    let expected = crate::model_relay::stream_observer::tests::samples()
        .into_iter()
        .find(|(kind, _, _)| *kind == "anthropic")
        .unwrap()
        .2;
    assert_eq!(streaming.text().await?, expected);
    assert_eq!(hits.load(Ordering::SeqCst), 2);

    // Force a failure after the in-memory queue and flag have both changed.
    let settings = GatewaySettingsStore::new(database.clone())?;
    let key = config_key(&scope)?;
    let exhausted = FailoverQueueState {
        version: FailoverQueueVersion(u64::MAX),
        enabled: false,
        members: vec![],
    };
    settings.upsert(tenant, NAMESPACE, &key, &serde_json::to_string(&exhausted)?)?;
    assert!(update(exhausted.version, true).is_err());
    assert_eq!(queues.get(&scope)?, exhausted, "failed enablement must roll back both fields");
    settings.delete(tenant, NAMESPACE, &key)?;

    let enabled = update(before.version, true)?;
    assert!(enabled.enabled);
    assert_eq!(enabled.members, ["route"]);
    assert_eq!(enabled.version.0, 1);
    assert_eq!(update(enabled.version, true)?, enabled);
    assert_eq!(fs::read(&path)?, configuration, "enabling must preserve native configuration");
    let providers =
        store.failover_providers(&scope.project, scope.app_id).map_err(anyhow::Error::msg)?;
    let queued = queues.update(&scope, enabled.version, |members, _| {
        members.add("backup", &providers).unwrap();
        Ok(())
    })?;
    let disabled = update(queued.version, false)?;
    assert!(!disabled.enabled);
    assert_eq!(disabled.members, ["route", "backup"]);
    assert_eq!(update(disabled.version, true)?.members, ["route", "backup"]);
    let persisted = queues.get(&scope)?;
    assert!(update(FailoverQueueVersion(0), true).is_err());
    assert_eq!(queues.get(&scope)?, persisted);

    // Selection consumes the real published mode and saved queue without
    // changing native configuration or using the display observation TTL.
    {
        use vw_agent_apps::high_availability::{
            circuit_breaker::AttemptOutcome,
            health::{HealthAccount, HealthKey, HealthRegistry},
            selection::SelectionError,
        };
        use vw_api_types::agent_apps_high_availability::CircuitBreakerPolicy;
        let health =
            HealthRegistry::new(std::num::NonZeroUsize::new(8).unwrap(), std::time::Duration::ZERO);
        let policy = CircuitBreakerPolicy {
            failure_threshold: 1,
            success_threshold: 1,
            cooldown_secs: 60,
            error_rate_threshold: 0.5,
            min_requests: 5,
        };
        let select = || {
            super::agent_apps_high_availability_service::select_candidates(
                database.clone(),
                &routing,
                home.path(),
                &root,
                &scope,
                &health,
                &policy,
            )
            .map(|selection| {
                assert_eq!(selection.generation, generation);
                assert_eq!(selection.queue_version, queues.get(&scope).unwrap().version);
                selection.provider_ids
            })
        };
        assert_eq!(select()?, ["route", "backup"]);
        // These saved third-party cards have no account binding. Observations
        // for a bound account, its replacement, or a new generation stay isolated.
        for (account, generation) in [("account-a", 1), ("account-b", 1), ("account-a", 2)] {
            let key = HealthKey {
                project: scope.project.clone(),
                target_host_id: scope.target_host_id.clone(),
                app_id: scope.app_id,
                provider_id: "route".into(),
                account: HealthAccount::Bound { id: account.into(), generation },
            };
            let now = std::time::Instant::now();
            health
                .circuit(&key, now)
                .unwrap()
                .allow_request(&policy, now)
                .unwrap()
                .record(AttemptOutcome::ProviderFailure, now);
            assert_eq!(select()?, ["route", "backup"]);
        }
        let before = store.claude_record(tenant, "route").map_err(anyhow::Error::msg)?.unwrap();
        let official = store
            .claude_save(
                tenant,
                "route",
                &ClaudeProfile {
                    name: "route".into(),
                    config: ClaudeConfig::Official { account: None },
                    model: None,
                },
                Some(&before),
                false,
            )
            .map_err(anyhow::Error::msg)?;
        assert!(select().is_err(), "changing the account kind invalidates the published proof");
        let original: ClaudeProfile = serde_json::from_value(before["claude"].clone())?;
        store
            .claude_save(tenant, "route", &original, Some(&official), false)
            .map_err(anyhow::Error::msg)?;
        assert_eq!(select()?, ["route", "backup"]);
        let backup_key = "\"claude\":backup";
        let backup = settings.get(tenant, "app_providers.records", backup_key)?.unwrap();
        let mut invalid: Value = serde_json::from_str(&backup)?;
        invalid["managedClaudeLogin"] = serde_json::to_value(ClaudeManagedLogin {
            tenant: tenant.clone(),
            principal: "manager".into(),
            host: scope.target_host_id.clone(),
            account_id: "account-a".into(),
        })?;
        settings.upsert(tenant, "app_providers.records", backup_key, &invalid.to_string())?;
        assert!(
            select().is_err(),
            "a managed account marker on a third-party card must fail owner validation"
        );
        settings.upsert(tenant, "app_providers.records", backup_key, &backup)?;
        let mut legacy: Value = serde_json::from_str(&backup)?;
        legacy["meta"] = json!({"claudeAccountId":"account-a"});
        settings.upsert(tenant, "app_providers.records", backup_key, &legacy.to_string())?;
        assert_eq!(select()?, ["route"], "unresolved legacy binding cannot use unbound health");
        settings.upsert(tenant, "app_providers.records", backup_key, &backup)?;
        let open = |id: &str| {
            let key = HealthKey {
                project: scope.project.clone(),
                target_host_id: scope.target_host_id.clone(),
                app_id: scope.app_id,
                provider_id: id.into(),
                account: HealthAccount::Unbound,
            };
            let now = std::time::Instant::now();
            health
                .circuit(&key, now)
                .unwrap()
                .allow_request(&policy, now)
                .unwrap()
                .record(AttemptOutcome::ProviderFailure, now);
        };
        open("route");
        assert_eq!(select()?, ["backup"]);
        open("backup");
        assert_eq!(
            select().unwrap_err().downcast_ref::<SelectionError>(),
            Some(&SelectionError::AllProvidersCircuitOpen)
        );
        let disabled = update(persisted.version, false)?;
        assert_eq!(select()?, ["route"], "disabled selection ignores open circuits");
        update(disabled.version, true)?;
        assert_eq!(fs::read(&path)?, configuration);
    }
    let persisted = queues.get(&scope)?;

    apply(&proxy("backup")).expect("switch must commit and publish the replacement");
    routing.with_provider(&scope, |provider, next_generation| {
        assert_eq!(provider.provider_id, "backup");
        assert!(next_generation > generation);
        Ok(())
    })?;
    assert_eq!(
        request(&token).send().await?.status(),
        401,
        "rotation revokes the previous credential"
    );
    let provider_key = "\"claude\":backup";
    let original = settings.get(tenant, "app_providers.records", provider_key)?.unwrap();
    settings.delete(tenant, "app_providers.records", provider_key)?;
    assert!(
        update(persisted.version, true).is_err(),
        "missing current provider must reject enablement"
    );
    assert_eq!(queues.get(&scope)?, persisted);
    settings.upsert(tenant, "app_providers.records", provider_key, &original)?;
    let configuration = fs::read(&path)?;
    // External file edits cannot turn a logical mode into a valid live snapshot.
    fs::write(&path, b"{}")?;
    assert!(update(persisted.version, true).is_err());
    assert_eq!(queues.get(&scope)?, persisted);
    fs::write(&path, &configuration)?;

    let mut foreign = scope.clone();
    foreign.project = ProjectScope::new(tenant.clone(), "other-project".into())?;
    assert!(
        set_enabled(
            database.clone(),
            &routing,
            home.path(),
            &root,
            &foreign,
            FailoverQueueVersion(0),
            true
        )
        .is_err()
    );
    let saved: Value = serde_json::from_slice(&fs::read(&path)?)?;
    let latest_token = saved["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap();
    routing.stop().await?;
    routing.start(address.port())?;
    assert_eq!(request(latest_token).send().await?.status(), 401);
    assert!(update(persisted.version, true).is_err(), "restart must require a fresh publication");
    assert_eq!(queues.get(&scope)?, persisted);
    apply(&ModeApplyAction::Direct {}).expect("direct restoration must commit before revocation");
    assert!(update(persisted.version, true).is_err());
    assert_eq!(queues.get(&scope)?, persisted);
    assert_eq!(update(persisted.version, false)?.members, ["route", "backup"]);
    routing.stop().await?;
    server.abort();
    let _ = server.await;
    Ok(())
}

struct ProviderPreimageFixture {
    queues: FailoverQueueStore,
    settings: GatewaySettingsStore,
    scope: FailoverQueueScope,
    providers: Vec<vw_agent_apps::providers::failover::ProviderSnapshot>,
    preimages: Vec<super::settings_store::GatewaySettingPreimage>,
    replacement: String,
}

impl ProviderPreimageFixture {
    fn new(database: GlobalDatabase) -> anyhow::Result<Self> {
        let scope = FailoverQueueScope {
            project: ProjectScope::new(TenantScope::new("p", "owner")?, "project".into())?,
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        };
        let providers = AppProviderStore::new(database.clone()).map_err(anyhow::Error::msg)?;
        providers
            .claude_save(
                scope.project.tenant_scope(),
                "route",
                &ClaudeProfile {
                    name: "Fixture".into(),
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
        let configuration = providers
            .failover_configuration(&scope.project, scope.app_id)
            .map_err(anyhow::Error::msg)?;
        let (providers, preimages): (Vec<_>, Vec<_>) =
            configuration.into_iter().map(|entry| (entry.provider, entry.preimage)).unzip();
        let mut replacement: Value = serde_json::from_str(&preimages[0].payload_json)?;
        replacement["name"] = json!("Updated");
        Ok(Self {
            queues: FailoverQueueStore::new(database.clone())?,
            settings: GatewaySettingsStore::new(database)?,
            scope,
            providers,
            preimages,
            replacement: replacement.to_string(),
        })
    }

    fn reject_stale(&self) -> anyhow::Result<()> {
        let reference = &self.preimages[0];
        let tenant = self.scope.project.tenant_scope();
        self.settings.upsert(
            tenant,
            reference.namespace,
            &reference.config_key,
            &self.replacement,
        )?;
        assert!(
            self.queues
                .update_with_preimages(
                    &self.scope,
                    FailoverQueueVersion(0),
                    &self.preimages,
                    |_, _| panic!("stale configuration reached queue mutation")
                )
                .is_err()
        );
        assert_eq!(self.queues.get(&self.scope)?, FailoverQueueState::default());
        assert_eq!(self.settings.get(tenant, NAMESPACE, &config_key(&self.scope)?)?, None);
        self.settings.upsert(
            tenant,
            reference.namespace,
            &reference.config_key,
            &reference.payload_json,
        )?;
        Ok(())
    }

    fn commit(&self, blocked_write: impl FnOnce()) -> anyhow::Result<()> {
        let result = self.queues.update_with_preimages(
            &self.scope,
            FailoverQueueVersion(0),
            &self.preimages,
            |queue, enabled| {
                blocked_write();
                queue.add("route", &self.providers).unwrap();
                *enabled = true;
                Ok(())
            },
        )?;
        assert_eq!(result.version.0, 1);
        assert!(result.enabled);
        assert_eq!(result.members, ["route"]);
        assert_eq!(self.queues.get(&self.scope)?, result);
        Ok(())
    }
}

#[test]
fn ha_enable_requires_routing_mode_and_keeps_queue_sqlite_preimages() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    let fixture = ProviderPreimageFixture::new(database.clone())?;
    fixture.reject_stale()?;
    let writer = database.connect_sqlite()?;
    writer.busy_timeout(std::time::Duration::ZERO)?;
    let reference = &fixture.preimages[0];
    let tenant = fixture.scope.project.tenant_scope();
    let attempt = || {
        writer.execute(
            "UPDATE gateway_settings SET payload_json = ?5
         WHERE platform_id = ?1 AND company_id = ?2 AND namespace = ?3 AND config_key = ?4",
            rusqlite::params![
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                reference.namespace,
                reference.config_key,
                fixture.replacement
            ],
        )
    };
    fixture.commit(|| {
        let error = attempt().expect_err("provider write must wait for the enable transaction");
        assert_eq!(error.sqlite_error_code(), Some(rusqlite::ErrorCode::DatabaseBusy));
    })?;
    assert_eq!(attempt()?, 1);
    assert_eq!(
        fixture.settings.get(tenant, reference.namespace, &reference.config_key)?,
        Some(fixture.replacement.clone())
    );
    Ok(())
}

#[test]
fn ha_enable_requires_routing_mode_and_keeps_queue_postgres_preimages() -> anyhow::Result<()> {
    let database = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    let fixture = ProviderPreimageFixture::new(database.database().clone())?;
    fixture.reject_stale()?;
    let mut writer = database.database().connect_postgres()?;
    writer.batch_execute("SET lock_timeout = '100ms'")?;
    let reference = &fixture.preimages[0];
    let tenant = fixture.scope.project.tenant_scope();
    let mut attempt = || {
        writer.execute(
            "UPDATE gateway_settings SET payload_json = $5
         WHERE platform_id = $1 AND company_id = $2 AND namespace = $3 AND config_key = $4",
            &[
                &tenant.platform_id().as_ref(),
                &tenant.company_id().as_ref(),
                &reference.namespace,
                &reference.config_key,
                &fixture.replacement,
            ],
        )
    };
    fixture.commit(|| {
        let error = attempt().expect_err("provider write must wait for the enable transaction");
        assert_eq!(error.code(), Some(&postgres::error::SqlState::LOCK_NOT_AVAILABLE));
    })?;
    assert_eq!(attempt()?, 1);
    assert_eq!(
        fixture.settings.get(tenant, reference.namespace, &reference.config_key)?,
        Some(fixture.replacement.clone())
    );
    drop(writer);
    drop(fixture);
    database.cleanup()
}
