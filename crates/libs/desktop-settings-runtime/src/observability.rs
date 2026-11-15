use crate::state::*;
use crate::messages::*;
fn normalize_observability_backend(raw: &str) -> String {
    let v = raw.trim().to_ascii_lowercase();
    match v.as_str() {
        "none" | "log" | "prometheus" | "otel" => v,
        _ => "none".to_string(),
    }
}


fn normalize_runtime_trace_mode(raw: &str) -> String {
    let v = raw.trim().to_ascii_lowercase();
    match v.as_str() {
        "none" | "rolling" | "full" => v,
        _ => "none".to_string(),
    }
}

fn persist_observability_settings(state: &mut ObservabilitySettingsState) -> Option<config_observability::ObservabilityConfig> {
    let s = &state;
    let backend = normalize_observability_backend(&s.backend);
    let otel_endpoint = s.otel_endpoint_input.trim().to_string();
    let otel_service_name = s.otel_service_name_input.trim().to_string();
    let runtime_trace_mode = normalize_runtime_trace_mode(&s.runtime_trace_mode);
    let runtime_trace_path = s.runtime_trace_path_input.trim().to_string();
    let runtime_trace_max_entries = s.runtime_trace_max_entries.clamp(1, 100_000) as usize;

    let mut observability = config_observability::ObservabilityConfig::default();
        observability.backend = backend;
        observability.otel_endpoint =
            if otel_endpoint.is_empty() { None } else { Some(otel_endpoint) };
        observability.otel_service_name =
            if otel_service_name.is_empty() { None } else { Some(otel_service_name) };
        observability.runtime_trace_mode = runtime_trace_mode;
        observability.runtime_trace_path = if runtime_trace_path.is_empty() {
            "state/runtime-trace.jsonl".to_string()
        } else {
            runtime_trace_path
        };
        observability.runtime_trace_max_entries = runtime_trace_max_entries;
    Some(observability)
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回需要宿主持久化的配置；仅本地更新或校验失败时返回 None。
pub fn update(state: &mut ObservabilitySettingsState, message: SettingsMessage) -> Option<config_observability::ObservabilityConfig> {
    match message {
        SettingsMessage::ObservabilityBackendChanged(v) => {
            state.backend = v;
            state.backend =
                normalize_observability_backend(&state.backend);
            state.save_error = None;
            persist_observability_settings(state)
        }
        SettingsMessage::ObservabilityOtelEndpointChanged(v) => {
            state.otel_endpoint_input = v;
            state.save_error = None;
            persist_observability_settings(state)
        }
        SettingsMessage::ObservabilityOtelServiceNameChanged(v) => {
            state.otel_service_name_input = v;
            state.save_error = None;
            persist_observability_settings(state)
        }
        SettingsMessage::ObservabilityRuntimeTraceModeChanged(v) => {
            state.runtime_trace_mode = v;
            state.runtime_trace_mode =
                normalize_runtime_trace_mode(&state.runtime_trace_mode);
            state.save_error = None;
            persist_observability_settings(state)
        }
        SettingsMessage::ObservabilityRuntimeTracePathChanged(v) => {
            state.runtime_trace_path_input = v;
            state.save_error = None;
            persist_observability_settings(state)
        }
        SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(v) => {
            state.runtime_trace_max_entries = v.clamp(1, 100_000);
            state.save_error = None;
            persist_observability_settings(state)
        }
        SettingsMessage::ObservabilityHelpOpen => {
            state.show_help_modal = true;
            None
        }
        SettingsMessage::ObservabilityHelpClose => {
            state.show_help_modal = false;
            None
        }
        _ => None,
    }
}
#[cfg(test)]
#[path = "observability_tests.rs"]
mod observability_tests;
