//! OMP replacement-node validation adapted from the pinned source schema.
use super::WriteError;
use serde_json::Value;
const APIS: &[&str] = &[
    "openai-completions",
    "openai-responses",
    "openai-codex-responses",
    "azure-openai-responses",
    "anthropic-messages",
    "bedrock-converse-stream",
    "google-generative-ai",
    "google-gemini-cli",
    "google-vertex",
    "openrouter-decisions",
    "openai-decisions",
    "workers-ai-systemone",
    "typesafe",
];

/// Runner APIs a single model may also name (omp's `RUNNER_API_KINDS`, minus web search).
const MODEL_ONLY_APIS: &[&str] = &[
    "openai-images",
    "openrouter-images",
    "xai-tts",
    "openai-speech",
    "openai-embeddings",
    "openrouter-rerank",
    "openrouter-video",
    "openai-transcriptions",
];

fn invalid_provider() -> WriteError {
    WriteError::InvalidResource
}

fn check_type(
    node: &Value,
    keys: &[&str],
    check: impl Fn(&Value) -> bool,
) -> Result<(), WriteError> {
    for key in keys {
        if node.get(key).is_some_and(|v| !check(v)) {
            return Err(invalid_provider());
        }
    }
    Ok(())
}

fn string_map(v: &Value) -> bool {
    v.as_object().is_some_and(|m| m.values().all(Value::is_string))
}

fn check_common(node: &Value, apis: &[&[&str]]) -> Result<(), WriteError> {
    if !node.is_object() {
        return Err(invalid_provider());
    }
    check_type(node, &["baseUrl", "apiKey"], |v| v.as_str().is_some_and(|s| !s.is_empty()))?;
    check_type(node, &["api"], |v| {
        v.as_str().is_some_and(|s| apis.iter().any(|list| list.contains(&s)))
    })?;
    check_type(node, &["headers"], string_map)?;
    if let Some(compat) = node.get("compat") {
        check_compat(compat)?;
        if let Some(when_thinking) = compat.get("whenThinking") {
            check_compat(when_thinking)?;
        }
    }
    Ok(())
}

/// Enumerated `compat` fields, as omp v18.12's models.yml schema lists them. A Pi value
/// such as `thinkingFormat: deepseek` would make omp drop every custom provider.
const COMPAT_ENUMS: &[(&str, &[&str])] = &[
    ("maxTokensField", &["max_completion_tokens", "max_tokens"]),
    ("reasoningContentField", &["reasoning_content", "reasoning", "reasoning_text"]),
    ("thinkingFormat", &["openai", "openrouter", "zai", "qwen", "qwen-chat-template"]),
    ("cacheControlFormat", &["anthropic"]),
    ("toolStrictMode", &["all_strict", "none"]),
    ("streamMarkupHealingPattern", &["kimi", "dsml", "qwen", "thinking"]),
    ("promptCacheMode", &["none", "automatic", "explicit"]),
];

fn check_compat(compat: &Value) -> Result<(), WriteError> {
    if !compat.is_object() {
        return Err(invalid_provider());
    }
    check_type(
        compat,
        &[
            "supportsStore",
            "supportsDeveloperRole",
            "supportsReasoningEffort",
            "supportsUsageInStreaming",
            "requiresToolResultName",
            "requiresMistralToolIds",
            "requiresAssistantAfterToolResult",
            "requiresThinkingAsText",
            "supportsStrictMode",
        ],
        Value::is_boolean,
    )?;
    for (key, allowed) in COMPAT_ENUMS {
        check_type(compat, &[key], |v| v.as_str().is_some_and(|s| allowed.contains(&s)))?;
    }
    if let Some(map) = compat.get("reasoningEffortMap") {
        if !map.is_object() {
            return Err(invalid_provider());
        }
        // omp declares these six levels and keeps any other key as is.
        check_type(map, &["minimal", "low", "medium", "high", "xhigh", "max"], Value::is_string)?;
    }
    Ok(())
}

/// `discovery.type` values omp v18.12 accepts.
const DISCOVERY_TYPES: &[&str] = &[
    "ollama",
    "llama.cpp",
    "lm-studio",
    "openai-models-list",
    "proxy",
    "litellm",
    "apple-foundation-models",
];

/// Validate the one node CC Switch writes. Other providers in the file are never judged.
pub(crate) fn validate_provider(id: &str, node: &Value) -> Result<(), WriteError> {
    if id.trim().is_empty() {
        return Err(invalid_provider());
    }
    check_common(node, &[APIS])?;
    check_type(node, &["auth"], |v| matches!(v.as_str(), Some("apiKey" | "none" | "oauth")))?;
    check_type(node, &["authHeader", "disableStrictTools"], Value::is_boolean)?;
    let models = match node.get("models") {
        None => &[][..],
        Some(v) => v.as_array().ok_or_else(invalid_provider)?.as_slice(),
    };
    if models.is_empty() {
        // An override-only node must still override something (omp's own rule).
        let overrides = [
            "baseUrl",
            "headers",
            "apiKey",
            "compat",
            "disableStrictTools",
            "guardrailIdentifier",
            "requestMetadata",
            "remoteCompaction",
            "discovery",
        ];
        if !overrides.iter().any(|k| {
            node.get(k).is_some_and(|value| match value {
                Value::Bool(b) => *b,
                Value::String(s) => !s.is_empty(),
                _ => !value.is_null(),
            })
        }) && node.get("auth").and_then(Value::as_str) != Some("none")
            && node.get("modelOverrides").and_then(Value::as_object).is_none_or(|m| m.is_empty())
        {
            return Err(invalid_provider());
        }
    } else if node.get("baseUrl").is_none()
        || (node.get("apiKey").is_none()
            && !matches!(node.get("auth").and_then(Value::as_str), Some("none" | "oauth")))
    {
        return Err(invalid_provider());
    }
    for model in models {
        check_common(model, &[APIS, MODEL_ONLY_APIS])?;
        if model.get("id").and_then(Value::as_str).is_none_or(|id| id.is_empty())
            || (model.get("api").is_none() && node.get("api").is_none())
        {
            return Err(invalid_provider());
        }
        check_type(model, &["input"], |v| {
            v.as_array()
                .is_some_and(|a| a.iter().all(|v| matches!(v.as_str(), Some("text" | "image"))))
        })?;
        check_type(model, &["contextWindow", "maxTokens"], |v| {
            v.as_f64().is_some_and(|n| n > 0.0)
        })?;
        if let Some(cost) = model.get("cost")
            && (!cost.is_object()
                || ["input", "output", "cacheRead", "cacheWrite"]
                    .iter()
                    .any(|k| cost.get(k).is_none_or(|v| !v.is_number())))
        {
            return Err(invalid_provider());
        }
    }
    // Proxy discovery supplies its API at runtime, as in omp's native validator.
    if let Some(discovery) = node.get("discovery") {
        let kind = discovery.get("type").and_then(Value::as_str);
        if !kind.is_some_and(|kind| DISCOVERY_TYPES.contains(&kind))
            || (kind != Some("proxy") && node.get("api").is_none())
            || (discovery.get("injectV1").is_some() && kind != Some("openai-models-list"))
        {
            return Err(invalid_provider());
        }
        check_type(discovery, &["injectV1"], Value::is_boolean)?;
        check_type(discovery, &["timeoutMs"], |v| {
            v.as_f64().is_some_and(|n| n > 0.0 && n.is_finite())
        })?;
    }
    Ok(())
}
