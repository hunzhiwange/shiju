use gateway_desktop_settings::{
    AcpInstallProgress, AcpInstallReport, McpExtension, McpInstallInstructions, McpInstallStatus,
};
use std::collections::BTreeMap;

#[derive(Default, Debug, Clone)]
pub struct State {
    pub cards: BTreeMap<McpExtension, Card>,
    pub active: Option<ActiveInstall>,
    pub copy_sequence: u64,
}

#[derive(Default, Debug, Clone)]
pub struct Card {
    pub expanded: bool,
    pub instructions: Option<Result<McpInstallInstructions, String>>,
    pub report: Option<AcpInstallReport>,
    pub status: Option<Result<McpInstallStatus, String>>,
    pub status_loading: bool,
    pub copy_feedback: BTreeMap<InstructionCopy, CopyFeedback>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstructionCopy {
    Command(usize),
    Configuration,
}

#[derive(Default, Debug, Clone)]
pub struct CopyFeedback {
    pub request: u64,
    pub copied: bool,
}

#[derive(Debug, Clone)]
pub struct ActiveInstall {
    pub extension: McpExtension,
    pub id: String,
    pub progress: Option<AcpInstallProgress>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Toggle(McpExtension),
    Instructions(McpExtension, Result<McpInstallInstructions, String>),
    RefreshStatus(McpExtension),
    Status(McpExtension, Result<McpInstallStatus, String>),
    Install(McpExtension),
    Progress(String, Option<AcpInstallProgress>),
    Finished(String, Result<AcpInstallReport, String>),
    Copy(String),
    #[cfg(not(target_arch = "wasm32"))]
    CopyInstruction(McpExtension, InstructionCopy),
    #[cfg(not(target_arch = "wasm32"))]
    InstructionCopied(McpExtension, InstructionCopy, u64, Result<(), String>),
    #[cfg(not(target_arch = "wasm32"))]
    ClearCopyFeedback(McpExtension, InstructionCopy, u64),
}
