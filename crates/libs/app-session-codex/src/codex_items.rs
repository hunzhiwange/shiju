//! Codex rollout 解析的底层零件：
//!
//! - 借用式反序列化类型：大行（9MB 的 `custom_tool_call_output`、10MB 的 `compacted`）
//!   只解析用得到的字段，base64 图片等不带转义的长字串直接借用行缓冲，不拷贝。
//! - `event_msg.item_completed` 结构化项（CommandExecution / FileChange / McpToolCall /
//!   ImageView / WebSearch / Extension）转成 [`ItemRecord`]，由 `codex.rs` 的状态机
//!   挂到对应的工具调用上。
//! - 工具输出头（`Script completed` / `Wall time` / `Process exited with code` …）、
//!   apply_patch 与 unified diff 的增删统计、data URL 图片引用。

use std::borrow::Cow;
use std::fmt;
use std::sync::LazyLock;

use regex::Regex;
use serde::de::{self, Deserializer, IgnoredAny, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;

use vw_api_external_session::{
    ContentRef, DiffFile, DiffOp, DiffSummary, ImageRef, ImageSource, SessionBlock, ToolKind,
    ToolStatus,
};

use app_session_core::blocks::{
    count_diff_lines, kind_from_codex_parsed_cmd, one_line_title, preview, preview_chars,
    refine_shell_kind, saturating_u32, title_mcp, title_path, title_shell, title_web, Preview,
    INPUT_PREVIEW_CHARS,
};
use app_session_core::pi_jsonl::JsonlSpan;

// ─── 行位置与 ContentRef ─────────────────────────────────────────────────

/// RFC 6901：`~` → `~0`，`/` → `~1`
pub(super) fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

// ─── 借用式反序列化 ───────────────────────────────────────────────────────

/// 字符串字段：没有转义时借用行缓冲，有转义时才分配。
///
/// serde 只对字段类型恰好是 `Cow<str>` 的情况做借用特判，`Option<Cow<str>>`
/// 会退化成总是拷贝；包一层自定义 visitor 才能让 `Option<Str>` 也借用。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Str<'a>(pub Cow<'a, str>);

impl Str<'_> {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for Str<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrVisitor;
        impl<'de> Visitor<'de> for StrVisitor {
            type Value = Str<'de>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string")
            }
            fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(Str(Cow::Borrowed(v)))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Str(Cow::Owned(v.to_owned())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(Str(Cow::Owned(v)))
            }
        }
        deserializer.deserialize_str(StrVisitor)
    }
}

pub(super) fn opt_str<'s>(value: &'s Option<Str<'_>>) -> Option<&'s str> {
    value.as_ref().map(Str::as_str)
}

/// 内容数组里的一项：`input_text` / `output_text` / `input_image` / MCP `image` …
#[derive(Debug, Deserialize)]
pub(super) struct ContentItem<'a> {
    #[serde(rename = "type", default, borrow)]
    pub ty: Option<Str<'a>>,
    #[serde(default, borrow)]
    pub text: Option<Str<'a>>,
    #[serde(default, borrow)]
    pub image_url: Option<Str<'a>>,
    /// MCP 结果里的 base64 图片
    #[serde(default, borrow)]
    pub data: Option<Str<'a>>,
    #[serde(rename = "mimeType", default, borrow)]
    pub mime_type: Option<Str<'a>>,
}

impl ContentItem<'_> {
    pub fn kind(&self) -> &str {
        opt_str(&self.ty).unwrap_or_default()
    }
}

/// `content` / `output` 字段：字符串，或内容项数组。
#[derive(Debug)]
pub(super) enum Contents<'a> {
    Text(Str<'a>),
    Items(Vec<ContentItem<'a>>),
}

impl<'de: 'a, 'a> Deserialize<'de> for Contents<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ContentsVisitor;
        impl<'de> Visitor<'de> for ContentsVisitor {
            type Value = Contents<'de>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or an array of content items")
            }
            fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(Contents::Text(Str(Cow::Borrowed(v))))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Contents::Text(Str(Cow::Owned(v.to_owned()))))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(Contents::Text(Str(Cow::Owned(v))))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Contents::Items(Vec::new()))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Contents::Items(Vec::new()))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element::<ContentItem<'de>>()? {
                    items.push(item);
                }
                Ok(Contents::Items(items))
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                // 未知形状：整体跳过
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(Contents::Items(Vec::new()))
            }
        }
        deserializer.deserialize_any(ContentsVisitor)
    }
}

impl Contents<'_> {
    /// 所有文本项（`input_text` / `output_text` / `text` / `summary_text` …）的
    /// `(下标, 文本)`；字符串形式时下标为 `None`。
    pub fn texts(&self) -> Vec<(Option<usize>, &str)> {
        match self {
            Contents::Text(text) => vec![(None, text.as_str())],
            Contents::Items(items) => items
                .iter()
                .enumerate()
                .filter(|(_, item)| !matches!(item.kind(), "input_image" | "image"))
                .filter_map(|(i, item)| opt_str(&item.text).map(|t| (Some(i), t)))
                .collect(),
        }
    }

    /// 文本项以 `sep` 拼接（跳过空白项）
    pub fn joined_text(&self, sep: &str) -> String {
        self.texts()
            .into_iter()
            .map(|(_, t)| t)
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join(sep)
    }

    /// 文本全文的 JSON Pointer：只有一项时指向该项，否则指向整个字段
    /// （P2 取全文时需把内容项数组的 `text` 拼起来）。
    pub fn text_pointer(&self, field: &str) -> String {
        let texts = self.texts();
        match texts.as_slice() {
            [(Some(i), _)] => format!("{field}/{i}/text"),
            _ => field.to_string(),
        }
    }
}

