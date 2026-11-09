//! Native provider queries admitted by the gateway's scoped refresh policy.

use reqwest::{ClientBuilder, Url};
use vw_agent_apps::usage::{
    balances::deepseek::query_deepseek_with_transport,
    coding::kimi::query_kimi_with_transport,
    detect::{
        BalanceProvider, CodingPlanProvider, NativeUsageCapability, ZhipuPlanSelection,
        detect_native_usage,
    },
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::AuthRegion,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryError, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageQueryResult, UsageSource,
    },
};

pub(super) enum NativeQuery {
    DeepSeek,
    Kimi(AuthRegion),
}

impl NativeQuery {
    pub(super) fn select(base: &Url, kind: UsageQueryKind) -> Option<Self> {
        if base.scheme() != "https" || base.port().is_some() {
            return None;
        }
        let capability = detect_native_usage(base.as_str(), ZhipuPlanSelection::Auto).ok()??;
        match (kind, capability) {
            (
                UsageQueryKind::Balance,
                NativeUsageCapability::Balance(BalanceProvider::DeepSeek),
            ) => Some(Self::DeepSeek),
            (
                UsageQueryKind::CodingPlan,
                NativeUsageCapability::CodingPlan(CodingPlanProvider::Kimi),
            ) => {
                let region = match base.host_str()? {
                    "api.kimi.com" => AuthRegion::MainlandCn,
                    "api.kimi.ai" => AuthRegion::Global,
                    _ => return None,
                };
                Some(Self::Kimi(region))
            }
            _ => None,
        }
    }

    pub(super) async fn run(
        self,
        transport: ClientBuilder,
        key: &str,
        now: TimestampMs,
        correlation_id: &str,
    ) -> UsageQueryResult {
        if let Self::Kimi(region) = self {
            return query_kimi_with_transport(transport, region, key, now, correlation_id).await;
        }
        let (outcome, credential_state) = match query_deepseek_with_transport(transport, key).await
        {
            Ok(balance) => (
                UsageQueryOutcome::Success { data: UsageQueryData::Balance(balance.wallets) },
                UsageCredentialState::Valid,
            ),
            Err(code) => (
                UsageQueryOutcome::Failure {
                    kind: UsageQueryKind::Balance,
                    error: UsageQueryError {
                        code,
                        message: "DeepSeek balance query failed".into(),
                        correlation_id: correlation_id.into(),
                    },
                },
                match code {
                    UsageQueryErrorCode::UpstreamUnauthorized => UsageCredentialState::Expired,
                    UsageQueryErrorCode::CredentialUnavailable => UsageCredentialState::NotFound,
                    _ => UsageCredentialState::Valid,
                },
            ),
        };
        UsageQueryResult {
            source: UsageSource::Provider("deepseek".into()),
            queried_at: (credential_state != UsageCredentialState::NotFound).then_some(now),
            credential_state,
            credential_message: None,
            outcome,
        }
    }
}
