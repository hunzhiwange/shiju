use crate::provider_state::ModelCatalogEntry;
use model_catalog_types as raw_model_provider;
use model_types as model_provider;
use std::collections::BTreeMap;
/// 处理 `load_popular_providers_from_config` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub fn load_popular_providers_from_config(
    _cfg: &config_aggregate::Config,
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
