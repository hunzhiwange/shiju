//! Aggregate settings events; the host supplies only its navigation and cross-domain envelopes.
use crate::SystemTab;
use config_security::{AutonomyLevel, ShellRedirectPolicy};
use desktop_settings_agents::dialogue_flow_message::{
    DialogueFlowPermissionLoaded, DialogueFlowUiSettingsLoaded,
};
use desktop_settings_cron::state::{
    CronAddJobType, CronAddScheduleKind, CronFrequency, CronListFilter, CronSettingsTab,
};
/// Embedding 路由设置页消息。
pub use desktop_settings_providers::embedding_view_message::EmbeddingRoutesMessage;
use desktop_settings_providers::provider_state::{
    CustomProviderModelFilter, CustomProviderTab, ModelCatalogEntry,
};
use desktop_settings_providers::state::CustomProviderModelTab;
use desktop_settings_providers::{
    models::{ModelStatusChange, ModelsLoaded},
    provider_message::ProvidersLoaded,
};
use desktop_settings_security::app_init::state::{AutonomyListKind, AutonomySettingsTab};
use desktop_settings_security::security_state::{SecurityListKind, SecuritySettingsTab};
use desktop_settings_skills::types::{PluginsLoaded, SkillsLoaded};
use iced::widget::text_editor;

pub use desktop_settings_security::security_message::{BashPolicyCheck, BashValidatorCheck};

/// 模型路由设置页消息。
pub use desktop_settings_providers::settings_message::ModelRoutesMessage;

/// 存储设置页消息。
pub use desktop_settings_memory::messages::StorageMessage;

/// 模型配置内的子标签切换消息。
pub use desktop_settings_providers::settings_message::ModelConfigTabMessage;

/// 智能体配置内的子标签切换消息。
pub use desktop_settings_agents::agents_update_message::AgentConfigTabMessage;

/// 浏览器工具设置页消息。
pub use desktop_settings_network::messages::BrowserMessage;

/// 网页搜索工具设置页消息。
pub use desktop_settings_network::messages::WebSearchMessage;

/// 记忆系统设置页消息。
pub use desktop_settings_memory::messages::MemoryMessage;

/// HTTP 请求工具设置页消息。
pub use desktop_settings_network::messages::HttpRequestMessage;

/// 渠道设置页消息。
pub use desktop_settings_channels::messages::ChannelsMessage;

/// 本地网关设置页消息。
pub use desktop_settings_network::messages::GatewayMessage;

/// 网关客户端设置页消息。
pub use desktop_settings_network::messages::GatewayClientMessage;

/// ACP 设置页消息。
pub use desktop_settings_agents::acp_update_message::AcpMessage;

/// MCP 设置页消息。
pub use desktop_settings_mcp::settings_message::McpMessage;

/// 隧道设置页消息。
pub use desktop_settings_network::messages::TunnelMessage;

pub use desktop_settings_appearance::system_settings_themes::messages::{
    ThemeChromeField, ThemeColorPickerTarget, ThemePaletteField, ThemesMessage,
};

/// Agent 设置页消息。
pub use desktop_settings_agents::message::AgentsMessage;

pub use desktop_settings_appearance::quick_messages_update::QuickMessagesMessage;

/// 系统设置页顶层消息。
pub use desktop_settings_appearance::general_message::GeneralSettingsMessage;

