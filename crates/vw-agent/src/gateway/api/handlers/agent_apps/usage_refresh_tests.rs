use std::os::unix::fs::PermissionsExt;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use axum::{
    Extension, Json, Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode},
    routing::get,
};
use serde_json::{Value, json};
use tokio::sync::{Notify, Semaphore};
use tower::ServiceExt;
use vw_agent_apps::usage::cache::UsageCacheResult;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, ProviderProtocol},
    apps_usage::{UsageQueryData, UsageQueryErrorCode, UsageQueryOutcome},
    id::{ProjectScope, TenantScope},
};

use super::usage_refresh::{RefreshError, UsageRefresh};
use crate::{
    agent_apps::usage_settings::{UsageSettingsScope, UsageSettingsStore},
    global_database::GlobalDatabase,
    project::{Info, TimeInfo, store::ProjectStore},
};

pub(super) struct Fixture {
    _home: tempfile::TempDir,
    pub(super) database: GlobalDatabase,
    project: ProjectScope,
    pub(super) path: PathBuf,
    pub(super) policy: Value,
    pub(super) hits: Arc<AtomicUsize>,
    upstream: tokio::task::JoinHandle<()>,
    started: Arc<Notify>,
    release: Arc<Semaphore>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.upstream.abort();
    }
}

impl Fixture {
    pub(super) async fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let database = GlobalDatabase::for_home(home.path()).unwrap();
        let tenant = TenantScope::new("test-platform", "test-company").unwrap();
        let project = ProjectScope::new(tenant.clone(), "project-a".into()).unwrap();
        ProjectStore::new(database.clone())
            .unwrap()
            .create(
                &tenant,
                &Info {
                    id: "project-a".into(),
                    worktree: home.path().to_string_lossy().into_owned(),
                    vcs: None,
                    name: None,
                    icon: None,
                    commands: None,
                    time: TimeInfo { created: 1, updated: 1, initialized: None },
                    sandboxes: vec![],
                },
            )
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        let started = Arc::new(Notify::new());
        let release = Arc::new(Semaphore::new(1));
        let signals = (started.clone(), release.clone());
        let app = Router::new().route(
            "/{provider}/user/balance",
            get(
                move |axum::extract::Path(provider): axum::extract::Path<String>,
                      headers: HeaderMap| {
                    let counter = counter.clone();
                    let (started, release) = signals.clone();
                    async move {
                        assert_eq!(headers["authorization"], "Bearer fixture-only-key");
                        counter.fetch_add(1, Ordering::SeqCst);
                        started.notify_one();
                        let _response = release.acquire().await.unwrap();
                        tokio::time::sleep(Duration::from_millis(60)).await;
                        if provider == "limited" {
                            (StatusCode::TOO_MANY_REQUESTS, Json(json!({})))
                        } else {
                            (StatusCode::OK, Json(json!({"balance": 17})))
                        }
                    }
                },
            ),
        );
        let upstream = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let providers = crate::gateway::api::handlers::app_providers::router()
            .layer(Extension(database.clone()))
            .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        for provider in ["ok", "limited", "ungranted"] {
            let profile = AppProviderWrite {
                name: provider.into(),
                protocol: ProviderProtocol::OpenaiResponses,
                api_key: "fixture-only-key".into(),
                base_url: format!("{origin}/{provider}"),
                model: "fixture-model".into(),
                request_options: None,
            };
            let request = Request::builder()
                .method("PUT")
                .uri(format!("/app-providers/codex/{provider}"))
                .header("content-type", "application/json")
                .header("x-platform-id", "test-platform")
                .header("x-company-id", "test-company")
                .body(Body::from(serde_json::to_vec(&profile).unwrap()))
                .unwrap();
            let response = providers.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        let path = home.path().join("access.json");
        let policy = json!({
            "principals": [{"id": "owner", "token_sha256": "0".repeat(64), "grants": [{
                "platform_id": "test-platform", "company_id": "test-company", "host_id": "local",
                "provider_config_read": true,
                "usage_refresh": [
                    {"project_id": "project-a", "app_id": "codex", "provider_id": "ok", "origin": origin},
                    {"project_id": "project-a", "app_id": "codex", "provider_id": "limited", "origin": origin}
                ]
            }]}],
            "hosts": [{"id": "local", "online": true, "binding": "local", "home": home.path(),
                       "applications": home.path(), "search_paths": []}]
        });
        let fixture =
            Self { _home: home, database, project, path, policy, hits, upstream, started, release };
        fixture.write_policy();
        for provider in ["ok", "limited", "ungranted"] {
            fixture.save(provider, true, 1);
        }
        fixture
    }

