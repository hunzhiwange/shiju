//! 处理模型提供商设置子模块的目录、认证和连接状态。

use crate::app::message::settings::util::is_provider_connected;
use crate::app::state::{ModelCatalogEntry, ProviderSummary};
use std::collections::BTreeMap;
use vw_shared::provider::models as raw_model_provider;
use vw_shared::provider::types as model_provider;

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
                source_label: source_label.to_string(),
                connected,
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    out
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
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

/// 处理 `load_popular_providers_from_config` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub fn load_popular_providers_from_config(
    _cfg: &vw_config_types::config::Config,
) -> (Vec<String>, bool) {
    (Vec::new(), false)
}

/// 处理 `build_catalog_from_provider_infos` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub fn build_catalog_from_provider_infos(
    providers: &std::collections::HashMap<String, model_provider::Info>,
) -> Vec<ModelCatalogEntry> {
    let mut out = providers
        .values()
        .filter(|p| !p.id.trim().is_empty())
        .flat_map(|p| {
            let provider_id = p.id.clone();
            let provider_name = p.name.clone();
            p.models.values().filter(|m| !m.id.trim().is_empty()).map(move |m| ModelCatalogEntry {
                provider_id: provider_id.clone(),
                provider_name: provider_name.clone(),
                model_id: m.id.clone(),
                model_name: m.name.clone(),
            })
        })
        .collect::<Vec<_>>();
    sort_catalog(&mut out);
    out
}

/// 处理 `build_catalog_from_sources` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub fn build_catalog_from_sources(
    raw_providers: &std::collections::HashMap<String, raw_model_provider::Provider>,
    providers: &std::collections::HashMap<String, model_provider::Info>,
) -> Vec<ModelCatalogEntry> {
    let mut merged = BTreeMap::<(String, String), ModelCatalogEntry>::new();

    for provider in raw_providers.values().filter(|p| !p.id.trim().is_empty()) {
        for model in provider.models.values().filter(|m| !m.id.trim().is_empty()) {
            let entry = ModelCatalogEntry {
                provider_id: provider.id.clone(),
                provider_name: provider.name.clone(),
                model_id: model.id.clone(),
                model_name: model.name.clone(),
            };
            merged.insert((entry.provider_id.clone(), entry.model_id.clone()), entry);
        }
    }

    for entry in build_catalog_from_provider_infos(providers) {
        merged.insert((entry.provider_id.clone(), entry.model_id.clone()), entry);
    }

    let mut out = merged.into_values().collect::<Vec<_>>();
    sort_catalog(&mut out);
    out
}

fn sort_catalog(out: &mut [ModelCatalogEntry]) {
    out.sort_by(|a, b| {
        a.provider_name
            .cmp(&b.provider_name)
            .then_with(|| a.provider_id.cmp(&b.provider_id))
            .then_with(|| a.model_name.cmp(&b.model_name))
            .then_with(|| a.model_id.cmp(&b.model_id))
    });
}
