//! 定义系统设置页面的消息枚举，作为 UI 事件与状态更新之间的显式契约。

use crate::app::state::{
    AutonomyListKind, AutonomySettingsTab, ChannelsSettingsTab, CronAddJobType,
    CronAddScheduleKind, CronSettingsTab, CustomProviderModelFilter, CustomProviderModelTab,
    CustomProviderTab, GatewaySettingsTab, McpNewServerTransport, McpSettingsTab,
    MemorySettingsTab, ModelCatalogEntry, SecurityListKind, SecuritySettingsTab, ThemeEditorTab,
};
use crate::app::{SettingsTab, components::system_settings::SystemTab};
use crate::app::components::system_settings_search::SystemSettingsSearchTarget;
use iced::widget::text_editor;
use vw_config_types::security::{AutonomyLevel, ShellRedirectPolicy};
use vw_config_types::ui::{QuickMessageGroup, QuickMessageKind};

use super::types::{
    AgentsLoaded, DialogueFlowPermissionLoaded, DialogueFlowUiSettingsLoaded, ModelStatusChange,
    HookAuditLoaded, HookCatalogLoaded, HookMutationCompleted, ModelsLoaded, PluginsLoaded,
    ProvidersLoaded, SkillsLoaded, StaffLoaded,
};

/// Embedding 路由设置页消息。
#[derive(Debug, Clone)]
pub enum EmbeddingRoutesMessage {
    AddRoute,
    InsertAlibabaTemplate,
    InsertDoubaoTemplate,
    InsertShijuTemplate,
    RemoveRoute(usize),
    PatternChanged(usize, String),
    ProviderChanged(usize, String),
    ModelChanged(usize, String),
    DimensionsChanged(usize, String),
    ApiKeyChanged(usize, String),
    Save,
}

#[derive(Debug, Clone, Copy)]
pub enum BashPolicyCheck {
    Enabled,
    ReadOnly,
    UnsafeShellPatterns,
    CommandRules,
    WorkspacePaths,
    ForbiddenPaths,
    Risk,
    Redirects,
}

#[derive(Debug, Clone, Copy)]
pub enum BashValidatorCheck {
    Enabled,
    EmptyCommand,
    Heredoc,
    GitCommit,
    Jq,
    Metacharacters,
    DangerousVars,
    Substitution,
    Redirection,
    NewlineInjection,
    IfsInjection,
    ProcEnviron,
    Malformed,
    ObfuscatedFlags,
    Backslash,
    BraceExpansion,
    UnicodeWhitespace,
    HashComment,
    CommentQuoteDesync,
    QuotedNewline,
    ZshDangerous,
    ControlChars,
}

#[cfg(test)]
#[path = "messages_tests.rs"]
mod messages_tests;

/// 模型路由设置页消息。
#[derive(Debug, Clone)]
pub enum ModelRoutesMessage {
    Refresh,
    AddRoute,
    RemoveRoute(usize),
    PatternChanged(usize, String),
    ProviderChanged(usize, String),
    ModelChanged(usize, String),
    PriorityChanged(usize, String),
}

/// 钩子设置页消息。
#[derive(Debug, Clone)]
pub enum HooksMessage {
    EnabledToggled(bool),
    CommandLoggerToggled(bool),
    PluginHooksToggled(bool),
    CatalogRefresh,
    CatalogLoaded(HookCatalogLoaded),
    AuditRefresh,
    AuditLoaded(HookAuditLoaded),
    TrustRequested { plugin_id: String, hook_hash: String },
    RevokeRequested { plugin_id: String, hook_hash: String },
    MutationCompleted(HookMutationCompleted),
}

/// 存储设置页消息。
#[derive(Debug, Clone)]
pub enum StorageMessage {
    ProviderChanged(String),
    DbUrlChanged(String),
    SchemaChanged(String),
    TableChanged(String),
    ConnectTimeoutSecsChanged(String),
    TlsToggled(bool),
    Save,
}

/// 查询分类设置页消息。
#[derive(Debug, Clone)]
pub enum QueryClassificationMessage {
    Refresh,
    EnabledToggled(bool),
    AddRule,
    RemoveRule(usize),
    PatternChanged(usize, String),
    CategoryChanged(usize, String),
    PriorityChanged(usize, String),
}

/// 模型配置内的子标签切换消息。
#[derive(Debug, Clone, Copy)]
pub enum ModelConfigTabMessage {
    Selected(crate::app::state::ModelConfigTab),
}

/// 智能体配置内的子标签切换消息。
#[derive(Debug, Clone, Copy)]
pub enum AgentConfigTabMessage {
    Selected(crate::app::state::AgentConfigTab),
}