// ─── item_completed ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(super) struct ItemDuration {
    #[serde(default)]
    pub secs: u64,
    #[serde(default)]
    pub nanos: u64,
}

impl ItemDuration {
    pub fn as_ms(&self) -> u64 {
        self.secs * 1000 + self.nanos / 1_000_000
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ParsedCmd<'a> {
    #[serde(rename = "type", default, borrow)]
    pub ty: Option<Str<'a>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct McpResult<'a> {
    #[serde(default, borrow)]
    pub content: Option<Contents<'a>>,
    #[serde(rename = "isError", default)]
    pub is_error: Option<bool>,
}

/// `event_msg.item_completed.item`：只声明各类型用得到的字段，其余跳过不分配。
#[derive(Debug, Deserialize)]
pub(super) struct RawItem<'a> {
    #[serde(rename = "type", borrow)]
    pub ty: Str<'a>,
    #[serde(default, borrow)]
    pub id: Option<Str<'a>>,
    // CommandExecution
    #[serde(default)]
    pub command: Option<Value>,
    #[serde(default, borrow)]
    pub cwd: Option<Str<'a>>,
    #[serde(default, borrow)]
    pub parsed_cmd: Option<Vec<ParsedCmd<'a>>>,
    #[serde(default, borrow)]
    pub status: Option<Str<'a>>,
    #[serde(default)]
    pub exit_code: Option<i64>,
    #[serde(default)]
    pub duration: Option<ItemDuration>,
    #[serde(default, borrow)]
    pub aggregated_output: Option<Str<'a>>,
    #[serde(default, borrow)]
    pub source: Option<Str<'a>>,
    // FileChange
    #[serde(default)]
    pub changes: Option<serde_json::Map<String, Value>>,
    // McpToolCall
    #[serde(default, borrow)]
    pub server: Option<Str<'a>>,
    #[serde(default, borrow)]
    pub tool: Option<Str<'a>>,
    #[serde(default)]
    pub arguments: Option<Value>,
    #[serde(default, borrow)]
    pub result: Option<McpResult<'a>>,
    // ImageView
    #[serde(default, borrow)]
    pub path: Option<Str<'a>>,
    // WebSearch / Extension
    #[serde(default, borrow)]
    pub kind: Option<Str<'a>>,
    #[serde(default, borrow)]
    pub query: Option<Str<'a>>,
    #[serde(default)]
    pub action: Option<Value>,
    // AgentMessage
    #[serde(default, borrow)]
    pub phase: Option<Str<'a>>,
}

/// 结构化项归一后的记录，挂到工具调用上或单独成消息。
#[derive(Debug, Clone)]
pub(super) enum ItemRecord {
    Command(CommandInfo),
    FileChange(FileChangeInfo),
    Mcp(McpInfo),
    ImageView { id: String, path: String },
    Web(WebInfo),
}

#[derive(Debug, Clone)]
pub(super) struct CommandInfo {
    pub id: String,
    /// 去掉 `/bin/zsh -lc` 包装后的命令
    pub command: String,
    pub cwd: Option<String>,
    pub kind: ToolKind,
    pub status: ToolStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub output: Preview,
    pub output_ref: Option<ContentRef>,
    /// `aggregated_output` 前 200 字符，用于按输出内容兜底配对
    pub output_head: String,
    pub by_user: bool,
}

#[derive(Debug, Clone)]
pub(super) struct FileChangeInfo {
    pub id: String,
    pub diff: DiffSummary,
    pub status: ToolStatus,
}

#[derive(Debug, Clone)]
pub(super) struct McpInfo {
    pub id: String,
    pub server: String,
    pub tool: String,
    pub detail: Option<String>,
    pub input: String,
    pub status: ToolStatus,
    pub duration_ms: Option<u64>,
    pub output: Preview,
    pub output_ref: Option<ContentRef>,
    pub images: Vec<ImageRef>,
}

#[derive(Debug, Clone)]
pub(super) struct WebInfo {
    pub id: String,
    pub title: String,
    pub input: String,
}

/// Codex 的 status 字串 → [`ToolStatus`]
pub(super) fn status_from_str(status: &str) -> ToolStatus {
    match status {
        "completed" | "success" | "succeeded" => ToolStatus::Success,
        "failed" | "error" | "declined" | "rejected" => ToolStatus::Error,
        "interrupted" | "cancelled" | "canceled" | "aborted" => ToolStatus::Interrupted,
        "in_progress" | "pending" | "running" => ToolStatus::Pending,
        _ => ToolStatus::Unknown,
    }
}


mod commands;
pub(super) use commands::*;

mod artifacts;
pub(super) use artifacts::*;

mod records;
pub(super) use records::*;
