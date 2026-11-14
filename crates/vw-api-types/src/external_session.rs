//! External application history contracts. Source locators are data, never read authority.

use crate::ProjectScope;
use serde::{Deserialize, Serialize};

mod model;
pub use model::*;

/// Kept distinct from ShiJu SessionId and ACP resume identities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExternalSessionId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionProvider {
    Claude,
    Codex,
    Opencode,
    Hermes,
    Pi,
    Gemini,
    Openclaw,
    Grokbuild,
    Kimi,
    Omp,
    Mcode,
    Dsh,
    Agy,
}

/// Resolved again by the execution host under its trusted application roots.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum SessionSource {
    File { relative_path: String },
    Sqlite { database: String, record_id: String },
}

/// Every operation is bound to a project, execution host and native application instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalSessionIdentity {
    pub scope: ProjectScope,
    pub target_host: String,
    pub instance_id: String,
    pub provider: SessionProvider,
    pub session_id: ExternalSessionId,
    pub source: SessionSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCapabilities {
    pub read: bool,
    pub delete: bool,
    pub resume: bool,
    pub export_markdown: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMeta {
    pub identity: ExternalSessionIdentity,
    pub capabilities: SessionCapabilities,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub project_dir: Option<String>,
    /// Unix milliseconds.
    pub created_at: Option<i64>,
    pub last_active_at: Option<i64>,
    /// Display only. Never accepted as an operation's path or command.
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSessionErrorCode {
    InvalidInput,
    NotFound,
    Unsupported,
    Forbidden,
    Conflict,
    CursorExpired,
    SourceChanged,
    StorageUnavailable,
    HostUnreachable,
    Cancelled,
}

/// Safe public error; no SQL, filesystem contents or nested error chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSessionError {
    pub code: ExternalSessionErrorCode,
    pub message: String,
    pub correlation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSessionRequest {
    pub identity: ExternalSessionIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum DeleteSessionResult {
    Deleted,
    AlreadyAbsent,
    Failed { error: ExternalSessionError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSessionOutcome {
    pub identity: ExternalSessionIdentity,
    pub result: DeleteSessionResult,
}

/// Metadata-only directory query. Execution roots are never request inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionCatalogQuery {
    pub project_id: crate::ProjectId,
    pub target_host_id: String,
    pub provider: Option<SessionProvider>,
    pub instance_id: Option<String>,
    pub project_dir: Option<String>,
    #[serde(default)]
    pub search: String,
    pub page_size: usize,
    pub cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCatalogPage {
    pub sessions: Vec<SessionMeta>,
    pub total: usize,
    pub next_cursor: Option<String>,
    pub groups: Vec<SessionCatalogGroup>,
    pub truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCatalogGroup {
    pub provider: SessionProvider,
    pub instance_id: String,
    pub project_dir: Option<String>,
    pub count: usize,
}

/// One-shot history read. Source fields are matched against host-discovered records.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionMessagesRequest {
    pub identity: ExternalSessionIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMessagesResponse {
    pub session: SessionMeta,
    pub messages: Vec<SessionMessage>,
    pub turns: Vec<TurnIndex>,
    pub cached: bool,
    pub payload_bytes: usize,
    pub max_payload_bytes: usize,
    pub max_messages: usize,
}

/// Client-generated opaque read ID; scoped to the authenticated principal and session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionStreamRequest {
    pub identity: ExternalSessionIdentity,
    pub read_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStreamFrame {
    pub read_id: String,
    pub sequence: usize,
    pub chunk: TranscriptChunk,
}

/// Reference must match a text reference in the host-resolved transcript.
/// Offsets and lengths count Unicode scalar values, not UTF-8 bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionContentRequest {
    pub identity: ExternalSessionIdentity,
    pub content_ref: ContentRef,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionContentResponse {
    pub text: String,
    pub total_len: u32,
    pub truncated: bool,
    pub next_offset: Option<u32>,
}

/// Image reference must belong to the host-resolved transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionImageRequest {
    pub identity: ExternalSessionIdentity,
    pub image: ImageRef,
}

/// Native SDK result; the HTTP endpoint transports raw bytes with Content-Type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionImageResponse {
    pub bytes: Vec<u8>,
    pub media_type: String,
}

/// Terminal choice only; templates and native commands remain execution-host owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTerminal { Terminal, Iterm2, Ghostty, Kitty, Wezterm, Kaku, Otty, Alacritty, Warp, Custom }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionResumeRequest {
    pub identity: ExternalSessionIdentity,
    pub terminal: SessionTerminal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResumeResponse { pub completed: bool }

mod maintenance;
pub use maintenance::*;

/// Native compression state and stored bytes from the authorized Codex instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCompressionState {
    pub enabled: bool,
    pub active_bytes: u64,
    pub archived_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionCompressionUpdate {
    pub target: HistoryMaintenanceTarget,
    pub enabled: bool,
}

/// Export host-resolved history; saving and clipboard access belong to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionMarkdownRequest {
    pub identity: ExternalSessionIdentity,
    pub turn_key: Option<String>,
    #[serde(default)]
    pub include_thinking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMarkdownResponse {
    pub markdown: String,
}

/// Host-built POSIX command for manual use on the authorized host; contains no credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResumeCommandResponse { pub command: String }
