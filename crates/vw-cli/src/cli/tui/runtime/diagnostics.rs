//! Gateway 诊断的只读、受限展示适配边界。

use serde_json::Value;

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use crate::cli::tui::model::integration::sanitize_integration_text;

const MAX_DIAGNOSTIC_RESULTS: usize = 50;
const MAX_CATEGORY_CHARS: usize = 48;
const MAX_MESSAGE_CHARS: usize = 240;
const MAX_ERROR_CHARS: usize = 320;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GatewayDiagnosticItem {
    severity: &'static str,
    category: String,
    message: String,
}

/// 只保留 Gateway doctor 公共结果字段的展示快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GatewayDiagnosticsReport {
    items: Vec<GatewayDiagnosticItem>,
    ok_count: usize,
    warning_count: usize,
    error_count: usize,
}

impl GatewayDiagnosticsReport {
    pub(crate) fn render_text(&self) -> String {
        let mut lines = vec![
            "Gateway diagnostics".to_string(),
            format!(
                "Summary: {} ok · {} warnings · {} errors",
                self.ok_count, self.warning_count, self.error_count
            ),
        ];
        lines.extend(
            self.items
                .iter()
                .map(|item| format!("[{}] {}: {}", item.severity, item.category, item.message)),
        );
        lines.join("\n")
    }
}

pub(crate) fn load_gateway_diagnostics_blocking(
    runtime: &GatewayUiRuntime,
) -> Result<GatewayDiagnosticsReport, String> {
    block_on_gateway(async {
        runtime
            .client()
            .doctor_get()
            .await
            .and_then(|value| diagnostics_report(&value))
            .map_err(|error| sanitize_diagnostics_error(&error))
    })
}

fn diagnostics_report(value: &Value) -> Result<GatewayDiagnosticsReport, String> {
    let results = value
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| "Gateway diagnostics response is missing results".to_string())?;
    let items =
        results.iter().filter_map(diagnostic_item).take(MAX_DIAGNOSTIC_RESULTS).collect::<Vec<_>>();
    let ok_count = items.iter().filter(|item| item.severity == "ok").count();
    let warning_count = items.iter().filter(|item| item.severity == "warn").count();
    let error_count = items.iter().filter(|item| item.severity == "error").count();
    Ok(GatewayDiagnosticsReport { items, ok_count, warning_count, error_count })
}

fn diagnostic_item(value: &Value) -> Option<GatewayDiagnosticItem> {
    let severity = match value.get("severity").and_then(Value::as_str)? {
        "ok" => "ok",
        "warn" => "warn",
        "error" => "error",
        _ => return None,
    };
    let category =
        bounded_sanitized_text(value.get("category").and_then(Value::as_str)?, MAX_CATEGORY_CHARS);
    let message =
        bounded_sanitized_text(value.get("message").and_then(Value::as_str)?, MAX_MESSAGE_CHARS);
    if category.is_empty() || message.is_empty() {
        return None;
    }
    Some(GatewayDiagnosticItem { severity, category, message })
}

pub(crate) fn sanitize_diagnostics_error(error: &str) -> String {
    bounded_sanitized_text(error, MAX_ERROR_CHARS)
}

fn bounded_sanitized_text(text: &str, max_chars: usize) -> String {
    let redacted = redact_bearer_value(text);
    let sanitized = sanitize_integration_text(redacted.as_str());
    truncate_preview(sanitized.as_str(), max_chars)
}

fn redact_bearer_value(text: &str) -> String {
    let mut redact_next = false;
    text.split_whitespace()
        .map(|word| {
            if redact_next {
                redact_next = false;
                return "[redacted]";
            }
            let normalized = word.to_ascii_lowercase();
            if normalized.contains("authorization=") && normalized.ends_with("bearer") {
                redact_next = true;
            }
            word
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return value.chars().take(max_chars).collect();
    }
    let prefix = value.chars().take(max_chars.saturating_sub(3)).collect::<String>();
    format!("{prefix}...")
}
