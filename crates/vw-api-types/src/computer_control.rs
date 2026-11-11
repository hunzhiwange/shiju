//! Shared computer-control wire contracts; these reports never grant execution authority.
//!
//! Hosts must recheck session ownership, target identity, focus and permission version
//! immediately before execution. An enabled setting is not evidence of readiness.

use serde::{Deserialize, Serialize};

macro_rules! control_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
    };
}

control_id!(ControlSessionId);
control_id!(ControlRequestId);
control_id!(ControlDocumentId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PermissionVersion(pub u64);

/// One report per feature, including independent Excel and Word reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerControlStatus {
    pub permission_version: PermissionVersion,
    pub native_apps: ControlFeatureStatus,
    pub chrome: ControlFeatureStatus,
    pub excel: ControlFeatureStatus,
    pub word: ControlFeatureStatus,
    pub locked_use: LockedUseStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlFeatureStatus {
    pub enabled: bool,
    /// Host-probed summary, never inferred solely from `enabled` by consumers.
    pub readiness: ControlReadiness,
    pub system_authorization: SystemAuthorization,
    pub connection: ControlConnection,
    pub backend: ControlBackend,
    /// Actual capabilities advertised by the connected backend, not requested features.
    pub capabilities: Vec<ControlCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlReadiness {
    Disabled,
    NotInstalled,
    Unauthorized,
    Disconnected,
    Ready,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemAuthorization {
    NotChecked,
    NotRequired,
    Granted,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlConnection {
    Disconnected,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlBackend {
    NotInstalled,
    Available,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlCapability {
    Observe,
    Screenshot,
    Input,
    StructuredRead,
    StructuredWrite,
    LockedExecution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedUseStatus {
    pub status: ControlFeatureStatus,
    /// Independent mechanism probe; a setting or unlocked execution cannot satisfy it.
    pub probe: LockedUseProbe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockedUseProbe {
    NotProbed,
    Supported,
    Unsupported,
}

/// Stable backend identities; mutable window/document titles are not authorization keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComputerControlTarget {
    NativeApp { bundle_id: String, process_id: u32 },
    ChromeTab { tab_id: u64, origin: String },
    ExcelWorkbook { document_id: ControlDocumentId },
    WordDocument { document_id: ControlDocumentId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerControlRequest {
    pub request_id: ControlRequestId,
    pub session_id: ControlSessionId,
    pub permission_version: PermissionVersion,
    pub target: ComputerControlTarget,
    pub action: ComputerControlAction,
}

/// Closed command vocabulary: no shell, script, JavaScript or arbitrary Office.run payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComputerControlAction {
    NativeObserve { screenshot: bool },
    NativeInput { input: NativeInput },
    Chrome { command: ChromeCommand },
    Excel { command: ExcelCommand },
    Word { command: WordCommand },
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeInput {
    Activate,
    Click { x: i32, y: i32, button: MouseButton },
    Scroll { delta_x: i32, delta_y: i32 },
    Press { key: String, modifiers: Vec<KeyModifier> },
    TypeText { text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyModifier {
    Shift,
    Control,
    Alt,
    Meta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChromeCommand {
    Snapshot,
    Screenshot,
    Navigate {
        url: String,
    },
    Click {
        selector: String,
    },
    Fill {
        selector: String,
        text: String,
    },
    Press {
        key: String,
        modifiers: Vec<KeyModifier>,
    },
    Scroll {
        delta_x: i32,
        delta_y: i32,
    },
    /// Bounded bytes supplied by an independently authorized caller; never a local path.
    Upload {
        selector: String,
        file_name: String,
        content_base64: String,
    },
    /// Read a same-origin link into bounded bytes; redirects and filesystem writes are forbidden.
    Download {
        selector: String,
        url: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExcelCommand {
    ReadRange { sheet: String, range: String },
    WriteRange { sheet: String, range: String, values: Vec<Vec<OfficeCellValue>> },
}

/// Only primitive cell values, not executable code or opaque object commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OfficeCellValue {
    Text(String),
    Number(serde_json::Number),
    Boolean(bool),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WordCommand {
    ReadDocument,
    FindText { text: String },
    ReplaceText { search: String, replacement: String },
    InsertText { position: WordInsertPosition, text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordInsertPosition {
    Start,
    End,
    Selection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerControlResult {
    pub request_id: ControlRequestId,
    pub session_id: ControlSessionId,
    pub permission_version: PermissionVersion,
    pub outcome: ControlOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControlOutcome {
    Succeeded { output: ControlOutput },
    Rejected { error: ControlErrorCode },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControlOutput {
    Acknowledged,
    ChromeDownload { content_base64: String },
    Observation { text: String, screenshots: Vec<ScreenshotReference> },
    ExcelRange { sheet: String, range: String, values: Vec<Vec<OfficeCellValue>> },
    WordText { text: String },
}

/// Sanitized failures: raw backend errors, credentials and request bodies do not belong here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlErrorCode {
    PermissionDenied,
    StalePermission,
    TargetChanged,
    Disconnected,
    Unsupported,
    Cancelled,
    InvalidRequest,
    BackendFailure,
}

pub const MAX_SCREENSHOT_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenshotMimeType {
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/jpeg")]
    Jpeg,
}

/// Opaque host-managed media reference, resolvable to image content for the model.
/// The media resolver must check session ownership and actual bytes before delivery;
/// this DTO bounds declared size. It is neither a local path nor a bearer credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ScreenshotReferenceWire")]
pub struct ScreenshotReference {
    media_id: String,
    mime_type: ScreenshotMimeType,
    byte_length: u64,
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScreenshotReferenceWire {
    media_id: String,
    mime_type: ScreenshotMimeType,
    byte_length: u64,
    width: u32,
    height: u32,
}

impl ScreenshotReference {
    pub fn new(
        media_id: String,
        mime_type: ScreenshotMimeType,
        byte_length: u64,
        width: u32,
        height: u32,
    ) -> Result<Self, &'static str> {
        if media_id.is_empty()
            || media_id.len() > 128
            || !media_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("invalid screenshot media ID");
        }
        if byte_length == 0 || byte_length > MAX_SCREENSHOT_BYTES {
            return Err("invalid screenshot byte length");
        }
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err("invalid screenshot dimensions");
        }
        Ok(Self { media_id, mime_type, byte_length, width, height })
    }

    pub fn media_id(&self) -> &str {
        &self.media_id
    }

    pub fn mime_type(&self) -> ScreenshotMimeType {
        self.mime_type
    }

    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl TryFrom<ScreenshotReferenceWire> for ScreenshotReference {
    type Error = &'static str;

    fn try_from(value: ScreenshotReferenceWire) -> Result<Self, Self::Error> {
        Self::new(value.media_id, value.mime_type, value.byte_length, value.width, value.height)
    }
}
