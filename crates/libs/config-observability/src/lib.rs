use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 可观测性后端配置（`[observability]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObservabilityConfig {
    /// 可观测性后端类型：`none`、`log`、`prometheus`、`otel`。
    pub backend: String,

    /// OTLP 端点，例如 `http://localhost:4318`；仅在 `backend = "otel"` 时使用。
    #[serde(default)]
    pub otel_endpoint: Option<String>,

    /// 上报给 OTel collector 的服务名，默认值为 `vibewindow`。
    #[serde(default)]
    pub otel_service_name: Option<String>,

    /// 运行时 trace 存储模式：`none`、`rolling`、`full`。
    /// 用于控制是否持久化模型回复和工具调用诊断信息。
    #[serde(default = "default_runtime_trace_mode")]
    pub runtime_trace_mode: String,

    /// 运行时 trace 文件路径；相对路径会在 `workspace_dir` 下解析。
    #[serde(default = "default_runtime_trace_path")]
    pub runtime_trace_path: String,

    /// 当 `runtime_trace_mode = "rolling"` 时保留的最大条目数。
    #[serde(default = "default_runtime_trace_max_entries")]
    pub runtime_trace_max_entries: usize,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            backend: "none".into(),
            otel_endpoint: None,
            otel_service_name: None,
            runtime_trace_mode: default_runtime_trace_mode(),
            runtime_trace_path: default_runtime_trace_path(),
            runtime_trace_max_entries: default_runtime_trace_max_entries(),
        }
    }
}

fn default_runtime_trace_mode() -> String {
    "none".to_string()
}

fn default_runtime_trace_path() -> String {
    "state/runtime-trace.jsonl".to_string()
}

fn default_runtime_trace_max_entries() -> usize {
    200
}
#[cfg(test)]
#[path = "observability_tests.rs"]
mod observability_tests;
