//! Internal formatting helpers.
use super::*;

pub(super) fn should_skip_run_preview_key(key: &str) -> bool {
    matches!(key, "id" | "node_id" | "title" | "status") || is_sensitive_run_preview_key(key)
}

pub(super) fn is_sensitive_run_preview_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.starts_with("__vw_")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower == "authorization"
        || lower == "cookie"
}

pub(super) fn workflow_run_preview_value(
    i18n: desktop_i18n::I18n,
    key: &str,
    value: &serde_json::Value,
) -> Option<String> {
    if value.is_null() {
        return None;
    }

    if is_duration_run_preview_key(key) {
        if let Some(duration) = format_run_preview_duration(i18n, key, value) {
            return Some(duration);
        }
    }

    if is_time_run_preview_key(key) {
        return format_run_preview_timestamp_value(key, value);
    }

    match value {
        serde_json::Value::String(text) => compact_run_preview_text(text),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Array(items) => summarize_run_preview_array(i18n, items),
        serde_json::Value::Object(map) => summarize_run_preview_object(i18n, map),
        serde_json::Value::Null => None,
    }
}

pub(super) fn is_duration_run_preview_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("elapsed") || lower.contains("duration")
}

pub(super) fn is_time_run_preview_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower == "created_at"
        || lower == "started_at"
        || lower == "finished_at"
        || lower == "created_at_ms"
        || lower == "started_at_ms"
        || lower == "finished_at_ms"
}

pub(super) fn format_run_preview_duration(
    i18n: desktop_i18n::I18n,
    key: &str,
    value: &serde_json::Value,
) -> Option<String> {
    let raw = json_number_as_f64(value)?;
    if !raw.is_finite() || raw < 0.0 {
        return None;
    }

    let lower = key.to_ascii_lowercase();
    let millis =
        if lower.ends_with("_ms") || lower.contains("millis") { raw } else { raw * 1000.0 };
    if millis < 1000.0 {
        return Some(
            i18n.format("%s 毫秒", &[(millis.round() as u64).to_string().into()])
                .expect("valid workflow display template"),
        );
    }

    Some(
        i18n.format("%s 秒", &[trim_decimal(millis / 1000.0).to_string().into()])
            .expect("valid workflow display template"),
    )
}

pub(super) fn format_run_preview_timestamp_value(key: &str, value: &serde_json::Value) -> Option<String> {
    let raw = json_number_as_f64(value)?;
    if !raw.is_finite() || raw < 0.0 {
        return None;
    }

    let lower = key.to_ascii_lowercase();
    let millis = if lower.ends_with("_ms") || raw >= 10_000_000_000.0 { raw } else { raw * 1000.0 };
    let millis = millis.round();
    if millis > i64::MAX as f64 {
        return None;
    }

    Local
        .timestamp_millis_opt(millis as i64)
        .single()
        .map(|time| time.format("%Y/%m/%d %H:%M:%S").to_string())
}

pub(super) fn json_number_as_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

pub(super) fn summarize_run_preview_object(
    i18n: desktop_i18n::I18n,
    map: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    if map.is_empty() {
        return None;
    }

    for key in ["text", "answer", "result", "output", "data"] {
        if let Some(value) =
            map.get(key).and_then(|value| workflow_run_preview_value(i18n, key, value))
        {
            return Some(value);
        }
    }

    let mut parts = Vec::new();
    for (key, value) in map {
        if parts.len() >= 3 {
            break;
        }
        if is_sensitive_run_preview_key(key) {
            continue;
        }
        if let Some(value) = workflow_run_preview_value(i18n, key, value) {
            parts.push(format!("{key}={value}"));
        }
    }

    if parts.is_empty() {
        None
    } else if map.len() > parts.len() {
        Some(
            i18n.format("%s 等", &[parts.join(", ").to_string().into()])
                .expect("valid workflow display template"),
        )
    } else {
        Some(parts.join(", "))
    }
}

pub(super) fn summarize_run_preview_array(
    i18n: desktop_i18n::I18n,
    items: &[serde_json::Value],
) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    for value in items.iter().take(3) {
        let Some(value) = workflow_run_preview_value(i18n, "item", value) else {
            continue;
        };
        parts.push(value);
    }

    if parts.is_empty() {
        Some(
            i18n.format("%s 项", &[items.len().to_string().into()])
                .expect("valid workflow display template"),
        )
    } else if items.len() > parts.len() {
        Some(
            i18n.format(
                "%s 等，共 %s 项",
                &[parts.join(", ").to_string().into(), items.len().to_string().into()],
            )
            .expect("valid workflow display template"),
        )
    } else {
        Some(parts.join(", "))
    }
}

pub(super) fn compact_run_preview_text(text: &str) -> Option<String> {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let compact = compact.trim();
    if compact.is_empty() {
        return None;
    }

    if compact.chars().count() <= RUN_PREVIEW_NODE_MAX_VALUE_CHARS {
        Some(compact.to_string())
    } else {
        let mut preview =
            compact.chars().take(RUN_PREVIEW_NODE_MAX_VALUE_CHARS).collect::<String>();
        preview.push_str("...");
        Some(preview)
    }
}

pub(super) fn trim_decimal(value: f64) -> String {
    let mut formatted = format!("{value:.2}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}
