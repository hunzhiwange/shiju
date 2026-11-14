//! Per-attempt native observations. UI/session totals are never ledger inputs.
use vw_agent_apps::usage::store::{self, StoredUsageRecord, UsageFailure, UsageSourceIdentity};
use vw_api_types::{id::ProjectScope, usage::*};

tokio::task_local! {
    static SESSION: String;
}

pub(crate) async fn with_session<T>(session: String, future: impl std::future::Future<Output = T>) -> T {
    SESSION.scope(session, future).await
}

pub(crate) struct NativeRequest {
    scope: ProjectScope,
    host: String,
    id: String,
    app: String,
    provider: UsageProvider,
    model: Option<String>,
    session: Option<String>,
    started: i64,
    clock: std::time::Instant,
    measured_latency: bool,
    finished: bool,
}

impl NativeRequest {
    /// Missing trusted context rejects observation, without changing model authorization.
    pub(crate) fn begin(provider: &str, model: &str, session: Option<&str>) -> Option<Self> {
        match Self::try_begin(provider, model, session) {
            Ok(request) => Some(request),
            Err(_) => {
                tracing::debug!("native usage observation denied: trusted local scope unavailable");
                None
            }
        }
    }

    fn try_begin(provider: &str, model: &str, session: Option<&str>) -> anyhow::Result<Self> {
        let tenant = crate::session::ui_store::explicit_tenant_scope()
            .ok_or_else(|| anyhow::anyhow!("usage requires explicit tenant"))?;
        let project = crate::project::instance::project()
            .ok_or_else(|| anyhow::anyhow!("usage requires registered project"))?;
        let scope = ProjectScope::new(tenant, project.id.into())?;
        let database = super::context::database()?;
        anyhow::ensure!(crate::project::store::ProjectStore::new(database)?.get(&scope)?.is_some(),
            "usage project outside tenant");
        // Only existing private desktop authorization may establish this local host.
        anyhow::ensure!(vw_shared::local_apps::restore(&crate::global::paths().home,
            scope.platform_id().as_ref(), scope.company_id().as_ref())?.is_some(),
            "local usage host is not provisioned");
        Ok(Self {
            app: "shiju".into(), scope, host: super::context::local_host()?,
            id: format!("shiju-attempt:{}", uuid::Uuid::new_v4()),
            provider: if provider.is_empty() { UsageProvider::Unknown } else {
                UsageProvider::Known { provider_id: provider.to_owned() }
            },
            model: (!model.is_empty()).then(|| model.to_owned()),
            session: session.filter(|s| !s.is_empty()).map(str::to_owned)
                .or_else(|| SESSION.try_with(Clone::clone).ok()),
            started: chrono::Utc::now().timestamp(), clock: std::time::Instant::now(), measured_latency: true, finished: false,
        })
    }

    /// A CLI supplies this ID as evidence; never derive it from time or token totals.
    pub(crate) fn native_identity(&mut self, app: &str, id: &str) {
        self.measured_latency = false;
        self.app = app.to_owned();
        self.id = id.to_owned();
    }

    pub(crate) fn complete(&mut self, tokens: Option<UsageTokens>) {
        self.save(UsageRequestStatus::Succeeded, tokens);
    }

    pub(crate) fn fail(&mut self) {
        self.save(UsageRequestStatus::Failed, None);
    }

    fn save(&mut self, status: UsageRequestStatus, tokens: Option<UsageTokens>) {
        if self.finished { return; }
        self.finished = true;
        let source = UsageSourceRef {
            source_id: self.id.clone(), kind: UsageSourceKind::Shiju, app: self.app.clone(),
            native_request_id: Some(self.id.clone()), native_session_id: self.session.clone(),
        };
        let value = StoredUsageRecord {
            record: UsageRecord {
                id: uuid::Uuid::new_v4().to_string(), started_at_unix_seconds: UsageUnixSeconds(self.started),
                provider: self.provider.clone(), sources: vec![source],
                models: UsageModels { request_model: self.model.clone(), response_model: None,
                    pricing_model: None, pricing_multiplier: None, multiplier_basis: None },
                status, billable: tokens.as_ref().is_some_and(|t| [t.input, t.output, t.cache_read,
                    t.cache_creation, t.reasoning].into_iter().flatten().any(|n| n > 0)), tokens,
                cost: UsageCost { source: UsageCostSource::Unknown, amount_usd: None },
                performance: UsagePerformance { first_token_ms: None,
                    latency_ms: self.measured_latency.then(|| UsageMeasurement { value: self.clock.elapsed().as_secs_f64() * 1000.0,
                        kind: UsageMeasurementKind::Measured }), output_tokens_per_second: None },
                suspected_duplicate_of: None,
            },
            status_code: None,
            failure: match status { UsageRequestStatus::Failed => Some(UsageFailure::Upstream),
                UsageRequestStatus::Cancelled => Some(UsageFailure::Cancelled), _ => None },
        };
        let identity = UsageSourceIdentity { target_host: self.host.clone(), app: self.app.clone(),
            instance_id: "shiju".into(), kind: UsageSourceKind::Shiju,
            file_identity: "shiju-live-requests".into(), native_session_id: self.session.clone(),
            native_request_id: self.id.clone() };
        let result = (|| -> anyhow::Result<()> {
            let mut connection = super::context::database()?.connect_sqlite()?;
            store::initialize_usage_store(&mut connection)?;
            store::save_usage_record(&mut connection, &self.scope, &identity, &value)?;
            Ok(())
        })();
        if result.is_err() {
            tracing::warn!("native usage observation could not be persisted");
        }
    }
}

impl Drop for NativeRequest {
    fn drop(&mut self) {
        if !self.finished { self.save(UsageRequestStatus::Cancelled, None); }
    }
}

pub(crate) fn ui_tokens(usage: &crate::session::ui_types::TokenUsage) -> UsageTokens {
    let count = |n| u64::try_from(n).ok();
    UsageTokens { input: count(usage.input_tokens), output: count(usage.output_tokens),
        reasoning: count(usage.reasoning_tokens), cache_read: count(usage.cached_tokens),
        cache_creation: None, cache_creation_1h: None,
        input_semantics: UsageInputSemantics::Unknown,
        reasoning_semantics: UsageReasoningSemantics::Unknown }
}