    pub(super) fn write_policy(&self) {
        std::fs::write(&self.path, self.policy.to_string()).unwrap();
        std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    pub(super) fn save(&self, provider: &str, enabled: bool, minutes: u16) {
        UsageSettingsStore::new(self.database.clone())
            .unwrap()
            .save(
                UsageSettingsScope {
                    project: &self.project,
                    principal: "owner",
                    target_host: "local",
                    app: AppId::Codex,
                    provider,
                    account: None,
                },
                &serde_json::from_value(json!({
                    "template": "general", "enabled": enabled, "refreshIntervalMinutes": minutes,
                }))
                .unwrap(),
            )
            .unwrap();
    }
}

#[tokio::test]
async fn usage_refresh_returns_expected_result() {
    let fixture = Fixture::new().await;
    let refresh = UsageRefresh::new(fixture.database.clone()).unwrap();
    let now = Instant::now();
    let (automatic, batch) =
        tokio::join!(refresh.tick(&fixture.path, now), refresh.refresh_all(&fixture.path, None),);
    assert_eq!(automatic, batch);
    assert_eq!(automatic.len(), 2);
    assert!(automatic.iter().any(|item| matches!(item.result.as_ref(),
        Ok(UsageCacheResult::Current(result)) if matches!(&result.outcome,
            UsageQueryOutcome::Success { data: UsageQueryData::CustomScript(packages) }
            if f64::from(packages[0].remaining.unwrap()) == 17.0))));
    assert!(automatic.iter().any(|item| matches!(item.result.as_ref(),
        Ok(UsageCacheResult::Current(result)) if matches!(&result.outcome,
            UsageQueryOutcome::Failure { error, .. } if error.code == UsageQueryErrorCode::UpstreamRateLimited))));
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 2);
    assert!(refresh.tick(&fixture.path, now + Duration::from_secs(59)).await.is_empty());
    assert_eq!(refresh.tick(&fixture.path, now + Duration::from_secs(60)).await.len(), 2);
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn usage_refresh_rejects_invalid_input() {
    let mut fixture = Fixture::new().await;
    let refresh = UsageRefresh::new(fixture.database.clone()).unwrap();
    fixture.save("ok", false, 1);
    fixture.save("limited", true, 0);
    assert!(refresh.tick(&fixture.path, Instant::now()).await.is_empty());
    fixture.save("ok", true, 1);
    fixture.policy["principals"][0]["grants"][0]["usage_refresh"] = json!([]);
    fixture.write_policy();
    assert!(refresh.refresh_all(&fixture.path, None).await.is_empty());
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn usage_refresh_discards_in_flight_result_after_revocation() {
    let mut fixture = Fixture::new().await;
    fixture.policy["principals"][0]["grants"][0]["usage_refresh"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    fixture.write_policy();
    let refresh = UsageRefresh::new(fixture.database.clone()).unwrap();
    let permit = fixture.release.acquire().await.unwrap();
    let path = fixture.path.clone();
    let worker = tokio::spawn(async move { refresh.tick(&path, Instant::now()).await });
    tokio::time::timeout(Duration::from_secs(5), fixture.started.notified()).await.unwrap();
    fixture.policy["principals"][0]["grants"][0]["usage_refresh"] = json!([]);
    fixture.write_policy();
    let results = tokio::time::timeout(Duration::from_secs(5), worker).await.unwrap().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].result.as_ref(), &Err(RefreshError::Revoked));
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 1);
    drop(permit);
}
