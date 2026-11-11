use vw_gateway_client::{
    AggregationOmoError,
    vw_api_types::agent_apps::aggregation::{OmoSlimState, OmoState},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Standard,
    Slim,
}
#[derive(Clone, Debug)]
pub enum Snapshot {
    Standard(OmoState),
    Slim(OmoSlimState),
}
impl Snapshot {
    pub fn revision(&self) -> &str {
        match self {
            Self::Standard(v) => &v.revision,
            Self::Slim(v) => &v.revision,
        }
    }
    pub fn current(&self) -> Option<&str> {
        match self {
            Self::Standard(v) => v.current_provider_id.as_deref(),
            Self::Slim(v) => v.current_provider_id.as_deref(),
        }
    }
    pub fn sections(&self) -> Vec<(&'static str, String)> {
        let mut rows = Vec::new();
        let (agents, other) = match self {
            Self::Standard(v) => {
                if let Some(c) = &v.categories {
                    rows.push(("Categories", serde_json::to_string_pretty(c).unwrap_or_default()));
                }
                (&v.agents, &v.other_fields)
            }
            Self::Slim(v) => (&v.agents, &v.other_fields),
        };
        if let Some(v) = agents {
            rows.insert(0, ("Agents", serde_json::to_string_pretty(v).unwrap_or_default()));
        }
        if let Some(v) = other {
            rows.push(("其他字段", serde_json::to_string_pretty(v).unwrap_or_default()));
        }
        rows
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Form {
    pub variant: Variant,
    pub binding: Option<(vw_gateway_client::vw_api_types::id::ProjectId, u64)>,
    pub provider: String,
    pub snapshot: Option<Snapshot>,
    pub busy: bool,
    pub error: Option<String>,
}
impl Form {
    pub fn new(variant: Variant) -> Self {
        Self {
            variant,
            binding: None,
            provider: String::new(),
            snapshot: None,
            busy: true,
            error: None,
        }
    }
    pub fn finish(&mut self, result: Result<Snapshot, AggregationOmoError>) {
        self.busy = false;
        match result {
            Ok(v) => {
                self.snapshot = Some(v);
                self.error = None;
            }
            Err(e) => {
                // A failed write may have reached the target: re-read before any retry.
                self.snapshot = None;
                self.error = Some(failure(&e));
            }
        }
    }
}
pub(crate) fn failure(error: &AggregationOmoError) -> String {
    match error {
        AggregationOmoError::Http { status: 401 | 403, .. } => {
            "无权访问此项目的 OMO 配置，请检查授权。"
        }
        AggregationOmoError::Http { status: 409, body }
            if body.pointer("/error/code").and_then(serde_json::Value::as_str)
                == Some("variant_conflict") =>
        {
            "另一变体正在使用：请显式停用该变体，刷新后重试。"
        }
        AggregationOmoError::Http { status: 409, .. } => "配置修订已变化：请刷新实际状态后重试。",
        AggregationOmoError::Http { status: 422, body }
            if matches!(
                body.pointer("/error/code").and_then(serde_json::Value::as_str),
                Some("unsupported_app" | "unsupported_target")
            ) =>
        {
            "目标不支持此 OMO 变体，请检查安装与执行主机。"
        }
        AggregationOmoError::Http { status: 422, .. } => {
            "档案与变体不匹配，或 JSONC 配置损坏/无法安全编辑：检查档案与目标配置后刷新重试。"
        }
        AggregationOmoError::Http { status: 404, .. } => "供应商不存在，请刷新目录后重试。",
        AggregationOmoError::Http { status: 502, .. } => "目标执行失败；请刷新实际状态后重试。",
        AggregationOmoError::Http { status: 503, .. } => "目标主机不可用，请恢复连接后刷新重试。",
        AggregationOmoError::Http { status: 504, .. } => {
            "目标执行超时，结果未确定；请刷新实际状态后重试。"
        }
        _ => "目标请求失败，结果可能未确定；请刷新实际状态后重试。",
    }
    .into()
}
