//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config;
use crate::app::config::server_config_unreachable_error;
use crate::app::{
    App, Message,
    state::{ModelDetailModalState, ModelDetailRow, ModelSummary, ProviderModelsSummary},
};
use iced::Task;
use serde_json::{Map, Value};
use vw_shared::provider::types as provider_types;
use vw_shared::provider::types::Info;

use super::messages::SettingsMessage;
use super::util::is_provider_connected;

fn summarize_models(
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
                    let attachment = m.capabilities.attachment;
                    let context_limit = m.limit.context;
                    let enabled = m.status == "active";
                    ModelSummary {
                        id: m.id,
                        name: m.name,
                        enabled,
                        toolcall,
                        attachment,
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

async fn load_models_for_settings() -> Result<Vec<ProviderModelsSummary>, String> {
    let response = config::gateway_client()?.provider_list(None).await?;
    let providers =
        response.all.into_iter().map(|provider| (provider.id.clone(), provider)).collect();
    Ok(summarize_models(providers))
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;

/// 处理 `models_refresh_task` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn models_refresh_task() -> Task<Message> {
    Task::perform(
        async move { load_models_for_settings().await.map_err(server_config_unreachable_error) },
        |res| Message::Settings(SettingsMessage::ModelsRefreshed(res)),
    )
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::ModelsRefresh => {
            app.model_settings.loading = true;
            app.model_settings.save_error = None;
            app.model_settings.detail_modal = None;
            models_refresh_task()
        }
        SettingsMessage::ModelsRefreshed(res) => match res {
            Ok(providers) => {
                app.model_settings.loading = false;
                app.model_settings.providers = providers;
                app.model_settings.detail_modal = None;
                Task::none()
            }
            Err(e) => {
                app.model_settings.loading = false;
                app.model_settings.save_error = Some(e);
                app.model_settings.detail_modal = None;
                Task::none()
            }
        },
        SettingsMessage::ModelQueryChanged(v) => {
            app.model_settings.query = v;
            Task::none()
        }
        SettingsMessage::ModelToggle(provider_id, model_id, enabled) => {
            if !enabled && !app.auto_model {
                let full = format!("{}/{}", &provider_id, &model_id);
                let is_selected =
                    if app.model.contains('/') { app.model == full } else { app.model == model_id };
                if is_selected {
                    app.auto_model = true;
                    crate::app::set_config_field("auto_model", serde_json::Value::Bool(true));
                }
            }
            app.model_settings.loading = true;
            app.model_settings.save_error = None;
            app.model_settings.detail_modal = None;
            Task::perform(
                async move {
                    let status =
                        if enabled { "active".to_string() } else { "disabled".to_string() };
                    let mut model_cfg = Map::new();
                    model_cfg.insert("id".to_string(), Value::String(model_id.clone()));
                    model_cfg.insert("status".to_string(), Value::String(status));
                    let mut models_obj = Map::new();
                    models_obj.insert(model_id, Value::Object(model_cfg));
                    let mut provider_cfg = Map::new();
                    provider_cfg.insert("models".to_string(), Value::Object(models_obj));
                    let mut provider_obj = Map::new();
                    provider_obj.insert(provider_id.clone(), Value::Object(provider_cfg));
                    let mut root = Map::new();
                    root.insert("providers".to_string(), Value::Object(provider_obj));
                    config::patch_full_agent_config_async(Value::Object(root))
                        .await
                        .map_err(server_config_unreachable_error)?;
                    load_models_for_settings().await.map_err(server_config_unreachable_error)
                },
                |res| Message::Settings(SettingsMessage::ModelsRefreshed(res)),
            )
        }
        SettingsMessage::ModelDetailOpen(provider_id, model_id) => {
            let provider = app.model_settings.providers.iter().find(|p| p.id == provider_id);
            let Some(provider) = provider else {
                app.model_settings.save_error = Some("未找到提供商".to_string());
                return Task::none();
            };
            let model = provider.models.iter().find(|m| m.id == model_id);
            let Some(model) = model else {
                app.model_settings.save_error = Some("未找到模型".to_string());
                return Task::none();
            };

            let detail_value = &model.detail;
            let raw_json = serde_json::to_string_pretty(detail_value)
                .unwrap_or_else(|_| serde_json::to_string(detail_value).unwrap_or_default());

            let get = |k: &str| detail_value.get(k).unwrap_or(&Value::Null);
            let get_str = |v: &Value| v.as_str().unwrap_or_default().to_string();
            let get_u64 = |v: &Value| v.as_u64().unwrap_or(0);
            let yes_no = |b: bool| if b { "支持" } else { "不支持" }.to_string();
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
                if out.is_empty() { "无".to_string() } else { out.join(", ") }
            };

            let mut rows = Vec::<ModelDetailRow>::new();
            rows.push(ModelDetailRow {
                label: "模型名称 (name)".to_string(),
                value: model.name.clone(),
            });
            rows.push(ModelDetailRow {
                label: "模型 ID (id)".to_string(),
                value: model.id.clone(),
            });
            rows.push(ModelDetailRow {
                label: "提供商 (providerID)".to_string(),
                value: provider.id.clone(),
            });
            rows.push(ModelDetailRow {
                label: "状态 (status)".to_string(),
                value: get_str(get("status")),
            });
            rows.push(ModelDetailRow {
                label: "发布日期 (release_date)".to_string(),
                value: get_str(get("release_date")),
            });

            let api = get("api");
            rows.push(ModelDetailRow {
                label: "API Base URL (api.url)".to_string(),
                value: api.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
            });
            rows.push(ModelDetailRow {
                label: "适配器 (api.adapter)".to_string(),
                value: api.get("adapter").and_then(Value::as_str).unwrap_or_default().to_string(),
            });

            let limit = get("limit");
            rows.push(ModelDetailRow {
                label: "上下文限制 (limit.context)".to_string(),
                value: get_u64(limit.get("context").unwrap_or(&Value::Null)).to_string(),
            });
            rows.push(ModelDetailRow {
                label: "输入限制 (limit.input)".to_string(),
                value: limit
                    .get("input")
                    .and_then(Value::as_u64)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            });
            rows.push(ModelDetailRow {
                label: "输出限制 (limit.output)".to_string(),
                value: get_u64(limit.get("output").unwrap_or(&Value::Null)).to_string(),
            });

            let caps = get("capabilities");
            rows.push(ModelDetailRow {
                label: "支持工具调用 (capabilities.toolcall)".to_string(),
                value: yes_no(caps.get("toolcall").and_then(Value::as_bool).unwrap_or(false)),
            });
            rows.push(ModelDetailRow {
                label: "支持附件 (capabilities.attachment)".to_string(),
                value: yes_no(caps.get("attachment").and_then(Value::as_bool).unwrap_or(false)),
            });
            rows.push(ModelDetailRow {
                label: "支持推理 (capabilities.reasoning)".to_string(),
                value: yes_no(caps.get("reasoning").and_then(Value::as_bool).unwrap_or(false)),
            });
            rows.push(ModelDetailRow {
                label: "支持温度 (capabilities.temperature)".to_string(),
                value: yes_no(caps.get("temperature").and_then(Value::as_bool).unwrap_or(false)),
            });
            rows.push(ModelDetailRow {
                label: "输入模态 (capabilities.input.*)".to_string(),
                value: join_caps(caps.get("input").unwrap_or(&Value::Null)),
            });
            rows.push(ModelDetailRow {
                label: "输出模态 (capabilities.output.*)".to_string(),
                value: join_caps(caps.get("output").unwrap_or(&Value::Null)),
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
                    label: "混合输入 (capabilities.interleaved)".to_string(),
                    value: interleaved_str,
                });
            }

            let cost = get("cost");
            let cache = cost.get("cache").unwrap_or(&Value::Null);
            rows.push(ModelDetailRow {
                label: "输入价格 (cost.input)".to_string(),
                value: cost.get("input").and_then(Value::as_f64).unwrap_or(0.0).to_string(),
            });
            rows.push(ModelDetailRow {
                label: "输出价格 (cost.output)".to_string(),
                value: cost.get("output").and_then(Value::as_f64).unwrap_or(0.0).to_string(),
            });
            rows.push(ModelDetailRow {
                label: "缓存读 (cost.cache.read)".to_string(),
                value: cache.get("read").and_then(Value::as_f64).unwrap_or(0.0).to_string(),
            });
            rows.push(ModelDetailRow {
                label: "缓存写 (cost.cache.write)".to_string(),
                value: cache.get("write").and_then(Value::as_f64).unwrap_or(0.0).to_string(),
            });

            let headers_count = get("headers").as_object().map(|o| o.len()).unwrap_or(0);
            let options_count = get("options").as_object().map(|o| o.len()).unwrap_or(0);
            let variants_count = get("variants").as_object().map(|o| o.len()).unwrap_or(0);
            rows.push(ModelDetailRow {
                label: "请求头数量 (headers)".to_string(),
                value: headers_count.to_string(),
            });
            rows.push(ModelDetailRow {
                label: "自定义选项数量 (options)".to_string(),
                value: options_count.to_string(),
            });
            rows.push(ModelDetailRow {
                label: "变体数量 (variants)".to_string(),
                value: variants_count.to_string(),
            });

            app.model_settings.detail_modal = Some(ModelDetailModalState {
                provider_id: provider.id.clone(),
                provider_name: provider.name.clone(),
                model_id: model.id.clone(),
                model_name: model.name.clone(),
                rows,
                raw_json,
                show_raw: false,
            });
            Task::none()
        }
        SettingsMessage::ModelDetailClose => {
            app.model_settings.detail_modal = None;
            Task::none()
        }
        SettingsMessage::ModelDetailToggleRaw => {
            if let Some(m) = &mut app.model_settings.detail_modal {
                m.show_raw = !m.show_raw;
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
