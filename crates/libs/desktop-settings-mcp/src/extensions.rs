use gateway_desktop_settings::{
    AcpInstallProgress, AcpInstallReport, McpExtension, McpInstallInstructions,
};
use std::collections::BTreeMap;

#[derive(Default, Debug, Clone)]
pub struct State {
    pub cards: BTreeMap<McpExtension, Card>,
    pub active: Option<ActiveInstall>,
}

#[derive(Default, Debug, Clone)]
pub struct Card {
    pub expanded: bool,
    pub instructions: Option<Result<McpInstallInstructions, String>>,
    pub report: Option<AcpInstallReport>,
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
    Install(McpExtension),
    Progress(String, Option<AcpInstallProgress>),
    Finished(String, Result<AcpInstallReport, String>),
    Copy(String),
}