/// 运行时设置页消息。
#[derive(Debug, Clone)]
pub enum RuntimeMessage {
    Refresh,
    KindChanged(String),
    DockerImageChanged(String),
    DockerNetworkChanged(String),
    DockerMemoryLimitMbChanged(String),
    DockerCpuLimitChanged(String),
    DockerReadOnlyRootfsToggled(bool),
    DockerMountWorkspaceToggled(bool),
    DockerAllowedWorkspaceRootsChanged(String),
    WasmToolsDirChanged(String),
    WasmFuelLimitChanged(String),
    WasmMemoryLimitMbChanged(String),
    WasmMaxModuleSizeMbChanged(String),
    WasmAllowWorkspaceReadToggled(bool),
    WasmAllowWorkspaceWriteToggled(bool),
    WasmAllowedHostsChanged(String),
    WasmRequireWorkspaceRelativeToolsDirToggled(bool),
    WasmRejectSymlinkModulesToggled(bool),
    WasmRejectSymlinkToolsDirToggled(bool),
    WasmStrictHostValidationToggled(bool),
    WasmCapabilityEscalationModeChanged(String),
    WasmModuleHashPolicyChanged(String),
    WasmModuleSha256Changed(String),
    ReasoningEnabledChanged(String),
    ReasoningLevelChanged(String),
}

/// 浏览器工具设置页消息。
#[derive(Debug, Clone)]
pub enum BrowserMessage {
    Refresh,
    EnabledToggled(bool),
    AllowedDomainsChanged(String),
    AllowedDomainsEditorAction(text_editor::Action),
    BrowserOpenChanged(String),
    SessionNameChanged(String),
    BackendChanged(String),
    NativeHeadlessToggled(bool),
    NativeWebdriverUrlChanged(String),
    NativeChromePathChanged(String),
    ComputerUseEndpointChanged(String),
    ComputerUseApiKeyChanged(String),
    ComputerUseTimeoutMsChanged(String),
    ComputerUseAllowRemoteEndpointToggled(bool),
    ComputerUseWindowAllowlistChanged(String),
    ComputerUseMaxCoordinateXChanged(String),
    ComputerUseMaxCoordinateYChanged(String),
}

/// 网页搜索工具设置页消息。
#[derive(Debug, Clone)]
pub enum WebSearchMessage {
    Refresh,
    EnabledToggled(bool),
    ProviderChanged(String),
    ApiKeyChanged(String),
    ApiUrlChanged(String),
    BraveApiKeyChanged(String),
    MaxResultsChanged(String),
    TimeoutSecsChanged(String),
    UserAgentChanged(String),
    HelpOpen,
    HelpClose,
}

/// 记忆系统设置页消息。
#[derive(Debug, Clone)]
pub enum MemoryMessage {
    Refresh,
    TabSelected(MemorySettingsTab),
    BackendChanged(String),
    AutoSaveToggled(bool),
    HygieneEnabledToggled(bool),
    ArchiveAfterDaysChanged(u32),
    PurgeAfterDaysChanged(u32),
    ConversationRetentionDaysChanged(u32),
    EmbeddingProviderChanged(String),
    EmbeddingModelChanged(String),
    EmbeddingDimensionsChanged(u32),
    VectorWeightChanged(f32),
    KeywordWeightChanged(f32),
    MinRelevanceScoreChanged(f32),
    EmbeddingCacheSizeChanged(u32),
    ChunkMaxTokensChanged(u32),
    ResponseCacheEnabledToggled(bool),
    ResponseCacheTtlMinutesChanged(u32),
    ResponseCacheMaxEntriesChanged(u32),
    SnapshotEnabledToggled(bool),
    SnapshotOnHygieneToggled(bool),
    AutoHydrateToggled(bool),
    SqliteOpenTimeoutSecsChanged(u32),
    QdrantUrlChanged(String),
    QdrantCollectionChanged(String),
    QdrantApiKeyChanged(String),
}

/// 多模态设置页消息。
#[derive(Debug, Clone)]
pub enum MultimodalMessage {
    MaxImagesChanged(u32),
    MaxImageSizeMbChanged(u32),
    AllowRemoteFetchToggled(bool),
    OcrEnabledToggled(bool),
    ModelSupportsImagesToggled(bool),
    OcrModelChanged(String),
}

/// HTTP 请求工具设置页消息。
#[derive(Debug, Clone)]
pub enum HttpRequestMessage {
    Refresh,
    EnabledToggled(bool),
    NewAllowedDomainChanged(String),
    AddAllowedDomain,
    RemoveAllowedDomain(usize),
    MaxResponseSizeChanged(u32),
    TimeoutSecsChanged(u32),
    UserAgentChanged(String),
}

/// 渠道设置页消息。
#[derive(Debug, Clone)]
pub enum ChannelsMessage {
    Refresh,
    TabSelected(ChannelsSettingsTab),
    CliToggled(bool),
    ProjectDirChanged(String),
    MessageTimeoutSecsChanged(u32),
    EnabledToggled(String, bool),
    InstanceAdded(String),
    InstanceRemoved(String, usize),
    TextChanged(String, String),
    TextEditorAction(String, text_editor::Action),
    BoolToggled(String, bool),
    NumberChanged(String, u32),
    ReceiveModeChanged(String, String),
}

