use super::models::{self, Model as RawModel, Provider as RawProvider};
use super::types::*;
use serde_json::Value;
use std::collections::HashMap;

/// provider 运行时状态快照。
#[derive(Debug, Default)]
pub struct State {
    pub providers: HashMap<String, Info>,
}

/// 未知模型的保守上下文窗口，避免上下文治理因零值被绕过。
pub const DEFAULT_MODEL_CONTEXT_LIMIT: u64 = 32_768;
/// 未知模型的保守输入窗口，给输出预留默认空间。
pub const DEFAULT_MODEL_INPUT_LIMIT: u64 = 28_672;
/// 未知模型的保守文本输出窗口。
pub const DEFAULT_MODEL_OUTPUT_LIMIT: u64 = 4_096;

/// 按“有效配置值 -> 基线值 -> 集中默认值”解析模型限制。
///
/// `Some(0)` 的 output 保留为“不适用”，其它位置的零值都回退到默认值。
pub fn resolve_model_limit(
    context: Option<u64>,
    input: Option<u64>,
    output: Option<u64>,
) -> ModelLimit {
    ModelLimit {
        context: context.filter(|value| *value > 0).unwrap_or(DEFAULT_MODEL_CONTEXT_LIMIT),
        input: Some(input.filter(|value| *value > 0).unwrap_or(DEFAULT_MODEL_INPUT_LIMIT)),
        output: output.unwrap_or(DEFAULT_MODEL_OUTPUT_LIMIT),
    }
}

/// 计算当前请求可使用的输入窗口。
///
/// 显式输入限制优先；没有输入限制时，从总上下文窗口中预留输出空间。
pub fn effective_input_window(limit: &ModelLimit) -> u64 {
    limit
        .input
        .filter(|value| *value > 0)
        .unwrap_or_else(|| limit.context.saturating_sub(limit.output))
}

/// 判断模型限制是否完全来自未知模型的集中默认值。
pub fn is_default_model_limit(limit: &ModelLimit) -> bool {
    let defaults = resolve_model_limit(None, None, None);
    limit.context == defaults.context
        && limit.input == defaults.input
        && limit.output == defaults.output
}

/// 未填写模型能力时的 OpenAI 兼容文本模型默认能力。
pub fn default_model_capabilities() -> Capabilities {
    Capabilities {
        temperature: false,
        reasoning: false,
        attachment: false,
        toolcall: true,
        input: CapabilityIO {
            text: true,
            audio: false,
            image: false,
            video: false,
            pdf: false,
        },
        output: CapabilityIO {
            text: true,
            audio: false,
            image: false,
            video: false,
            pdf: false,
        },
        interleaved: InterleavedCapability::Bool(false),
    }
}

/// 解析模型级能力覆盖，未显式设置的字段沿用基线或兼容默认值。
pub fn resolve_model_capabilities(
    model_cfg: &Value,
    base: Option<&Capabilities>,
) -> Capabilities {
    let mut capabilities = base.cloned().unwrap_or_else(default_model_capabilities);

    for (key, target) in [
        ("temperature", &mut capabilities.temperature),
        ("reasoning", &mut capabilities.reasoning),
        ("attachment", &mut capabilities.attachment),
        ("tool_call", &mut capabilities.toolcall),
    ] {
        if let Some(value) = model_cfg.get(key).and_then(Value::as_bool) {
            *target = value;
        }
    }

    if let Some(modalities) = model_cfg.get("modalities").and_then(Value::as_object) {
        if let Some(input) = modalities.get("input").and_then(Value::as_array) {
            capabilities.input = capability_io_from_modalities(input);
        }
        if let Some(output) = modalities.get("output").and_then(Value::as_array) {
            capabilities.output = capability_io_from_modalities(output);
        }
    }

    if let Some(interleaved) = model_cfg.get("interleaved")
        && let Ok(interleaved) = serde_json::from_value(interleaved.clone())
    {
        capabilities.interleaved = interleaved;
    }

    capabilities
}

fn capability_io_from_modalities(modalities: &[Value]) -> CapabilityIO {
    let supports = |name: &str| modalities.iter().any(|item| item.as_str() == Some(name));
    CapabilityIO {
        text: supports("text"),
        audio: supports("audio"),
        image: supports("image"),
        video: supports("video"),
        pdf: supports("pdf"),
    }
}

/// 归一化适配器名称，兼容历史别名。
pub fn normalize_adapter(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return default_adapter();
    }
    let lowered = trimmed.to_ascii_lowercase();
    match lowered.as_str() {
        "acp" | "agent-client-protocol" | "agent_client_protocol" => {
            "openai-compatible".to_string()
        }
        _ => trimmed.to_string(),
    }
}

