//! 模型查询、建议与默认选择。
use crate::{Info, Model, ModelNotFoundError, ParsedModelRef, parse_model, sort};
use crate::state::{state_filtered, state_for_settings};
use provider_config_reader as config;
use provider_model_cache as models;
use std::collections::HashMap;

/// 列出当前可用 provider。
///
/// # 返回值
///
/// 返回已过滤后的 provider 映射，key 为 provider_id。
pub async fn list() -> HashMap<String, Info> {
    state_filtered().await.providers.clone()
}

/// 列出设置页需要展示的全部 provider。
pub async fn list_for_settings() -> HashMap<String, Info> {
    state_for_settings().await.providers.clone()
}

/// 按 provider_id 获取 provider 信息。
///
/// # 参数
///
/// * `provider_id` - provider 的稳定标识
pub async fn get_provider(provider_id: &str) -> Option<Info> {
    state_filtered().await.providers.get(provider_id).cloned()
}

/// 按 provider_id 与 model_id 获取模型信息，并提供有限的模糊建议。
///
/// 当 provider_id 或 model_id 不存在时，会返回带建议项的 `ModelNotFoundError`，
/// 以改善命令行与 UI 场景下的可诊断性。
pub async fn get_model(provider_id: &str, model_id: &str) -> Result<Model, ModelNotFoundError> {
    let state = state_filtered().await;

    let Some(provider) = state.providers.get(provider_id) else {
        let suggestions =
            suggest(provider_id, state.providers.keys().map(|s| s.as_str()).collect());
        return Err(ModelNotFoundError {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            suggestions,
        });
    };

    let Some(model) = provider.models.get(model_id).cloned() else {
        let alias_matches =
            provider.models.values().filter(|m| m.api.id == model_id).cloned().collect::<Vec<_>>();

        if alias_matches.len() == 1 {
            return Ok(alias_matches[0].clone());
        }

        let suggestions = if alias_matches.len() > 1 {
            let mut ids = alias_matches.into_iter().map(|m| m.id).collect::<Vec<_>>();
            ids.sort();
            ids.truncate(3);
            ids
        } else {
            suggest(model_id, provider.models.keys().map(|s| s.as_str()).collect())
        };

        return Err(ModelNotFoundError {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            suggestions,
        });
    };

    Ok(model)
}

/// 返回默认模型；若配置未指定，则选择排序后的首个 provider 与模型。
///
/// 默认选择逻辑保持确定性：先按 provider_id 排序，再按模型排序规则选择首项。
pub async fn default_model() -> Result<ParsedModelRef, String> {
    if let Some(m) = config::read_default_model().await.as_deref() {
        return Ok(parse_model(m));
    }

    let providers = list().await;
    let mut candidates = providers.values().collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.id.cmp(&b.id));

    let Some(p) = candidates.first() else {
        return Err("没有可用的 provider".to_string());
    };

    let mut model_list = p.models.values().cloned().collect::<Vec<_>>();
    model_list = sort(model_list);

    let Some(m) = model_list.first() else {
        return Err("没有可用的模型".to_string());
    };

    Ok(ParsedModelRef { provider_id: p.id.clone(), model_id: m.id.clone() })
}

/// 对 provider 或模型 id 给出最多三个相似候选。
///
/// 当前使用 Jaro-Winkler 相似度，阈值为 `0.70`。
fn suggest(query: &str, candidates: Vec<&str>) -> Vec<String> {
    let mut scored = candidates
        .into_iter()
        .map(|c| (c, strsim::jaro_winkler(query, c)))
        .filter(|(_, s)| *s > 0.70)
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored.into_iter().take(3).map(|(c, _)| c.to_string()).collect()
}

pub async fn get_small_model(provider_id: &str) -> Option<Model> {
    let state = state_filtered().await;
    let provider = state.providers.get(provider_id)?;

    let mut priority = vec![
        "claude-haiku-4-5",
        "claude-haiku-4.5",
        "3-5-haiku",
        "3.5-haiku",
        "gemini-3-flash",
        "gemini-2.5-flash",
        "gpt-5-nano",
    ];

    if provider_id.starts_with("vibewindow") {
        priority = vec!["gpt-5-nano"];
    }

    if provider_id.starts_with("github-copilot") {
        priority.insert(0, "gpt-5-mini");
        priority.insert(1, "claude-haiku-4.5");
    }

    for item in priority {
        for (id, model) in &provider.models {
            if id.contains(item) {
                return Some(model.clone());
            }
        }
    }

    None
}

pub fn init() {
    models::init();
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "query_tests.rs"]
mod tests;
