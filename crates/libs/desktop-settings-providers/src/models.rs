//! 模型摘要、启用状态变更和配置补丁。
use crate::state::{ModelSummary, ProviderModelsSummary};
use serde_json::{Map, Value};
use model_types::{self as provider_types, Info};
/// 一项待持久化的模型启用状态变更。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStatusChange {
    pub provider_id: String,
    pub model_id: String,
    pub previous_enabled: bool,
    pub enabled: bool,
}

pub fn summarize_models(
    providers: std::collections::HashMap<String, Info>,
) -> Vec<ProviderModelsSummary> {
    let mut out = providers
        .into_values()
        .filter(is_provider_connected)
        .map(|p| {
            let mut models = p.models.into_values().collect::<Vec<_>>();
            models = provider_types::sort(models);
            let models = models
                .into_iter()
                .map(|m| {
                    let detail = serde_json::to_value(&m).unwrap_or(Value::Null);
                    let toolcall = m.capabilities.toolcall;
                    let image_input = m.capabilities.input.image;
                    let context_limit = m.limit.context;
                    let enabled = m.status == "active";
                    ModelSummary {
                        id: m.id,
                        name: m.name,
                        enabled,
                        toolcall,
                        attachment: image_input,
                        context_limit,
                        detail,
                    }
                })
                .collect::<Vec<_>>();
            ProviderModelsSummary { id: p.id, name: p.name, models }
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    out
}

pub(crate) fn collect_model_status_changes(
    providers: &[ProviderModelsSummary],
    enabled: bool,
) -> Vec<ModelStatusChange> {
    providers
        .iter()
        .flat_map(|provider| {
            provider.models.iter().filter(move |model| model.enabled != enabled).map(move |model| {
                ModelStatusChange {
                    provider_id: provider.id.clone(),
                    model_id: model.id.clone(),
                    previous_enabled: model.enabled,
                    enabled,
                }
            })
        })
        .collect()
}

pub(crate) fn collect_provider_model_status_changes(
    providers: &[ProviderModelsSummary],
    provider_id: &str,
    enabled: bool,
    include_unchanged: bool,
) -> Vec<ModelStatusChange> {
    providers
        .iter()
        .find(|provider| provider.id == provider_id)
        .into_iter()
        .flat_map(|provider| {
            provider
                .models
                .iter()
                .filter(move |model| include_unchanged || model.enabled != enabled)
                .map(move |model| ModelStatusChange {
                    provider_id: provider.id.clone(),
                    model_id: model.id.clone(),
                    previous_enabled: model.enabled,
                    enabled,
                })
        })
        .collect()
}

pub(crate) fn model_status_patch(changes: &[ModelStatusChange]) -> Value {
    let mut grouped = std::collections::BTreeMap::<String, Map<String, Value>>::new();
    for change in changes {
        let status = if change.enabled { "active" } else { "disabled" };
        grouped.entry(change.provider_id.clone()).or_default().insert(
            change.model_id.clone(),
            serde_json::json!({
                "id": change.model_id,
                "status": status,
            }),
        );
    }

    let providers = grouped
        .into_iter()
        .map(|(provider_id, models)| {
            let mut provider = Map::new();
            provider.insert("models".to_string(), Value::Object(models));
            (provider_id, Value::Object(provider))
        })
        .collect::<Map<_, _>>();

    let mut root = Map::new();
    root.insert("providers".to_string(), Value::Object(providers));
    Value::Object(root)
}

pub(crate) fn provider_model_status_reset_patch(
    config: &config_aggregate::Config,
    provider_id: &str,
) -> Option<Value> {
    let configured_models =
        config.providers.get(provider_id)?.get("models").and_then(Value::as_object)?;
    let models = configured_models
        .iter()
        .filter(|(_, model)| model.get("status").is_some())
        .map(|(model_id, _)| {
            let mut model = Map::new();
            model.insert("status".to_string(), Value::Null);
            (model_id.clone(), Value::Object(model))
        })
        .collect::<Map<_, _>>();
    if models.is_empty() {
        return None;
    }

    let mut provider = Map::new();
    provider.insert("models".to_string(), Value::Object(models));
    let mut providers = Map::new();
    providers.insert(provider_id.to_string(), Value::Object(provider));
    let mut root = Map::new();
    root.insert("providers".to_string(), Value::Object(providers));
    Some(Value::Object(root))
}

pub(crate) fn set_local_model_status(
    providers: &mut [ProviderModelsSummary],
    change: &ModelStatusChange,
    enabled: bool,
) {
    let Some(model) = providers
        .iter_mut()
        .find(|provider| provider.id == change.provider_id)
        .and_then(|provider| provider.models.iter_mut().find(|model| model.id == change.model_id))
    else {
        return;
    };

    model.enabled = enabled;
    if let Some(detail) = model.detail.as_object_mut() {
        let status = if enabled { "active" } else { "disabled" };
        detail.insert("status".to_string(), Value::String(status.to_string()));
    }
}

pub fn is_provider_connected(provider: &model_types::Info) -> bool {
    let has_key = provider.key.as_deref().is_some_and(|key| !key.trim().is_empty());

    has_key
        || matches!(
            provider.source,
            model_types::ProviderSource::Api
                | model_types::ProviderSource::Env
        )
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;

/// 构建模型详情；调用方负责处理提供商或模型不存在的错误。
pub(crate) fn model_detail(provider: &ProviderModelsSummary, model: &ModelSummary) -> crate::state::ModelDetailModalState {
    use crate::state::{ModelDetailModalState, ModelDetailRow};
    use desktop_i18n::settings_models::messages::Feedback;
    let detail_value = &model.detail;
    let raw_json = serde_json::to_string_pretty(detail_value)
        .unwrap_or_else(|_| serde_json::to_string(detail_value).unwrap_or_default());

    let get = |k: &str| detail_value.get(k).unwrap_or(&Value::Null);
    let get_str = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let get_u64 = |v: &Value| v.as_u64().unwrap_or(0);
    let yes_no = |b: bool| Feedback::owned(if b { "支持" } else { "不支持" }, vec![]);
    let join_caps = |io: &Value| {
        let mut out = Vec::new();
        let obj = io.as_object();
        if let Some(obj) = obj {
            for (k, v) in obj {
                if v.as_bool().unwrap_or(false) {
                    out.push(k.clone());
                }
            }
        }
        out.sort_unstable();
        if out.is_empty() { Feedback::owned("无", vec![]) } else { out.join(", ").into() }
    };

    let mut rows = Vec::<ModelDetailRow>::new();
    rows.push(ModelDetailRow {
        label: Feedback::owned("模型名称 (name)", vec![]),
        value: (model.name.clone()).into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("模型 ID (id)", vec![]),
        value: (model.id.clone()).into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("提供商 (providerID)", vec![]),
        value: (provider.id.clone()).into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("状态 (status)", vec![]),
        value: (get_str(get("status"))).into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("发布日期 (release_date)", vec![]),
        value: (get_str(get("release_date"))).into(),
    });

    let api = get("api");
    rows.push(ModelDetailRow {
        label: Feedback::owned("API Base URL (api.url)", vec![]),
        value: (api.get("url").and_then(Value::as_str).unwrap_or_default().to_string())
            .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("适配器 (api.adapter)", vec![]),
        value: (api.get("adapter").and_then(Value::as_str).unwrap_or_default().to_string())
            .into(),
    });

    let limit = get("limit");
    rows.push(ModelDetailRow {
        label: Feedback::owned("上下文限制 (limit.context)", vec![]),
        value: (get_u64(limit.get("context").unwrap_or(&Value::Null)).to_string()).into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("输入限制 (limit.input)", vec![]),
        value: (limit
            .get("input")
            .and_then(Value::as_u64)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string()))
        .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("输出限制 (limit.output)", vec![]),
        value: (get_u64(limit.get("output").unwrap_or(&Value::Null)).to_string()).into(),
    });

    let caps = get("capabilities");
    rows.push(ModelDetailRow {
        label: Feedback::owned("支持工具调用 (capabilities.toolcall)", vec![]),
        value: (yes_no(caps.get("toolcall").and_then(Value::as_bool).unwrap_or(false)))
            .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("支持附件 (capabilities.attachment)", vec![]),
        value: (yes_no(caps.get("attachment").and_then(Value::as_bool).unwrap_or(false)))
            .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("支持推理 (capabilities.reasoning)", vec![]),
        value: (yes_no(caps.get("reasoning").and_then(Value::as_bool).unwrap_or(false)))
            .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("支持温度 (capabilities.temperature)", vec![]),
        value: (yes_no(caps.get("temperature").and_then(Value::as_bool).unwrap_or(false)))
            .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("输入模态 (capabilities.input.*)", vec![]),
        value: (join_caps(caps.get("input").unwrap_or(&Value::Null))).into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("输出模态 (capabilities.output.*)", vec![]),
        value: (join_caps(caps.get("output").unwrap_or(&Value::Null))).into(),
    });

    let interleaved = caps.get("interleaved").unwrap_or(&Value::Null);
    let interleaved_str = match interleaved {
        Value::Bool(b) => b.to_string(),
        Value::Object(o) => o
            .get("field")
            .and_then(Value::as_str)
            .map(|s| format!("field: {}", s))
            .unwrap_or_default(),
        _ => String::new(),
    };
    if !interleaved_str.trim().is_empty() {
        rows.push(ModelDetailRow {
            label: Feedback::owned("混合输入 (capabilities.interleaved)", vec![]),
            value: (interleaved_str).into(),
        });
    }

    let cost = get("cost");
    let cache = cost.get("cache").unwrap_or(&Value::Null);
    rows.push(ModelDetailRow {
        label: Feedback::owned("输入价格 (cost.input)", vec![]),
        value: (cost.get("input").and_then(Value::as_f64).unwrap_or(0.0).to_string())
            .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("输出价格 (cost.output)", vec![]),
        value: (cost.get("output").and_then(Value::as_f64).unwrap_or(0.0).to_string())
            .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("缓存读 (cost.cache.read)", vec![]),
        value: (cache.get("read").and_then(Value::as_f64).unwrap_or(0.0).to_string())
            .into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("缓存写 (cost.cache.write)", vec![]),
        value: (cache.get("write").and_then(Value::as_f64).unwrap_or(0.0).to_string())
            .into(),
    });

    let headers_count = get("headers").as_object().map(|o| o.len()).unwrap_or(0);
    let options_count = get("options").as_object().map(|o| o.len()).unwrap_or(0);
    let variants_count = get("variants").as_object().map(|o| o.len()).unwrap_or(0);
    rows.push(ModelDetailRow {
        label: Feedback::owned("请求头数量 (headers)", vec![]),
        value: (headers_count.to_string()).into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("自定义选项数量 (options)", vec![]),
        value: (options_count.to_string()).into(),
    });
    rows.push(ModelDetailRow {
        label: Feedback::owned("变体数量 (variants)", vec![]),
        value: (variants_count.to_string()).into(),
    });

    ModelDetailModalState {
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        model_id: model.id.clone(),
        model_name: model.name.clone(),
        rows,
        raw_json,
        show_raw: false,
    }
}

pub fn changes_disable_selected_model(selected_model: &str, changes: &[ModelStatusChange]) -> bool {
    changes.iter().any(|change| {
        if change.enabled {
            return false;
        }
        let full = format!("{}/{}", change.provider_id, change.model_id);
        if selected_model.contains('/') { selected_model == full } else { selected_model == change.model_id }
    })
}

pub fn begin_status_save(state: &mut crate::state::ModelSettingsState, changes: &[ModelStatusChange]) {
    for change in changes { set_local_model_status(&mut state.providers, change, change.enabled); }
    state.saving = true;
    state.save_error = None;
    state.detail_modal = None;
}

#[cfg(test)]
#[path = "model_update_tests.rs"]
mod model_update_tests;

pub use crate::model_update::{ModelMessage, ModelEffect, update};
pub type ModelsLoaded = Result<Vec<ProviderModelsSummary>, desktop_i18n::settings_models::messages::Feedback>;

pub fn can_begin_status_save(state: &crate::state::ModelSettingsState, changes: &[ModelStatusChange]) -> bool {
    !changes.is_empty() && !state.loading && !state.saving
}