/// 将原始模型定义转换为对外暴露的模型结构。
pub fn from_models_dev_model(provider: &RawProvider, model: &RawModel) -> Model {
    let adapter = model
        .provider
        .as_ref()
        .map(|p| p.adapter.clone())
        .or_else(|| provider.adapter.clone())
        .unwrap_or_else(default_adapter);

    let api = ApiInfo {
        id: model.id.clone(),
        url: provider.api.clone().unwrap_or_default(),
        adapter: normalize_adapter(&adapter),
    };

    let cost = ModelCost {
        input: model.cost.as_ref().map(|c| c.input).unwrap_or(0.0),
        output: model.cost.as_ref().map(|c| c.output).unwrap_or(0.0),
        cache: ModelCostCache {
            read: model.cost.as_ref().and_then(|c| c.cache_read).unwrap_or(0.0),
            write: model.cost.as_ref().and_then(|c| c.cache_write).unwrap_or(0.0),
        },
        experimental_over_200k: model.cost.as_ref().and_then(|c| c.context_over_200k.as_ref()).map(
            |x| ModelCostOver200k {
                input: x.input,
                output: x.output,
                cache: ModelCostCache {
                    read: x.cache_read.unwrap_or(0.0),
                    write: x.cache_write.unwrap_or(0.0),
                },
            },
        ),
    };

    let modalities = model.modalities.as_ref();
    let input = CapabilityIO {
        text: modalities.is_some_and(|m| m.input.iter().any(|s| s == "text")),
        audio: modalities.is_some_and(|m| m.input.iter().any(|s| s == "audio")),
        image: modalities.is_some_and(|m| m.input.iter().any(|s| s == "image")),
        video: modalities.is_some_and(|m| m.input.iter().any(|s| s == "video")),
        pdf: modalities.is_some_and(|m| m.input.iter().any(|s| s == "pdf")),
    };
    let output = CapabilityIO {
        text: modalities.is_some_and(|m| m.output.iter().any(|s| s == "text")),
        audio: modalities.is_some_and(|m| m.output.iter().any(|s| s == "audio")),
        image: modalities.is_some_and(|m| m.output.iter().any(|s| s == "image")),
        video: modalities.is_some_and(|m| m.output.iter().any(|s| s == "video")),
        pdf: modalities.is_some_and(|m| m.output.iter().any(|s| s == "pdf")),
    };

    let interleaved =
        match model.interleaved.clone().unwrap_or(models::ModelInterleaved::Bool(false)) {
            models::ModelInterleaved::Bool(b) => InterleavedCapability::Bool(b),
            models::ModelInterleaved::Field { field } => InterleavedCapability::Field { field },
        };

    let output_limit = if model.limit.output > 0 {
        Some(model.limit.output)
    } else if model.modalities.as_ref().is_some_and(|modalities| {
        !modalities.output.is_empty() && !modalities.output.iter().any(|item| item == "text")
    }) {
        Some(0)
    } else {
        None
    };
    let limit = resolve_model_limit(
        Some(model.limit.context),
        model.limit.input,
        output_limit,
    );

    Model {
        id: model.id.clone(),
        provider_id: provider.id.clone(),
        api,
        name: model.name.clone(),
        family: model.family.clone(),
        capabilities: Capabilities {
            temperature: model.temperature,
            reasoning: model.reasoning,
            attachment: model.attachment,
            toolcall: model.tool_call,
            input,
            output,
            interleaved,
        },
        cost,
        limit,
        status: model.status.clone().unwrap_or_else(|| "active".to_string()),
        headers: model.headers.clone().unwrap_or_default(),
        options: model.options.clone(),
        release_date: model.release_date.clone(),
        variants: model.variants.clone().unwrap_or_default(),
    }
}

/// 将原始 provider 定义转换为对外暴露的 provider 结构。
pub fn from_models_dev_provider(provider: RawProvider) -> Info {
    let models = provider
        .models
        .values()
        .map(|m| (m.id.clone(), from_models_dev_model(&provider, m)))
        .collect::<HashMap<_, _>>();

    Info {
        id: provider.id.clone(),
        source: ProviderSource::Custom,
        name: provider.name.clone(),
        env: provider.env.clone(),
        key: None,
        options: HashMap::new(),
        models,
    }
}

/// 从 JSON 值中提取字符串键值对映射。
pub fn as_string_map(v: &Value) -> HashMap<String, String> {
    v.as_object()
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default()
}

/// 兼容旧调用方的内建模型限制查询入口；默认值由 [`resolve_model_limit`] 统一解析。
pub fn builtin_model_limit(_provider_id: &str, _model_id: &str) -> Option<(u64, u64, Option<u64>)> {
    None
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