/// 本地网关设置页消息。
#[derive(Debug, Clone)]
pub enum GatewayMessage {
    Refresh,
    TabSelected(GatewaySettingsTab),
    PortChanged(u16),
    HostChanged(String),
    AuthEnabledToggled(bool),
    NewSkeyNameChanged(String),
    NewSkeyChanged(String),
    NewSkeyExpiresAtChanged(String),
    NewSkeyCalendarToggled,
    NewSkeyCalendarClosed,
    NewSkeyExpiresDateSelected(String),
    NewSkeyExpiresMonthChanged(i32),
    NewSkeyExpiresAtCleared,
    AddSkey,
    CopyLastCreatedSkey,
    ClearLastCreatedSkeyCopied,
    SkeyEnabledToggled(usize, bool),
    RemoveSkey(usize),
    RequirePairingToggled(bool),
    AllowPublicBindToggled(bool),
    TrustForwardedHeadersToggled(bool),
    NewPairedTokenChanged(String),
    AddPairedToken,
    RemovePairedToken(usize),
    PairRateLimitPerMinuteChanged(u32),
    WebhookRateLimitPerMinuteChanged(u32),
    RateLimitMaxKeysChanged(u32),
    IdempotencyTtlSecsChanged(u32),
    IdempotencyMaxKeysChanged(u32),
    NodeControlEnabledToggled(bool),
    NodeControlAuthTokenChanged(String),
    NodeControlAllowedNodeIdsChanged(String),
    ServiceCommandRequested(String),
    ServiceCommandCompleted(String, Result<String, String>),
    ServiceRecoveryCompleted(String, Result<(), String>),
    CopyServiceCommand(String),
    ClearCopiedServiceCommand(String),
    ConfirmDangerServiceCommand(String),
    CancelDangerServiceCommand,
    HelpOpen,
    HelpClose,
}

/// 网关客户端设置页消息。
#[derive(Debug, Clone)]
pub enum GatewayClientMessage {
    SelectServer(String),
    AddServer,
    RemoveServerRequested(String),
    RemoveServerConfirmed(String),
    RemoveServerCanceled,
    NameChanged(String),
    HostChanged(String),
    PortChanged(u16),
    PlatformIdChanged(String),
    CompanyIdChanged(String),
    SkeyChanged(String),
    HelpOpen,
    HelpClose,
}

/// ACP 设置页消息。
#[derive(Debug, Clone)]
pub enum AcpMessage {
    Refresh,
    Loaded(Result<crate::app::config::AcpSettingsSnapshot, String>),
    NewAgentPanelToggled,
    NewAgentNameChanged(String),
    NewAgentCommandChanged(String),
    NewAgentArgsChanged(String),
    AddAgent,
    AddAgentCompleted {
        agent: String,
        result: Result<crate::app::config::AcpSettingsSnapshot, String>,
    },
    CopyCommand {
        agent: String,
        command: String,
    },
    ClearCopiedCommand(String),
    SetEnabled {
        agent: String,
        enabled: bool,
    },
    SetEnabledCompleted {
        agent: String,
        enabled: bool,
        result: Result<crate::app::config::AcpSettingsSnapshot, String>,
    },
}

/// MCP 设置页消息。
#[derive(Debug, Clone)]
pub enum McpMessage {
    TabSelected(McpSettingsTab),
    Refresh,
    Loaded(Result<serde_json::Value, String>),
    InspectRefresh,
    Inspected(Result<vw_gateway_client::vw_api_types::mcp::McpInspectResponse, String>),
    EditorAction(text_editor::Action),
    Format,
    InsertTemplate,
    NewServerPanelToggled,
    NewServerNameChanged(String),
    NewServerTransportChanged(McpNewServerTransport),
    NewServerCommandChanged(String),
    NewServerArgsChanged(String),
    NewServerUrlChanged(String),
    NewServerTimeoutChanged(String),
    NewServerEnabledToggled(bool),
    NewServerRequiredToggled(bool),
    AddServer,
    ServerEditRequested(usize),
    ServerEditCanceled,
    SaveServerEdit,
    ServerEnabledToggled(usize, bool),
    RuntimeServerEnabledToggled(String, bool),
    PreviewExpandedToggled(String),
    /// 请求删除指定索引的 MCP 服务器（弹出确认）。
    RequestDelete(usize),
    /// 确认删除指定的 MCP 服务器；处理时仍需匹配当前待确认索引。
    ConfirmDelete(usize),
    /// 取消删除操作。
    CancelDelete,
    Save,
    Saved(Result<(), String>),
}

/// 隧道设置页消息。
#[derive(Debug, Clone)]
pub enum TunnelMessage {
    ProviderChanged(String),
    CloudflareTokenChanged(String),
    TailscaleFunnelToggled(bool),
    TailscaleHostnameChanged(String),
    NgrokAuthTokenChanged(String),
    NgrokDomainChanged(String),
    CustomStartCommandChanged(String),
    CustomHealthUrlChanged(String),
    CustomUrlPatternChanged(String),
}

/// 主题编辑器调色板字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePaletteField {
    Background,
    Text,
    Primary,
    Success,
    Warning,
    Danger,
}

/// 主题编辑器控件皮肤字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChromeField {
    InputBg,
    InputBorder,
    InputText,
    PanelBg,
    PanelBorder,
    ButtonBg,
    ButtonText,
    RadiusPx,
    PanelAlpha,
    ChatHeaderAlpha,
    ChatBodyAlpha,
    ChatInputAlpha,
    /// 聊头背景图缩放策略。
    ChatHeaderImageFit,
    /// 消息区背景图缩放策略。
    ChatBodyImageFit,
    /// 输入区背景图缩放策略。
    ChatInputImageFit,
}

