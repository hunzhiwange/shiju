//! Native balance and Coding Plan capabilities derived from provider base URLs.

use reqwest::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceProvider {
    DeepSeek,
    StepFun,
    SiliconFlowCn,
    SiliconFlowGlobal,
    OpenRouter,
    Novita,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodingPlanProvider {
    Kimi,
    ZhipuCn,
    ZhipuGlobal,
    ZhipuTeam,
    MiniMaxCn,
    MiniMaxGlobal,
    ZenMux,
    Volcengine,
    OpenCodeGo,
    CommandCode,
}

impl CodingPlanProvider {
    /// Volcengine needs separate account-level signing credentials before enabling queries.
    pub fn enabled_by_default(self) -> bool {
        self != Self::Volcengine
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeUsageCapability {
    Balance(BalanceProvider),
    CodingPlan(CodingPlanProvider),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZhipuPlanSelection {
    Auto,
    Team,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageDetectionError {
    InvalidBaseUrl,
    IncompatibleZhipuTeam,
}

/// Classifies an authorized provider's base URL. A valid URL without a native capability
/// returns `Ok(None)`. Callers authorize requests and resolve credentials separately.
pub fn detect_native_usage(
    base_url: &str,
    zhipu_plan: ZhipuPlanSelection,
) -> Result<Option<NativeUsageCapability>, UsageDetectionError> {
    let url = Url::parse(base_url).map_err(|_| UsageDetectionError::InvalidBaseUrl)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(UsageDetectionError::InvalidBaseUrl);
    }
    let host = url.host_str().ok_or(UsageDetectionError::InvalidBaseUrl)?;

    use NativeUsageCapability::{Balance, CodingPlan};

    if zhipu_plan == ZhipuPlanSelection::Team {
        return if host == "open.bigmodel.cn" {
            Ok(Some(CodingPlan(CodingPlanProvider::ZhipuTeam)))
        } else {
            Err(UsageDetectionError::IncompatibleZhipuTeam)
        };
    }

    let mut path = url.path().split('/').skip(1);
    let capability = match (host, path.next(), path.next()) {
        ("api.deepseek.com", _, _) => Balance(BalanceProvider::DeepSeek),
        ("api.stepfun.ai" | "api.stepfun.com", _, _) => Balance(BalanceProvider::StepFun),
        ("api.siliconflow.cn", _, _) => Balance(BalanceProvider::SiliconFlowCn),
        ("api.siliconflow.com", _, _) => Balance(BalanceProvider::SiliconFlowGlobal),
        ("openrouter.ai", _, _) => Balance(BalanceProvider::OpenRouter),
        ("api.novita.ai", _, _) => Balance(BalanceProvider::Novita),
        ("api.kimi.com" | "api.kimi.ai", Some("coding"), _) => CodingPlan(CodingPlanProvider::Kimi),
        ("bigmodel.cn" | "open.bigmodel.cn", _, _) => CodingPlan(CodingPlanProvider::ZhipuCn),
        ("api.z.ai", _, _) => CodingPlan(CodingPlanProvider::ZhipuGlobal),
        ("zenmux.ai", _, _) => CodingPlan(CodingPlanProvider::ZenMux),
        ("opencode.ai", Some("zen" | "inference"), Some("go")) => {
            CodingPlan(CodingPlanProvider::OpenCodeGo)
        }
        ("api.commandcode.ai", Some("provider"), _) => CodingPlan(CodingPlanProvider::CommandCode),
        (_, Some("api"), Some("plan" | "coding")) if is_volcengine_ark_host(host) => {
            CodingPlan(CodingPlanProvider::Volcengine)
        }
        _ if host_matches(host, "api.minimaxi.com") || host_matches(host, "api.minimax.cn") => {
            CodingPlan(CodingPlanProvider::MiniMaxCn)
        }
        _ if host_matches(host, "api.minimax.io") => CodingPlan(CodingPlanProvider::MiniMaxGlobal),
        _ => return Ok(None),
    };
    Ok(Some(capability))
}

fn host_matches(host: &str, domain: &str) -> bool {
    host == domain || host.strip_suffix(domain).is_some_and(|prefix| prefix.ends_with('.'))
}

fn is_volcengine_ark_host(host: &str) -> bool {
    host.strip_prefix("ark.")
        .and_then(|host| host.strip_suffix(".volces.com"))
        .is_some_and(|region| !region.is_empty() && !region.contains('.'))
}
