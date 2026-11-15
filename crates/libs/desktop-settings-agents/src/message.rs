use iced::widget::text_editor;
#[derive(Debug, Clone)]
pub enum AgentsMessage {
CatalogTabSelected(bool),
CatalogQueryChanged(String),
CatalogToggle(String, bool),
CatalogCopy(String),
CatalogLaunch(String),
CatalogRuntimeSettings(String),
ProfileOpen(String),
ProfileNameChanged(String),
ProfileDefaultsChanged(config_agent_profile::AgentDefaults),
ProfileModelChanged(
        config_agent_profile::AgentDefault<config_agent_profile::AgentModelReference>,
    ),
ProfileDescriptionAction(text_editor::Action),
ProfileRuntimeChanged(String),
ProfilePromptChanged(usize, String),
ProfilePromptAdd,
ProfilePromptRemove(usize),
ProfilePromptMove(usize, usize),
AddAgentOpened,
DetailTabSelected(String),
ForceModelSelectionToggled(String, bool),
AllowAsSubagentToggled(String, bool),
MaxParallelSubagentsChanged(String, u32),
SystemPromptAction(String, text_editor::Action),
PromptTabSelected(String),
WorkspaceIdentityRestoreDefaultRequested(String),
WorkspaceIdentityEnabledToggled {
        agent_key: String,
        file_name: String,
        enabled: bool,
    },
WorkspaceIdentityEditorAction(String, text_editor::Action),
ApiKeyChanged(String, String),
TemperatureChanged(String, f32),
CompactContextToggled(String, bool),
MaxHistoryMessagesChanged(String, u32),
ParallelToolsToggled(String, bool),
MaxDepthChanged(String, u32),
AgenticToggled(String, bool),
MaxIterationsChanged(String, u32)
}
#[derive(Debug, Clone)]
pub enum AcpMessage {
StartDiagnostic,
TestAgent(String),
InstallAgent(String),
CancelDiagnostic,
EditAgent(String),
CancelEditor,
RestoreDefaultArguments,
RestoreDefaultEnvironment,
ArgumentChanged(usize, String),
AddArgument,
RemoveArgument(usize),
EnvironmentChanged(usize, bool, String),
AddEnvironment,
RemoveEnvironment(usize),
Refresh,
NewAgentPanelToggled,
NewAgentNameChanged(String),
NewAgentCommandChanged(String),
AddAgent,
CopyCommand {
        agent: String,
        command: String,
    },
SetEnabled {
        agent: String,
        enabled: bool,
    }
}
#[derive(Debug, Clone)]
pub enum Event { CopyText(String), Agents(AgentsMessage), Acp(AcpMessage), RefreshMcp, OpenMcp, None }
