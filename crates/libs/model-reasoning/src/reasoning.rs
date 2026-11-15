//! Verified effort levels and wire formats. A reasoning boolean alone is not evidence
//! of adjustable effort. Unknown models/protocols deliberately have no choices.
//! Sources (checked 2026-10-23):
//! https://developers.openai.com/api/docs/guides/reasoning
//! https://platform.claude.com/docs/en/build-with-claude/effort
//! https://docs.qwencloud.com/api-reference/chat/openai-chat
//! https://help.aliyun.com/en/model-studio/deepseek-api

/// Provider-declared controls. Absence permits legacy inference; an empty list does not.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningOption {
    Effort {
        values: Vec<String>,
    },
    Toggle,
    BudgetTokens {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<i64>,
    },
    #[serde(other)]
    Unknown,
}

/// Provider metadata takes precedence over inference from a model name.
pub fn reasoning_levels(
    model: &str,
    adapter: &str,
    provider: &str,
    url: &str,
    declared: Option<&[ReasoningOption]>,
) -> Vec<String> {
    if !matches!(adapter, "openai" | "openai-compatible" | "openai-responses" | "anthropic") {
        return Vec::new();
    }
    if let Some(declared) = declared {
        let mut levels = Vec::new();
        for option in declared {
            if let ReasoningOption::Effort { values } = option {
                for value in values {
                    if matches!(
                        value.as_str(),
                        "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max"
                    ) && !levels.contains(value)
                    {
                        levels.push(value.clone());
                    }
                }
            }
        }
        return levels;
    }
    reasoning_profile(model, adapter, provider, url)
        .levels
        .iter()
        .map(|level| (*level).to_string())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFormat {
    OpenAi,
    Anthropic,
    DeepSeek,
    DashScope,
}

#[derive(Debug, Clone, Copy)]
pub struct ReasoningProfile {
    pub levels: &'static [&'static str],
    pub format: ReasoningFormat,
}

fn matches_model(id: &str, model: &str) -> bool {
    id == model
        || id.strip_prefix(model).and_then(|rest| rest.strip_prefix('-')).is_some_and(|suffix| {
            !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit() || c == '-')
        })
}

pub fn is_dashscope(provider: &str, url: &str) -> bool {
    matches!(
        provider,
        "token-plan" | "alibaba-cn" | "alibaba" | "alibaba-token-plan" | "alibaba-token-plan-cn"
    ) || url.contains(".aliyuncs.com")
        || url.contains(".qwencloud.com")
}

/// Resolve using the API model ID, never its display name or provider-local alias.
pub fn reasoning_profile(
    model: &str,
    adapter: &str,
    provider: &str,
    url: &str,
) -> ReasoningProfile {
    use ReasoningFormat::*;
    let id = model.to_ascii_lowercase();
    let native_anthropic =
        adapter == "anthropic" || url.trim_end_matches('/').ends_with("/messages");
    if native_anthropic {
        let levels: &[_] = if [
            "claude-opus-4-7",
            "claude-opus-4-8",
            "claude-opus-5",
            "claude-opus-5-5",
            "claude-sonnet-5",
            "claude-sonnet-5-5",
            "claude-haiku-5-5",
            "claude-fable-5",
            "claude-fable-5-1",
            "claude-mythos-5",
            "claude-mythos-5-1",
        ]
        .iter()
        .any(|m| matches_model(&id, m))
        {
            &["low", "medium", "high", "xhigh", "max"]
        } else if ["claude-opus-4-6", "claude-sonnet-4-6", "claude-mythos-preview"]
            .iter()
            .any(|m| matches_model(&id, m))
        {
            &["low", "medium", "high", "max"]
        } else if matches_model(&id, "claude-opus-4-5") {
            &["low", "medium", "high"]
        } else {
            &[]
        };
        return ReasoningProfile { levels, format: Anthropic };
    }
    if !matches!(adapter, "openai" | "openai-compatible" | "openai-responses") {
        return ReasoningProfile { levels: &[], format: OpenAi };
    }
    // Shiju keeps Qwen model IDs while forwarding its native thinking switch.
    let dashscope = is_dashscope(provider, url) || (provider == "shiju" && id.starts_with("qwen"));
    let format = if dashscope { DashScope } else { DeepSeek };
    if ["deepseek-v4-flash-0731", "deepseek-v4-pro-0813", "deepseek-v4.1-flash"]
        .iter()
        .any(|m| matches_model(&id, m))
    {
        return ReasoningProfile { levels: &["none", "low", "high", "max"], format };
    }
    if ["deepseek-v4-flash", "deepseek-v4-pro"].iter().any(|m| matches_model(&id, m)) {
        return ReasoningProfile { levels: &["none", "high", "max"], format };
    }
    if dashscope {
        let levels: &[_] = if [
            "qwen3.8-max",
            "qwen3.8-flash",
            "qwen3.8-2.4t-a95b",
            "qwen3.8-27b",
            "qwen3.8-omni-flash",
        ]
        .iter()
        .any(|m| matches_model(&id, m))
        {
            &["none", "low", "medium", "xhigh"]
        } else if ["glm-5", "glm-5.1", "glm-5.2"].iter().any(|m| matches_model(&id, m)) {
            &["none", "high", "max"]
        } else if matches_model(&id, "glm-5.3") {
            &["low", "high", "max"]
        } else {
            &[]
        };
        return ReasoningProfile { levels, format: DashScope };
    }
    let levels: &[_] = if ["gpt-5.2", "gpt-5.4", "gpt-5.4-mini", "gpt-5.4-nano", "gpt-5.5"]
        .iter()
        .any(|m| matches_model(&id, m))
    {
        &["none", "low", "medium", "high", "xhigh"]
    } else if matches_model(&id, "gpt-5.1") {
        &["none", "low", "medium", "high"]
    } else if ["gpt-5", "gpt-5-mini", "gpt-5-nano"].iter().any(|m| matches_model(&id, m)) {
        &["minimal", "low", "medium", "high"]
    } else if ["o1", "o3", "o3-mini", "o4-mini"].iter().any(|m| matches_model(&id, m)) {
        &["low", "medium", "high"]
    } else {
        &[]
    };
    ReasoningProfile { levels, format: OpenAi }
}

#[cfg(test)]
#[path = "reasoning_tests.rs"]
mod tests;
