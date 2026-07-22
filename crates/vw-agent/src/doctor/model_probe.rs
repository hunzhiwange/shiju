//! 模型目录探测入口。
//!
//! 本模块负责确定需要探测的 provider 列表，并为 doctor 的模型目录探测命令
//! 提供输出入口。目前真实探测能力被显式禁用，因此这里只报告目标 provider。

use crate::app::agent::config::Config;
use anyhow::Result;

#[cfg(test)]
/// 测试中用于归类模型探测错误的结果。
///
/// 该枚举只在测试配置下暴露，用来确保错误字符串能稳定映射到跳过、鉴权/额度、
/// 以及普通错误三类结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModelProbeOutcome {
    /// provider 明确不支持在线模型发现。
    Skipped,
    /// 鉴权、访问权限、额度或限流相关错误。
    AuthOrAccess,
    /// 不属于已知可归类场景的普通错误。
    Error,
}

#[cfg(test)]
/// 将模型探测错误文本归类为测试断言可用的稳定结果。
///
/// 参数：
/// - `err_message`：来自 provider 或传输层的错误文本。
///
/// 返回值：
/// 返回 `ModelProbeOutcome`，用于测试 doctor 对常见错误的分类策略。
pub(super) fn classify_model_probe_error(err_message: &str) -> ModelProbeOutcome {
    let lower = err_message.to_lowercase();

    if lower.contains("does not support live model discovery") {
        return ModelProbeOutcome::Skipped;
    }

    if [
        "401",
        "403",
        "429",
        "unauthorized",
        "forbidden",
        "api key",
        "token",
        "insufficient balance",
        "insufficient quota",
        "plan does not include",
        "rate limit",
    ]
    .iter()
    .any(|hint| lower.contains(hint))
    {
        // 鉴权与额度类错误通常需要用户修复配置或账户状态；测试中单独归类，
        // 避免把可操作问题混入普通探测失败。
        return ModelProbeOutcome::AuthOrAccess;
    }

    ModelProbeOutcome::Error
}

/// 计算本次模型探测的 provider 目标列表。
///
/// 参数：
/// - `provider_override`：用户显式指定的 provider；非空时只探测该 provider。
///
/// 返回值：
/// 返回排序后的 provider key 列表，保证 doctor 输出顺序稳定。
pub(super) fn resolve_model_probe_targets(
    provider_override: Option<&str>,
    provider_keys: impl IntoIterator<Item = String>,
) -> Vec<String> {
    if let Some(provider) =
        provider_override.map(str::trim).filter(|value| !value.is_empty()).map(ToString::to_string)
    {
        return vec![provider];
    }

    let mut providers = provider_keys.into_iter().collect::<Vec<_>>();
    providers.sort();
    providers
}

pub(super) async fn doctor_model_targets(provider_override: Option<&str>) -> Vec<String> {
    if provider_override.map(str::trim).is_some_and(|value| !value.is_empty()) {
        return resolve_model_probe_targets(provider_override, Vec::<String>::new());
    }

    let providers = crate::app::agent::provider::provider::list().await.into_keys();
    resolve_model_probe_targets(provider_override, providers)
}

pub(super) fn report_model_probe_targets(targets: Vec<String>) -> Result<()> {
    if targets.is_empty() {
        anyhow::bail!("No providers available for model probing");
    }

    println!("🩺 VibeWindow Doctor — Model Catalog Probe");
    println!("  Note: Model probing feature is currently disabled");
    println!("  Providers: {}", targets.join(", "));
    Ok(())
}

/// 运行模型目录探测命令。
///
/// 参数：
/// - `_config`：doctor 运行配置；当前探测禁用，暂未读取。
/// - `provider_override`：可选 provider 过滤。
/// - `_use_cache`：缓存开关占位；当前探测禁用，暂未使用。
///
/// 返回值：
/// 成功时打印当前状态并返回 `Ok(())`。
///
/// 错误处理：
/// 如果没有可用 provider，会返回错误；探测能力禁用时不会尝试网络请求，避免产生
/// 误导性的鉴权或连通性副作用。
pub async fn run_models(
    _config: &Config,
    provider_override: Option<&str>,
    _use_cache: bool,
) -> Result<()> {
    let targets = doctor_model_targets(provider_override).await;
    report_model_probe_targets(targets)
}
