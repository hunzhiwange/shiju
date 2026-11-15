//! Claims from trusted token material; this does not verify JWT signatures.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;

pub fn extract_codex_auth_user_identity(auth: &Value) -> Option<String> {
    let id_token = auth.pointer("/tokens/id_token")?.as_str()?;
    extract_codex_id_token_user_identity(id_token)
}
fn extract_codex_id_token_user_identity(id_token: &str) -> Option<String> {
    extract_codex_id_token_subject(id_token).map(|subject| format!("sub:{subject}"))
}
fn extract_codex_id_token_subject(id_token: &str) -> Option<String> {
    parse_codex_jwt_claims(id_token)?
        .get("sub")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

// Read claims from a token already obtained through a trusted channel.
pub fn parse_codex_jwt_claims(token: &str) -> Option<Value> {
    let mut segments = token.split('.');
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

    URL_SAFE_NO_PAD.decode(payload).ok().and_then(|decoded| serde_json::from_slice(&decoded).ok())
}
pub fn workspace_id(claims: &Value) -> Option<&str> {
    claims
        .get("chatgpt_account_id")
        .and_then(Value::as_str)
        .or_else(|| claims.get("https://api.openai.com/auth")?.get("chatgpt_account_id")?.as_str())
}

pub fn token_subject(claims: &Value) -> Option<&str> {
    claims
        .get("sub")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|subject| !subject.is_empty() && !subject.chars().any(char::is_control))
}
