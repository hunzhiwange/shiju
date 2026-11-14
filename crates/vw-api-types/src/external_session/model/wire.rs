use super::*;
#[derive(Serialize)]
#[serde(
    remote = "SessionBlock",
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum SessionBlockWire {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full: Option<ContentRef>,
    },
    Thinking {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        redacted: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full: Option<ContentRef>,
    },
    ToolCall {
        id: String,
        raw_name: String,
        kind: ToolKind,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        server: Option<String>,
        input_preview: String,
        input_total_len: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input_full: Option<ContentRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diff: Option<DiffSummary>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        by_user: bool,
    },
    ToolResult {
        call_id: String,
        status: ToolStatus,
        preview: String,
        total_len: u32,
        line_count: u32,
        truncated: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full: Option<ContentRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        images: Vec<ImageRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        saved_path: Option<String>,
    },
    Image {
        image: ImageRef,
    },
    Event {
        kind: EventKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full: Option<ContentRef>,
    },
    Step {
        phase: StepPhase,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cost_usd: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

impl Serialize for SessionBlock {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Thinking { redacted: true, duration_ms, .. } => {
                let safe = Self::Thinking {
                    text: String::new(), summary: None, redacted: true,
                    duration_ms: *duration_ms, full: None,
                };
                SessionBlockWire::serialize(&safe, serializer)
            }
            _ => SessionBlockWire::serialize(self, serializer),
        }
    }
}
