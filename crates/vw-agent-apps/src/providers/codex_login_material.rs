//! Frozen native auth classification; not an OAuth/token validation service.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexResolvedAuthMode {
    ApiKey,
    Chatgpt,
    ChatgptAuthTokens,
    Headers,
    AgentIdentity,
    PersonalAccessToken,
    BedrockApiKey,
    BedrockAccessKeys,
    /// An `auth_mode` string Codex's serde rejects (`rename_all =
    /// "lowercase"` plus explicit camelCase renames, exact match): the whole
    /// file fails to load, which Codex reports as signed out.
    Unrecognized,
}

pub(super) fn extract_codex_auth_user_identity(auth: &Value) -> Option<String> {
    let id_token = auth.pointer("/tokens/id_token")?.as_str()?;
    extract_codex_id_token_user_identity(id_token)
}
pub(super) fn extract_codex_id_token_user_identity(id_token: &str) -> Option<String> {
    extract_codex_id_token_subject(id_token).map(|subject| format!("sub:{subject}"))
}
pub(super) fn extract_codex_id_token_subject(id_token: &str) -> Option<String> {
    let mut segments = id_token.split('.');
    let header = segments.next()?;
    let payload = segments.next()?;
    segments.next()?;
    if segments.next().is_some() {
        return None;
    }

    let header: Value = URL_SAFE_NO_PAD
        .decode(header)
        .ok()
        .and_then(|decoded| serde_json::from_slice(&decoded).ok())?;
    header.get("alg").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())?;

    let claims: Value = URL_SAFE_NO_PAD
        .decode(payload)
        .ok()
        .and_then(|decoded| serde_json::from_slice(&decoded).ok())?;
    claims
        .get("sub")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
pub(super) fn extract_codex_auth_api_key(auth: &Value) -> Option<String> {
    auth.get("OPENAI_API_KEY")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
}
pub(super) fn codex_auth_has_login_material(auth: &Value) -> bool {
    let Some(obj) = auth.as_object() else {
        return false;
    };

    obj.iter().any(|(key, value)| {
        if key == "auth_mode" {
            return false;
        }

        if key == "OPENAI_API_KEY" {
            return value.as_str().map(str::trim).is_some_and(|token| !token.is_empty());
        }

        match value {
            Value::Null => false,
            Value::String(text) => !text.trim().is_empty(),
            Value::Array(items) => !items.is_empty(),
            Value::Object(map) => !map.is_empty(),
            _ => true,
        }
    })
}
fn codex_auth_resolved_mode(auth: &serde_json::Map<String, Value>) -> CodexResolvedAuthMode {
    let present = |key: &str| auth.get(key).is_some_and(|value| !value.is_null());

    // `auth_mode: null` deserializes to `None` (serde default) and falls
    // through to the implicit precedence below.
    if let Some(mode) = auth.get("auth_mode").filter(|value| !value.is_null()) {
        return match mode.as_str() {
            Some("apikey") => CodexResolvedAuthMode::ApiKey,
            Some("chatgpt") => CodexResolvedAuthMode::Chatgpt,
            Some("chatgptAuthTokens") => CodexResolvedAuthMode::ChatgptAuthTokens,
            Some("headers") => CodexResolvedAuthMode::Headers,
            Some("agentIdentity") => CodexResolvedAuthMode::AgentIdentity,
            Some("personalAccessToken") => CodexResolvedAuthMode::PersonalAccessToken,
            Some("bedrockApiKey") => CodexResolvedAuthMode::BedrockApiKey,
            Some("bedrockAccessKeys") => CodexResolvedAuthMode::BedrockAccessKeys,
            _ => CodexResolvedAuthMode::Unrecognized,
        };
    }
    if present("personal_access_token") {
        return CodexResolvedAuthMode::PersonalAccessToken;
    }
    if present("bedrock_api_key") {
        return CodexResolvedAuthMode::BedrockApiKey;
    }
    if present("bedrock_access_keys") {
        return CodexResolvedAuthMode::BedrockAccessKeys;
    }
    if present("OPENAI_API_KEY") {
        return CodexResolvedAuthMode::ApiKey;
    }
    CodexResolvedAuthMode::Chatgpt
}
pub(super) fn codex_auth_has_openai_account_material(auth: &Value) -> bool {
    let Some(obj) = auth.as_object() else {
        return false;
    };

    let value_present = |value: &Value| match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => true,
    };
    let has = |key: &str| obj.get(key).is_some_and(value_present);

    match codex_auth_resolved_mode(obj) {
        CodexResolvedAuthMode::ApiKey => extract_codex_auth_api_key(auth).is_some(),
        CodexResolvedAuthMode::PersonalAccessToken => has("personal_access_token"),
        CodexResolvedAuthMode::AgentIdentity => has("agent_identity"),
        CodexResolvedAuthMode::Chatgpt | CodexResolvedAuthMode::ChatgptAuthTokens => {
            obj.get("tokens").and_then(Value::as_object).is_some_and(|tokens| {
                ["id_token", "access_token", "refresh_token"]
                    .iter()
                    .any(|key| tokens.get(*key).is_some_and(value_present))
            })
        }
        CodexResolvedAuthMode::Headers
        | CodexResolvedAuthMode::BedrockApiKey
        | CodexResolvedAuthMode::BedrockAccessKeys
        | CodexResolvedAuthMode::Unrecognized => false,
    }
}
pub(super) fn codex_auth_has_credential_login_material(auth: &Value) -> bool {
    let Some(obj) = auth.as_object() else {
        return false;
    };

    let value_present = |value: &Value| match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => true,
    };

    if ["personal_access_token", "agent_identity", "bedrock_api_key"]
        .iter()
        .any(|key| obj.get(*key).is_some_and(value_present))
    {
        return true;
    }

    obj.get("tokens").and_then(Value::as_object).is_some_and(|tokens| {
        ["id_token", "access_token", "refresh_token"]
            .iter()
            .any(|key| tokens.get(*key).is_some_and(value_present))
    })
}
pub(super) fn codex_live_auth_is_stale_third_party_residue(live_auth: &Value) -> bool {
    if codex_auth_has_credential_login_material(live_auth) {
        return false;
    }
    live_auth
        .get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|key| !key.is_empty())
}