/// 系统设置页顶层消息。
#[derive(Debug, Clone)]
pub enum SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders> {
    MigrationPreview(desktop_settings_memory::system_settings_data::migration_preview_ui::Event),
    DataBackups(desktop_settings_memory::data_management::backup_list_ui::Event),
    DataRecovery(desktop_settings_memory::data_management::recovery_ui::Event),
    DataExport(desktop_settings_memory::data_management::export_ui::Event),
    DeviceSyncWebdav(desktop_settings_memory::device_sync_webdav::Event),
    DeviceSyncS3(desktop_settings_memory::device_sync_s3::Event),
    BackupInventory(desktop_settings_memory::backup_inventory::Event),
    #[cfg(not(target_arch = "wasm32"))]
    ComputerControl(desktop_settings_security::computer_control_update::Event),
    #[cfg(not(target_arch = "wasm32"))]
    Voice(desktop_settings_voice::Event),
    ArchivedChats(desktop_settings_sessions::archived_chats::ArchivedChatsMessage),
    Worktrees(desktop_settings_workspace::messages::WorktreeSettingsMessage),
    AppsCatalog(AppsCatalog),
    #[cfg(not(target_arch = "wasm32"))]
    AgentAppsAuth(desktop_provider_apps::auth::Event),
    AppProviders(AppProviders),
    AppProviderEffect(desktop_provider_apps::message::provider_update::Effect),
    AppProviderEdit(desktop_provider_apps::message::edit::EditMessage),
    TabSelected(SettingsTab),
    SystemTabSelected(SystemTab),
    SystemSearchTargetSelected(SearchTarget),
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
    ModelStatusesSaved(
        Vec<ModelStatusChange>,
        Result<(), desktop_i18n::settings_models::messages::Feedback>,
    ),
    SkillsProjectSelected(String),
    SkillsRefresh,
    SkillsLoaded(SkillsLoaded),
    PluginInstallEffect(desktop_settings_skills::plugin_install::Event),
    PluginsInstallOpen,
    PluginsUpdateOpen(String),
    PluginsInstallSourceChanged(String),
    PluginsInstallPrepare,
    PluginsInstallPrepared {
        request_id: u64,
        result: Result<gateway_desktop_settings::DesktopPluginPreviewDto, String>,
    },
    PluginsInstallConfirm,
    PluginsInstallCompleted {
        request_id: u64,
        result: Result<String, String>,
    },
    PluginsInstallClose,
    PluginsPreviewDiscarded(Result<(), String>),
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
    CustomProviderEditLoaded(
        Result<
            desktop_settings_providers::provider_state::CustomProviderDraft,
            desktop_i18n::settings_models::messages::Feedback,
        >,
    ),
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
        result: Result<Vec<String>, desktop_i18n::settings_models::messages::Feedback>,
    },
    CustomProviderSave,
    CustomProviderSaveDone(ProvidersLoaded),
    PopularProviderRemove(usize),
    PopularProvidersSaved(Result<(), desktop_i18n::settings_models::messages::Feedback>),
    ProviderCatalogOpen,
    ProviderCatalogClose,
    ProviderCatalogQueryChanged(String),
    ProviderCatalogLoaded(
        Result<Vec<ModelCatalogEntry>, desktop_i18n::settings_models::messages::Feedback>,
    ),
    ProviderCatalogAddToPopular(String),
    RecentProjectQueryChanged(String),
    RecentProjectDeleteRequested(usize),
    RecentProjectDeleteCanceled,
    RecentProjectDeleteConfirmed(usize),
    ProjectEnableWorktreeToggled(String, bool),
    ProjectWorktreeSaved(Result<(), String>),
    SessionDelete(String),
    SessionCopy(String),
    DialogueFlowPermissionRefresh,
    DialogueFlowPermissionLoaded(DialogueFlowPermissionLoaded),
    DialogueFlowPermissionReset,
    DialogueFlowUiSettingsLoaded(
        desktop_config::config_desktop::PreferencesScope,
        u64,
        DialogueFlowUiSettingsLoaded,
    ),
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
    CronPickerOpen {
        editing: bool,
    },
    CronPickerNavigate(i32),
    CronPickerDate(chrono::NaiveDate),
    CronPickerTime {
        hour: u32,
        minute: u32,
    },
    CronFullAccessConfirm {
        editing: bool,
    },
    CronFullAccessCancel {
        editing: bool,
    },
    CronPickerConfirm,
    CronPickerCancel,
    CronScheduleDraftChanged {
        editing: bool,
        draft: desktop_settings_cron::state::CronScheduleDraft,
    },
    CronSchedulePreview {
        editing: bool,
    },
    CronSchedulePreviewLoaded {
        editing: bool,
        revision: u64,
        result: Result<gateway_cron::CronPreviewResponse, String>,
    },
    CronEnabledToggled(bool),
    CronMaxRunHistoryChanged(u32),
    CronTabSelected(CronSettingsTab),
    CronTemplatesToggled(bool),
    CronTemplateSelected(desktop_settings_cron::state::CronTemplateId),
    CronHistory(desktop_settings_cron::message::HistoryMessage),
    CronListFilterChanged(CronListFilter),
    CronBatchModeToggled(bool),
    CronJobsRefresh,
    CronJobsLoaded(Result<Vec<gateway_cron::CronJobDto>, String>),
    CronJobSelectionToggled(String, bool),
    CronJobDetailsToggled(String, bool),
    CronJobsSelectAllToggled(bool),
    CronJobRunsOpen(String),
    CronJobRunsLoaded(String, Result<Vec<gateway_cron::CronRunDto>, String>),
    CronJobRunsEditorAction(text_editor::Action),
    CronJobRunsClose,
    CronJobEditStarted(String),
    CronJobEditCanceled,
    CronJobEditNameChanged(String),
    CronJobEditJobTypeChanged(CronAddJobType),
    CronJobEditScheduleKindChanged(CronAddScheduleKind),
    CronJobEditScheduleChanged(String),
    CronJobEditFrequencyChanged(CronFrequency),
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
    CronAddFrequencyChanged(CronFrequency),
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
    CronEditorSaveCompleted {
        editing: bool,
        result: Result<String, String>,
    },
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
        config_security::NonCliNaturalLanguageApprovalMode,
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
    SkillsTabChanged(desktop_settings_skills::types::SkillsSettingsTab),
    SkillsDetailClosed,
    SkillsDetailRequested(String),
    SkillsDetailLoaded {
        skill_id: String,
        result: Result<gateway_desktop_settings::DesktopSkillDetailDto, String>,
    },
    SkillsQueryChanged(String),
    SkillsDirectoryScopeChanged(desktop_settings_skills::types::SkillsDirectoryScope),
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
    SkillsDirectoryProviderChanged(config_skills::SkillsDirectoryProvider),
    SkillsProjectAutoLoadToggled(bool),
    SkillsPromptInjectionModeChanged(config_skills::SkillsPromptInjectionMode),
    SkillsSave,
    SkillsUpdates(desktop_settings_skills::updates_view::Event),
    SkillsInstall(desktop_settings_skills::install_view::Event),
    SkillsJobs(desktop_settings_skills::jobs_view::Event),
    SkillsMarket(desktop_settings_skills::market_view::Event),
    SkillRepositories(desktop_settings_skills::repo_modal::Event),
    SkillsImport(desktop_settings_skills::import_modal::Event),
    SkillsBackups(desktop_settings_skills::update::backup_modal::Event),
    SkillsStorage(desktop_settings_skills::storage_modal::Event),
    SkillsBindings(desktop_settings_skills::update::bindings_view::Event),
    SkillsClaudeConfig(desktop_settings_skills::update::claude_view::Event),
    SkillsHelpOpen,
    SkillsHelpClose,
}

pub use desktop_settings_runtime::messages::{
    HooksMessage, MultimodalMessage, QueryClassificationMessage, RuntimeMessage,
};

mod routes;
pub use routes::SettingsRoute;