/// 主题编辑器当前打开的颜色选择器目标（同时只开一个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeColorPickerTarget {
    Palette(ThemePaletteField),
    Chrome(ThemeChromeField),
}

/// 主题设置页消息。
#[derive(Debug, Clone)]
pub enum ThemesMessage {
    Refresh,
    /// 画廊上一页（已在首页时由 UI 禁用）。
    GalleryPrevPage,
    /// 画廊下一页（已在末页时由 UI 禁用）。
    GalleryNextPage,
    ImportPack,
    ImportPackPicked(Option<String>),
    ExportPack(String),
    ExportPackPicked {
        theme_id: String,
        path: Option<String>,
    },
    ApplyBuiltin(String),
    ApplyCustom(String),
    DeleteCustom(String),
    NewThemeRequested,
    EditCustom(String),
    EditBuiltin(String),
    RestoreBuiltinDefault(String),
    RestorePresetDefault(String),
    CancelEdit,
    /// 切换主题编辑器分段 Tab。
    EditorTabSelected(ThemeEditorTab),
    DraftNameChanged(String),
    DraftTaglineChanged(String),
    DraftPaletteChanged(ThemePaletteField, String),
    PickCover,
    CoverPicked(Option<String>),
    ClearCover,
    PickBackground,
    BackgroundPicked(Option<String>),
    ClearBackground,
    PickChatHeader,
    ChatHeaderPicked(Option<String>),
    ClearChatHeader,
    PickChatBody,
    ChatBodyPicked(Option<String>),
    ClearChatBody,
    PickChatInput,
    ChatInputPicked(Option<String>),
    ClearChatInput,
    DraftChromeChanged(ThemeChromeField, String),
    /// 打开或切换到指定色字段的颜色对话框；同一目标再次触发则关闭。
    ColorPickerToggled(ThemeColorPickerTarget),
    ColorPickerClosed,
    /// 选择器提交的颜色字符串（由 UI 规范化为 `#rrggbb`）。
    ColorPickerColorChanged(String),
    ColorPickerFormatChanged(crate::app::components::color_picker::ColorFormat),
    SaveDraft,
}

/// Agent 设置页消息。
#[derive(Debug, Clone)]
#[allow(private_interfaces)]
pub enum AgentsMessage {
    Refresh,
    Loaded(AgentsLoaded),
    ConfigLoaded {
        project_path: Option<String>,
        result: Result<Box<vw_config_types::config::Config>, String>,
    },
    ProjectSelected(String),
    ProjectChanged(String),
    ProjectConfigLoaded {
        project_path: String,
        result: Result<Box<vw_config_types::config::Config>, String>,
    },
    SelectAgent(String),
    AddAgentKeyChanged(String),
    AddAgentRequested,
    DetailTabSelected(String),
    ProviderChanged(String, String),
    ProviderCleared(String),
    ModelChanged(String, String),
    ModelCleared(String),
    ForceModelSelectionToggled(String, bool),
    EnabledToggled(String, bool),
    AllowAsSubagentToggled(String, bool),
    MaxParallelSubagentsChanged(String, u32),
    SystemPromptAction(String, text_editor::Action),
    PromptTabSelected(String),
    WorkspaceIdentityLoaded {
        agent_key: String,
        file_name: String,
        workspace_root_path: Option<String>,
        result: Result<(String, Option<u64>, Option<u64>), String>,
    },
    WorkspaceIdentityRestoreDefaultRequested(String),
    WorkspaceIdentityEnabledToggled {
        agent_key: String,
        file_name: String,
        enabled: bool,
    },
    WorkspaceIdentityDefaultRestored {
        agent_key: String,
        file_name: String,
        workspace_root_path: Option<String>,
        result: Result<(String, Option<u64>, Option<u64>), String>,
    },
    WorkspaceIdentityEditorAction(String, text_editor::Action),
    WorkspaceIdentitySaved {
        file_name: String,
        size_bytes: Option<u64>,
        modified_at_ms: Option<u64>,
        result: Result<(), String>,
    },
    ApiKeyChanged(String, String),
    TemperatureChanged(String, f32),
    CompactContextToggled(String, bool),
    MaxToolIterationsChanged(String, u32),
    MaxHistoryMessagesChanged(String, u32),
    ParallelToolsToggled(String, bool),
    ToolDispatcherChanged(String, String),
    MaxDepthChanged(String, u32),
    AgenticToggled(String, bool),
    ToolPolicyChanged(String, vw_config_types::agent::DelegateCapabilityPolicy),
    AllowedToolToggled(String, String, bool),
    AllowedToolsSelectAll(String),
    AllowedToolsInvertSelection(String),
    AllowedToolsApplyPreset(String, String),
    ToolsListTabSelected(String),
    SkillPolicyChanged(String, vw_config_types::agent::DelegateCapabilityPolicy),
    AllowedSkillToggled(String, String, bool),
    AllowedSkillsSelectAll(String, crate::app::state::SkillsDirectoryScope),
    AllowedSkillsInvertSelection(String, crate::app::state::SkillsDirectoryScope),
    MaxIterationsChanged(String, u32),
}

