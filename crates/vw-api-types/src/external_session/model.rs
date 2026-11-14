//! Structured transcript model, adapted from the pinned cc-switch snapshot.

use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub injected: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<SessionBlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MessageMeta>,
}

impl Serialize for SessionMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire<'a> {
            role: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            ts: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            turn_id: Option<&'a str>,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            injected: bool,
            #[serde(skip_serializing_if = "<[SessionBlock]>::is_empty")]
            blocks: &'a [SessionBlock],
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: Option<&'a MessageMeta>,
        }
        Wire {
            role: &self.role,
            content: self.blocks.is_empty().then_some(self.content.as_str()),
            ts: self.ts,
            id: self.id.as_deref(),
            turn_id: self.turn_id.as_deref(),
            injected: self.injected,
            blocks: &self.blocks,
            meta: self.meta.as_ref(),
        }
        .serialize(serializer)
    }
}

impl SessionMessage {
    pub fn from_blocks(
        role: impl Into<String>,
        ts: Option<i64>,
        blocks: Vec<SessionBlock>,
    ) -> Self {
        Self {
            role: role.into(),
            content: project_content(&blocks),
            ts,
            blocks,
            ..Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty() && self.content.trim().is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SessionBlock {
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

impl SessionBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            full: None,
        }
    }

    pub fn event(kind: EventKind, text: Option<String>, url: Option<String>) -> Self {
        Self::Event {
            kind,
            text,
            url,
            full: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Shell,
    Read,
    Search,
    Edit,
    Write,
    Web,
    Mcp,
    Agent,
    Ask,
    Todo,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    Interrupted,
    Pending,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Aborted,
    Compaction,
    ModelChange,
    ThinkingLevel,
    Hook,
    PrLink,
    SlashCommand,
    Info,
    Error,
    SubAgent,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepPhase {
    Start,
    Finish,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSummary {
    pub files: Vec<DiffFile>,
    pub added: u32,
    pub removed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full: Option<ContentRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffFile {
    pub path: String,
    pub op: DiffOp,
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffOp {
    Add,
    Update,
    Delete,
    Rename,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ContentRef {
    Jsonl {
        offset: u64,
        len: u32,
        pointer: String,
    },
    Sqlite {
        table: String,
        id: String,
        column: String,
        pointer: String,
    },
    File { rel_path: String, pointer: String },
    Sidecar { rel_path: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRef {
    pub source: ImageSource,
    pub media_type: String,
    pub size: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ImageSource {
    Inline { content: ContentRef },
    LocalFile { path: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TranscriptChunk {
    Header {
        total: usize,
        turns: Vec<TurnIndex>,
        cached: bool,
        parse_ms: u64,
    },
    Messages {
        start: usize,
        messages: Vec<SessionMessage>,
    },
    Done {
        payload_bytes: u64,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnIndex {
    pub turn_id: String,
    pub first_message_index: usize,
    pub last_message_index: usize,
    pub question_preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
    pub step_count: u32,
    pub error_count: u32,
    pub has_final_reply: bool,
    pub aborted: bool,
}

pub fn project_content(blocks: &[SessionBlock]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for block in blocks {
        let part = match block {
            SessionBlock::Text { text, .. } => text.clone(),
            SessionBlock::ToolCall {
                raw_name, title, ..
            } => {
                if title.is_empty() {
                    format!("[Tool: {raw_name}]")
                } else {
                    format!("[Tool: {raw_name}] {title}")
                }
            }
            SessionBlock::ToolResult { preview, .. } => preview.clone(),
            SessionBlock::Image { image } => {
                format!("[Image: {} {}]", image.media_type, image.size)
            }
            SessionBlock::Event { text, .. } => text.clone().unwrap_or_default(),
            SessionBlock::Thinking { .. } | SessionBlock::Step { .. } => continue,
        };
        if !part.trim().is_empty() {
            parts.push(part);
        }
    }
    parts.join("\n\n")
}

mod wire;
