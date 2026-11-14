//! Copilot wire differences; credentials and endpoint selection stay with the owner.
use axum::http::{HeaderValue, StatusCode, header};
use serde_json::Value;

pub(crate) struct Context {
    initiator: &'static str,
}

impl Context {
    /// Classify before protocol conversion loses tool-result roles.
    pub(crate) fn new(body: &Value) -> Self {
        let initiator = if let Some(input) = body.get("input") {
            let classify = |item: &Value| match item["type"].as_str() {
                Some("function_call_output" | "custom_tool_call_output" | "tool_search_output") => Some("agent"),
                Some("reasoning" | "function_call" | "custom_tool_call" | "tool_search_call") => None,
                _ => match item["role"].as_str() {
                    Some("assistant" | "system" | "developer") => None,
                    _ => Some("user"),
                },
            };
            match input {
                Value::Array(items) => items.iter().rev().find_map(classify),
                Value::Object(_) => classify(input),
                _ => None,
            }.unwrap_or("user")
        } else {
            let last = body["messages"].as_array().and_then(|items| items.last());
            match last {
                Some(item) if item["role"] == "tool" || item["role"] == "function" => "agent",
                Some(item) if item["role"] == "user" && item["content"].as_array().is_some_and(|blocks| {
                    blocks.iter().any(|block| block["type"] == "tool_result")
                }) => "agent",
                _ => "user",
            }
        };
        Self { initiator }
    }

    /// Called after fresh managed credential resolution. Warmup requests are sent
    /// normally: no local success, extra network probe, or implicit model downgrade.
    pub(crate) fn apply(&self, request: &mut reqwest::Request) -> Result<(), StatusCode> {
        let bytes = request.body().and_then(reqwest::Body::as_bytes).ok_or(StatusCode::BAD_REQUEST)?;
        let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        if let Some(model) = body.get("model").and_then(Value::as_str) {
            body["model"] = normalize_model(model).into();
        }
        *request.body_mut() = Some(serde_json::to_vec(&body).map_err(|_| StatusCode::BAD_REQUEST)?.into());
        request.headers_mut().remove(header::CONTENT_LENGTH);
        request.headers_mut().insert("x-initiator", HeaderValue::from_static(self.initiator));
        Ok(())
    }
}

fn normalize_model(model: &str) -> String {
    let model = model.trim();
    if !model.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("claude-")) {
        return model.to_owned();
    }
    let bracket = model.get(model.len().saturating_sub(4)..).is_some_and(|suffix| suffix.eq_ignore_ascii_case("[1m]"));
    if model.contains('.') && !bracket { return model.to_owned(); }
    let one_m = bracket || model.ends_with("-1m");
    let mut base = if bracket { &model[..model.len()-4] } else if one_m { &model[..model.len()-3] } else { model };
    if let Some((head, date)) = base.rsplit_once('-') {
        if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) { base = head; }
    }
    let dotted = base.rsplit_once('-').and_then(|(head, minor)| {
        let (_, major) = head.rsplit_once('-')?;
        (!minor.is_empty() && !major.is_empty() && minor.bytes().chain(major.bytes()).all(|b| b.is_ascii_digit()))
            .then(|| format!("{head}.{minor}"))
    });
    match dotted {
        Some(mut model) => { if one_m { model.push_str("-1m"); } model },
        None if one_m => format!("{base}-1m"),
        None => model.to_owned(),
    }
}
