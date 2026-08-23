//! 验证可观测性后端、运行时追踪及可选 OTLP 字段的配置默认值。
//! 空配置不主动上报遥测，显式端点则必须原样保留供运行时连接。

// 默认关闭外部后端和追踪输出，避免未配置时产生额外 I/O。
#[test]
fn observability_defaults_are_stable() {
    let config = super::ObservabilityConfig::default();
    assert_eq!(config.backend, "none");
    assert_eq!(config.runtime_trace_mode, "none");
    assert_eq!(config.runtime_trace_path, "state/runtime-trace.jsonl");
    assert_eq!(config.runtime_trace_max_entries, 200);
}

// OTLP 端点与服务名是可选扩展，不能影响其它默认追踪字段。
#[test]
fn observability_deserializes_optional_otlp_fields() {
    let parsed: super::ObservabilityConfig = serde_json::from_value(serde_json::json!({
        "backend": "otel",
        "otel_endpoint": "http://localhost:4318",
        "otel_service_name": "vw"
    }))
    .unwrap();

    assert_eq!(parsed.otel_endpoint.as_deref(), Some("http://localhost:4318"));
    assert_eq!(parsed.otel_service_name.as_deref(), Some("vw"));
}
