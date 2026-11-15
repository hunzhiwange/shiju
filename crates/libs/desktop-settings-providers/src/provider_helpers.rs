//! 处理模型提供商设置子模块的目录、认证和连接状态。

#[cfg(test)]
use model_catalog_types as raw_model_provider;
#[cfg(test)]
use crate::provider_state::ModelCatalogEntry;
use desktop_i18n::settings_models::messages::Feedback;
use crate::models::is_provider_connected;
use crate::provider_state::ProviderSummary;
use model_types as model_provider;

/// 处理 `summarize_providers` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub fn summarize_providers(
    providers: std::collections::HashMap<String, model_provider::Info>,
) -> Vec<ProviderSummary> {
    let mut out = providers
        .into_values()
        .map(|p| {
            let (source_label, connected) = match p.source {
                model_provider::ProviderSource::Api => ("API 密钥", true),
                model_provider::ProviderSource::Env => ("环境变量", true),
                model_provider::ProviderSource::Config => ("配置", is_provider_connected(&p)),
                model_provider::ProviderSource::Custom => ("内置", is_provider_connected(&p)),
            };
            ProviderSummary {
                id: p.id,
                name: p.name,
                source_label: Feedback::owned(source_label, vec![]),
                connected,
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    out
}

#[cfg(test)]
#[path = "provider_helpers_tests.rs"]
mod helpers_tests;

/// 处理 `is_valid_provider_id` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub fn is_valid_provider_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.to_ascii_lowercase() != s {
        return false;
    }
    s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}


pub use crate::provider_catalog::*;