/// 员工配置设置页消息。
#[derive(Debug, Clone)]
pub enum StaffMessage {
    Refresh,
    Loaded(StaffLoaded),
    Resources(StaffResourceMessage),
    TabChanged(String),
    SelectMember(String),
    AddMemberRequested,
    DeleteMemberRequested(String),
    ResetDraftRequested,
    MemberIdChanged(String, String),
    MemberNameChanged(String, String),
    MemberRoleChanged(String, String),
    MemberLevelChanged(String, String),
    MemberStatusChanged(String, String),
    MemberEnabledToggled(String, bool),
    MemberCapacityChanged(String, String),
    MemberDelegateAgentChanged(String, Option<String>),
    MemberAcpAgentChanged(String, Option<String>),
    ChannelBindingAdded(String),
    ChannelBindingRemoved(String, usize),
    ChannelBindingChannelTypeChanged(String, usize, String),
    ChannelBindingInstanceIdChanged(String, usize, String),
    CapabilityAgenticToggled(String, bool),
    CapabilityAllowedToolToggled(String, String, bool),
    CapabilityAllowedSkillToggled(String, String, bool),
    CapabilityMaxIterationsChanged(String, String),
    CapabilityMaxParallelTasksChanged(String, String),
    CapabilityMaxActiveMinutesChanged(String, String),
    SaveRequested,
    Saved(Result<(), String>),
}

/// 员工设置中的项目级 Worker 与 Team 资源消息。
#[derive(Debug, Clone)]
pub enum StaffResourceMessage {
    Refresh,
    WorkersLoaded(
        Result<Vec<vw_gateway_client::vw_api_types::agent_scheduler::AgentWorkerDto>, String>,
    ),
    WorkerSelected(String),
    WorkerNewRequested,
    WorkerFieldChanged(StaffWorkerDraftField, String),
    WorkerCapabilitiesChanged(
        Vec<vw_gateway_client::vw_api_types::agent_scheduler::AgentCapabilityDto>,
    ),
    WorkerSaveRequested,
    WorkerSaved(Result<vw_gateway_client::vw_api_types::agent_scheduler::AgentWorkerDto, String>),
    TeamsLoaded(
        Result<Vec<vw_gateway_client::vw_api_types::agent_scheduler::AgentTeamDto>, String>,
    ),
    TeamSelected(String),
    TeamNewRequested,
    TeamFieldChanged(StaffTeamDraftField, String),
    TeamStatusChanged(vw_gateway_client::vw_api_types::agent_scheduler::AgentTeamStatusDto),
    TeamMemberAdded(String),
    TeamMemberRemoved(usize),
    TeamMemberFieldChanged(usize, StaffTeamMemberDraftField, String),
    TeamMemberReadyToggled(usize, bool),
    TeamSaveRequested,
    TeamSaved(Result<vw_gateway_client::vw_api_types::agent_scheduler::AgentTeamDto, String>),
}

#[derive(Debug, Clone, Copy)]
pub enum StaffWorkerDraftField {
    WorkerId,
    Runtime,
    Deployment,
    Model,
    ModelProvider,
    TeamId,
    WorkerRole,
    MatrixUserId,
    RoomId,
}

#[derive(Debug, Clone, Copy)]
pub enum StaffTeamDraftField {
    TeamId,
    Name,
    LeaderId,
    TeamRoomId,
    LeaderDmRoomId,
}

#[derive(Debug, Clone, Copy)]
pub enum StaffTeamMemberDraftField {
    WorkerId,
    RuntimeName,
    Role,
    RoomId,
    MatrixUserId,
}

/// 聊天快捷消息设置消息。
#[derive(Debug, Clone)]
pub enum QuickMessagesMessage {
    Add,
    Remove(usize),
    TextChanged(usize, String),
    GroupChanged(usize, QuickMessageGroup),
    KindChanged(usize, QuickMessageKind),
    MoveUp(usize),
    MoveDown(usize),
    Save,
    Saved(Result<(), String>),
}

/// 系统设置页顶层消息。
#[derive(Debug, Clone)]
pub enum GeneralSettingsMessage {
    TabSelected(crate::app::state::GeneralSettingsTab),
    TimezoneSelected(String),
}

