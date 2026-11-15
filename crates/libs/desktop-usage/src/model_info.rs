//! Model selection and usage limits.
use crate::view_types::UsageModelInfo;
/// 依据模型命名约定选择用量提供商，无法精确匹配时稳定回退到首项。
pub fn pick_usage_provider_id(candidates: &[String], model_id: &str) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }
    let first_segment = model_id.split('/').next().unwrap_or_default();
    let first_segment_lower = first_segment.to_ascii_lowercase();
    if !first_segment.is_empty() {
        if let Some(v) = candidates.iter().find(|id| id.as_str() == first_segment) {
            return Some(v.clone());
        }
        if let Some(v) =
            candidates.iter().find(|id| id.to_ascii_lowercase().contains(&first_segment_lower))
        {
            return Some(v.clone());
        }
    }

    let model_lower = model_id.to_ascii_lowercase();
    let prefer = if model_lower.starts_with("gpt-") || model_lower.starts_with('o') {
        Some("openai")
    } else if model_lower.contains("claude") {
        Some("anthropic")
    } else if model_lower.contains("deepseek") {
        Some("deepseek")
    } else {
        None
    };
    if let Some(substr) = prefer
        && let Some(v) = candidates.iter().find(|id| id.to_ascii_lowercase().contains(substr))
    {
        return Some(v.clone());
    }
    if model_lower.contains("deepseek")
        && let Some(v) = candidates.iter().find(|id| id.to_ascii_lowercase().contains("openrouter"))
    {
        return Some(v.clone());
    }
    Some(candidates[0].clone())
}

pub async fn load(model: Option<String>) -> Option<UsageModelInfo> {
    let parsed = match model {
        Some(s) => {
            let s: String = s;
            if s.contains('/') {
                let parsed = model_types::parse_model(&s);
                if provider_resolution::get_model(&parsed.provider_id, &parsed.model_id)
                    .await
                    .is_ok()
                {
                    Some(parsed)
                } else {
                    let providers = provider_resolution::list().await;
                    let mut candidates = Vec::<String>::new();
                    for (provider_id, info) in providers {
                        if info.models.contains_key(&s) {
                            candidates.push(provider_id);
                        }
                    }
                    pick_usage_provider_id(&candidates, &s).map(|provider_id| {
                        model_types::ParsedModelRef {
                            provider_id,
                            model_id: s.clone(),
                        }
                    })
                }
            } else {
                let providers = provider_resolution::list().await;
                let mut candidates = Vec::<String>::new();
                for (provider_id, info) in providers {
                    if info.models.contains_key(&s) {
                        candidates.push(provider_id);
                    }
                }
                pick_usage_provider_id(&candidates, &s).map(|provider_id| {
                    model_types::ParsedModelRef { provider_id, model_id: s.clone() }
                })
            }
        }
        None => provider_resolution::default_model().await.ok(),
    };
    let Some(parsed) = parsed else {
        return None;
    };
    let m =
        provider_resolution::get_model(&parsed.provider_id, &parsed.model_id).await.ok()?;
    let provider_name = provider_resolution::get_provider(&parsed.provider_id)
        .await
        .map(|p| p.name)
        .unwrap_or_else(|| parsed.provider_id.clone());
    Some(UsageModelInfo {
        provider_id: parsed.provider_id,
        provider_name,
        model_id: m.id.clone(),
        model_name: m.name,
        context_limit: m.limit.context,
        input_limit: model_state::effective_input_window(&m.limit),
        output_limit: m.limit.output,
        limit_is_default: model_state::is_default_model_limit(
            &m.limit,
        ),
        cost_input_per_million: m.cost.input,
        cost_output_per_million: m.cost.output,
        cost_cache_read_per_million: m.cost.cache.read,
        cost_cache_write_per_million: m.cost.cache.write,
    })

}