/// 系统设置页顶层消息。
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    TabSelected(SettingsTab),
    SystemTabSelected(SystemTab),
    SystemSearchTargetSelected(SystemSettingsSearchTarget),
    ModelConfigTab(ModelConfigTabMessage),
    AgentConfigTab(AgentConfigTabMessage),
    SystemTabQueryChanged(String),
    SystemHelpOpen(SystemTab),
    SystemHelpClose,
    ToggleSettingsSidebar,
    General(GeneralSettingsMessage),
    EmbeddingRoutes(EmbeddingRoutesMessage),
    Hooks(HooksMessage),
    Storage(StorageMessage),
    ModelRoutes(ModelRoutesMessage),
    QueryClassification(QueryClassificationMessage),
    Runtime(RuntimeMessage),
    Browser(BrowserMessage),
    WebSearch(WebSearchMessage),
    HttpRequest(HttpRequestMessage),
    Memory(MemoryMessage),
    Channels(ChannelsMessage),
    Multimodal(MultimodalMessage),
    Acp(AcpMessage),
    Mcp(McpMessage),
    Gateway(GatewayMessage),
    GatewayClient(GatewayClientMessage),
    Tunnel(TunnelMessage),
    Themes(ThemesMessage),
    Agents(AgentsMessage),
    Staff(StaffMessage),
    QuickMessages(QuickMessagesMessage),
    AgentConfigSaved {
        tag: &'static str,
        result: Result<(), String>,
    },
    ProvidersRefresh,
    ProvidersRefreshed(ProvidersLoaded),
    ProviderModelsSyncRemote,
    ProviderModelsSyncTick,
    ProviderModelsSyncDone(ProvidersLoaded),
    ModelsRefresh,
    ModelsRefreshed(ModelsLoaded),
    ModelsSetAll(bool),
    ProviderModelsSetAll(String, bool),
    ProviderModelsReset(String),
    ProviderModelsResetCompleted(Vec<ModelStatusChange>, ModelsLoaded),
    ProviderModelsExpansionToggled(String),
    ModelStatusesSaved(Vec<ModelStatusChange>, Result<(), String>),
    SkillsProjectSelected(String),
    SkillsRefresh,
    SkillsLoaded(SkillsLoaded),
    PluginsRefresh,
    PluginsLoaded(PluginsLoaded),
    PluginsSelected(String),
    PluginsSetEnabledRequested {
        plugin_id: String,
        enabled: bool,
    },
    PluginsSetEnabledCompleted {
        plugin_id: String,
        enabled: bool,
        result: Result<String, String>,
    },
    PluginsDeleteRequested(String),
    PluginsDeleteCompleted {
        plugin_id: String,
        result: Result<String, String>,
    },
    ModelQueryChanged(String),
    ModelToggle(String, String, bool),
    ModelDetailOpen(String, String),
    ModelDetailClose,
    ModelDetailToggleRaw,
    ProviderConnectOpen(String),
    ProviderConnectClose,
    ProviderConnectApiKeyChanged(String),
    ProviderConnectSubmit,
    ProviderConnectDone(ProvidersLoaded),
    ProviderDisconnectRequested(String),
    ProviderDisconnectCanceled,
    ProviderDisconnectConfirmed(String),
    ProviderDisconnect(String),
    ProviderDisconnectDone(ProvidersLoaded),
    CustomProviderOpen,
    CustomProviderEditOpen(String),
    CustomProviderEditLoaded(Result<crate::app::state::CustomProviderDraft, String>),
    CustomProviderClose,
    CustomProviderTabSelected(CustomProviderTab),
    CustomProviderIdChanged(String),
    CustomProviderNameChanged(String),
    CustomProviderBaseUrlChanged(String),
    CustomProviderApiKeyChanged(String),
    CustomProviderHeaderAdd,
    CustomProviderHeaderRemove(usize),
    CustomProviderHeaderKeyChanged(usize, String),
    CustomProviderHeaderValueChanged(usize, String),
    CustomProviderModelOpen(Option<usize>),
    CustomProviderModelClose,
    CustomProviderModelTabSelected(CustomProviderModelTab),
    CustomProviderModelModalIdChanged(String),
    CustomProviderModelModalNameChanged(String),
    CustomProviderModelModalFamilyChanged(String),
    CustomProviderModelModalContextLimitChanged(String),
    CustomProviderModelModalInputLimitChanged(String),
    CustomProviderModelModalOutputLimitChanged(String),
    CustomProviderModelModalReasoningChanged(bool),
    CustomProviderModelModalToolCallChanged(bool),
    CustomProviderModelModalTemperatureChanged(bool),
    CustomProviderModelModalAttachmentChanged(bool),
    CustomProviderModelModalInterleavedChanged(bool),
    CustomProviderModelModalInputModalityToggled(String, bool),
    CustomProviderModelModalOutputModalityToggled(String, bool),
    CustomProviderModelModalSave,
    CustomProviderModelRemove(usize),
    CustomProviderModelEnabledToggled(usize, bool),
    CustomProviderModelFilterSelected(CustomProviderModelFilter),
    CustomProviderModelsDiscover,
    CustomProviderModelsDiscovered {
        provider_id: String,
        result: Result<Vec<String>, String>,
    },
    CustomProviderSave,
    CustomProviderSaveDone(ProvidersLoaded),
    PopularProviderRemove(usize),
    PopularProvidersSaved(Result<(), String>),
    ProviderCatalogOpen,
    ProviderCatalogClose,
    ProviderCatalogQueryChanged(String),
    ProviderCatalogLoaded(Result<Vec<ModelCatalogEntry>, String>),
    ProviderCatalogAddToPopular(String),
    RecentProjectRenameChanged(usize, String),
    RecentProjectRenameSave(usize),
    RecentProjectDeleteRequested(usize),
    RecentProjectDeleteCanceled,
    RecentProjectDeleteConfirmed(usize),
    ProjectEnableWorktreeToggled(String, bool),
    SessionDelete(String),
    SessionCopy(String),
    DialogueFlowPermissionRefresh,
    DialogueFlowPermissionLoaded(DialogueFlowPermissionLoaded),
    DialogueFlowPermissionReset,
    DialogueFlowUiSettingsLoaded(DialogueFlowUiSettingsLoaded),
    DialogueFlowUiSettingsSave,
    DialogueFlowUiSettingsSaved(Result<(), String>),
    DialogueFlowShowReasoningSummaryToggled(bool),
    DialogueFlowExpandShellToolSectionToggled(bool),
    DialogueFlowExpandEditToolSectionToggled(bool),
    HeartbeatEnabledToggled(bool),
    HeartbeatIntervalChanged(u32),
    HeartbeatMessageChanged(String),
    HeartbeatTargetChanged(String),
    HeartbeatToChanged(String),
    HeartbeatSave,
    HeartbeatHelpOpen,
    HeartbeatHelpClose,
    CronEnabledToggled(bool),
    CronMaxRunHistoryChanged(u32),
    CronTabSelected(CronSettingsTab),
    CronJobsRefresh,
    CronJobsLoaded(Result<Vec<vw_gateway_client::CronJobDto>, String>),
    CronJobSelectionToggled(String, bool),
    CronJobDetailsToggled(String, bool),
    CronJobsSelectAllToggled(bool),
    CronJobRunsOpen(String),
    CronJobRunsLoaded(String, Result<Vec<vw_gateway_client::CronRunDto>, String>),
    CronJobRunsEditorAction(text_editor::Action),
    CronJobRunsClose,
    CronJobEditStarted(String),
    CronJobEditCanceled,
    CronJobEditNameChanged(String),
    CronJobEditJobTypeChanged(CronAddJobType),
    CronJobEditScheduleKindChanged(CronAddScheduleKind),
    CronJobEditScheduleChanged(String),
    CronJobEditAtChanged(String),
    CronJobEditEveryMsChanged(String),
    CronJobEditCommandChanged(String),
    CronJobEditCommandEditorAction(text_editor::Action),
    CronJobEditPromptChanged(String),
    CronJobEditPromptEditorAction(text_editor::Action),
    CronJobEditAgentChanged(String),
    CronJobEditAcpAgentChanged(String),
    CronJobEditProjectPathChanged(String),
    CronJobEditModelProviderChanged(String),
    CronJobEditModelChanged(String),
    CronJobEditWakeToggled(bool),
    CronJobEditFallbacksChanged(String),
    CronJobEditFullAccessToggled(bool),
    CronJobEditTaskPoolToggled(bool),
    CronJobEditDeliveryEnabledToggled(bool),
    CronJobEditDeliveryChannelChanged(String),
    CronJobEditDeliveryToChanged(String),
    CronJobEditDeliveryBestEffortToggled(bool),
    CronJobEditDeleteAfterRunToggled(bool),
    CronJobEditSave,
    CronJobEnabledChanged(String, bool),
    CronJobDelete(String),
    CronSelectedJobsEnable,
    CronSelectedJobsDisable,
    CronSelectedJobsDelete,
    CronAddNameChanged(String),
    CronAddJobTypeChanged(CronAddJobType),
    CronAddScheduleKindChanged(CronAddScheduleKind),
    CronAddScheduleChanged(String),
    CronAddAtChanged(String),
    CronAddEveryMsChanged(String),
    CronAddCommandChanged(String),
    CronAddCommandEditorAction(text_editor::Action),
    CronAddPromptChanged(String),
    CronAddPromptEditorAction(text_editor::Action),
    CronAddSessionTargetChanged(String),
    CronAddAgentChanged(String),
    CronAddAcpAgentChanged(String),
    CronAddProjectPathChanged(String),
    CronAddModelProviderChanged(String),
    CronAddModelChanged(String),
    CronAddWakeToggled(bool),
    CronAddFallbacksChanged(String),
    CronAddFullAccessToggled(bool),
    CronAddTaskPoolToggled(bool),
    CronAddDeliveryEnabledToggled(bool),
    CronAddDeliveryChannelChanged(String),
    CronAddDeliveryToChanged(String),
    CronAddDeliveryBestEffortToggled(bool),
    CronAddDeleteAfterRunToggled(bool),
    CronAddSubmit,
    CronJobMutationCompleted(Result<String, String>),
    CronSave,
    CronHelpOpen,
    CronHelpClose,
    SchedulerEnabledToggled(bool),
    SchedulerMaxTasksChanged(u32),
    SchedulerMaxConcurrentChanged(u32),
    SchedulerSave,
    SchedulerHelpOpen,
    SchedulerHelpClose,
    ReliabilityProviderRetriesChanged(u32),
    ReliabilityProviderBackoffMsChanged(u64),
    ReliabilityChannelInitialBackoffSecsChanged(u64),
    ReliabilityChannelMaxBackoffSecsChanged(u64),
    ReliabilitySchedulerPollSecsChanged(u64),
    ReliabilitySchedulerRetriesChanged(u32),
    ReliabilitySave,
    ReliabilityHelpOpen,
    ReliabilityHelpClose,
    SecurityBashSecurityEnabledToggled(bool),
    SecurityBashPolicyCheckToggled(BashPolicyCheck, bool),
    SecurityBashValidatorCheckToggled(BashValidatorCheck, bool),
    SecurityTabSelected(SecuritySettingsTab),
    SecurityListDraftChanged(SecurityListKind, String),
    SecurityListAdd(SecurityListKind),
    SecurityListRemoveRequested(SecurityListKind, usize),
    SecurityListRemoveConfirmed(SecurityListKind, usize),
    SecurityListRemoveCanceled,
    SecuritySandboxEnabledChanged(String),
    SecuritySandboxBackendChanged(String),
    SecurityResourcesMaxMemoryMbChanged(u32),
    SecurityResourcesMaxCpuTimeSecondsChanged(u64),
    SecurityResourcesMaxSubprocessesChanged(u32),
    SecurityResourcesMemoryMonitoringToggled(bool),
    SecurityAuditEnabledToggled(bool),
    SecurityAuditLogPathChanged(String),
    SecurityAuditMaxSizeMbChanged(u32),
    SecurityAuditSignEventsToggled(bool),
    SecurityOtpEnabledToggled(bool),
    SecurityOtpMethodChanged(String),
    SecurityOtpTokenTtlSecsChanged(u64),
    SecurityOtpCacheValidSecsChanged(u64),
    SecurityEstopEnabledToggled(bool),
    SecurityEstopStateFileChanged(String),
    SecurityEstopRequireOtpToResumeToggled(bool),
    SecuritySyscallAnomalyEnabledToggled(bool),
    SecuritySyscallAnomalyStrictModeToggled(bool),
    SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(bool),
    SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(u32),
    SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(u32),
    SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(u32),
    SecuritySyscallAnomalyAlertCooldownSecsChanged(u64),
    SecuritySyscallAnomalyLogPathChanged(String),
    SecurityCanaryTokensToggled(bool),
    SecuritySemanticGuardToggled(bool),
    SecuritySemanticGuardCollectionChanged(String),
    SecuritySemanticGuardThresholdChanged(f64),
    SecuritySave,
    SecurityHelpOpen,
    SecurityHelpClose,
    AutonomyTabSelected(AutonomySettingsTab),
    AutonomyLevelChanged(AutonomyLevel),
    AutonomyWorkspaceOnlyToggled(bool),
    AutonomyListDraftChanged(AutonomyListKind, String),
    AutonomyListAdd(AutonomyListKind),
    AutonomyListRemoveRequested(AutonomyListKind, usize),
    AutonomyListRemoveConfirmed(AutonomyListKind, usize),
    AutonomyListRemoveCanceled,
    AutonomyMaxActionsPerHourChanged(u32),
    AutonomyMaxCostPerDayCentsChanged(u32),
    AutonomyRequireApprovalForMediumRiskToggled(bool),
    AutonomyBlockHighRiskCommandsToggled(bool),
    AutonomyShellRedirectPolicyChanged(ShellRedirectPolicy),
    AutonomyNonCliApprovalApproversChanged(String),
    AutonomyNonCliNaturalLanguageApprovalModeChanged(
        vw_config_types::security::NonCliNaturalLanguageApprovalMode,
    ),
    AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(String),
    AutonomySave,
    AutonomyHelpOpen,
    AutonomyHelpClose,
    ObservabilityBackendChanged(String),
    ObservabilityOtelEndpointChanged(String),
    ObservabilityOtelServiceNameChanged(String),
    ObservabilityRuntimeTraceModeChanged(String),
    ObservabilityRuntimeTracePathChanged(String),
    ObservabilityRuntimeTraceMaxEntriesChanged(u32),
    ObservabilityHelpOpen,
    ObservabilityHelpClose,
    ProxyEnabledToggled(bool),
    ProxyScopeTextChanged(String),
    ProxyHttpChanged(String),
    ProxyHttpsChanged(String),
    ProxyAllChanged(String),
    ProxyNoProxyChanged(String),
    ProxyServicesChanged(String),
    ProxyHelpOpen,
    ProxyHelpClose,
    SkillsTabChanged(crate::app::state::SkillsSettingsTab),
    SkillsDetailClosed,
    SkillsDetailRequested(String),
    SkillsDetailLoaded {
        skill_id: String,
        result: Result<vw_gateway_client::DesktopSkillDetailDto, String>,
    },
    SkillsQueryChanged(String),
    SkillsDirectoryScopeChanged(crate::app::state::SkillsDirectoryScope),
    SkillsCreateNewRequested,
    SkillsCreateNewCompleted(Result<String, String>),
    SkillsCopyInstallCommand,
    SkillsInstallBuiltInRequested(String),
    SkillsInstallBuiltInCompleted(Result<String, String>),
    SkillsSetEnabledRequested {
        skill_id: String,
        enabled: bool,
    },
    SkillsSetEnabledCompleted {
        skill_id: String,
        enabled: bool,
        result: Result<String, String>,
    },
    SkillsDeleteRequested(String),
    SkillsDeleteCompleted {
        skill_id: String,
        result: Result<String, String>,
    },
    SkillsDirectoryProviderChanged(vw_config_types::skills::SkillsDirectoryProvider),
    SkillsProjectAutoLoadToggled(bool),
    SkillsOpenEnabledToggled(bool),
    SkillsOpenDirChanged(String),
    SkillsPromptInjectionModeChanged(vw_config_types::skills::SkillsPromptInjectionMode),
    SkillsSave,
    SkillsHelpOpen,
    SkillsHelpClose,
}
