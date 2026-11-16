use super::*;
/// Exhaustive domain destination selected before host effect assembly.
pub enum SettingsRoute<SettingsTab, SearchTarget, AppsCatalog, AppProviders> {
    DeviceSyncWebdav(desktop_settings_memory::device_sync_webdav::Event),
    DeviceSyncS3(desktop_settings_memory::device_sync_s3::Event),
    MigrationPreview(desktop_settings_memory::system_settings_data::migration_preview_ui::Event),
    DataBackups(desktop_settings_memory::data_management::backup_list_ui::Event),
    DataRecovery(desktop_settings_memory::data_management::recovery_ui::Event),
    DataExport(desktop_settings_memory::data_management::export_ui::Event),
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
    AppProviderEffect(desktop_provider_apps::message::provider_update::Effect),
    AppProviderEdit(desktop_provider_apps::message::edit::EditMessage),
    AppProviders(AppProviders),
    TabSelected(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    General(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    EmbeddingRoutes(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Hooks(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Storage(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    ModelRoutes(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    QueryClassification(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Runtime(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Browser(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    WebSearch(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    HttpRequest(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Memory(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Channels(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Multimodal(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Acp(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Mcp(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    GatewayClient(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Gateway(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Tunnel(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Themes(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    Agents(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    QuickMessages(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    AgentConfigSaved {
        tag: &'static str,
        result: Result<(), String>,
    },
    ProvidersRefresh(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    ModelsRefresh(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    PluginInstallEffect(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    SkillsDirectoryProviderChanged(
        SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>,
    ),
    SkillsProjectSelected(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    DialogueFlowPermissionRefresh(
        SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>,
    ),
    RecentProjectQueryChanged(
        SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>,
    ),
    SessionDelete(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    HeartbeatEnabledToggled(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    CronHistory(desktop_settings_cron::message::HistoryMessage),
    CronFullAccessConfirm(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    SchedulerEnabledToggled(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    ReliabilityProviderRetriesChanged(
        SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>,
    ),
    SecurityBashSecurityEnabledToggled(
        SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>,
    ),
    AutonomyTabSelected(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
    ObservabilityBackendChanged(
        SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>,
    ),
    ProxyEnabledToggled(SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>),
}

impl<SettingsTab, SearchTarget, AppsCatalog, AppProviders>
    SettingsMessage<SettingsTab, SearchTarget, AppsCatalog, AppProviders>
{
    pub fn route(self) -> SettingsRoute<SettingsTab, SearchTarget, AppsCatalog, AppProviders> {
        match self {
            SettingsMessage::DeviceSyncWebdav(event)  => SettingsRoute::DeviceSyncWebdav(event),
            SettingsMessage::DeviceSyncS3(event)  => SettingsRoute::DeviceSyncS3(event),
            SettingsMessage::MigrationPreview(event)  => SettingsRoute::MigrationPreview(event),
            SettingsMessage::DataBackups(event)  => SettingsRoute::DataBackups(event),
            SettingsMessage::DataRecovery(event)  => SettingsRoute::DataRecovery(event),
            SettingsMessage::DataExport(event)  => SettingsRoute::DataExport(event),
            SettingsMessage::BackupInventory(event)  => SettingsRoute::BackupInventory(event),
            #[cfg(not(target_arch = "wasm32"))]
    SettingsMessage::ComputerControl(event) => SettingsRoute::ComputerControl(event),
            #[cfg(not(target_arch = "wasm32"))]
    SettingsMessage::Voice(event) => SettingsRoute::Voice(event),
            SettingsMessage::ArchivedChats(message)  => SettingsRoute::ArchivedChats(message),
            SettingsMessage::Worktrees(message)  => SettingsRoute::Worktrees(message),
            SettingsMessage::AppsCatalog(message)  => SettingsRoute::AppsCatalog(message),
            #[cfg(not(target_arch = "wasm32"))]
    SettingsMessage::AgentAppsAuth(event) => SettingsRoute::AgentAppsAuth(event),
            SettingsMessage::AppProviderEffect(effect)  => SettingsRoute::AppProviderEffect(effect),
            SettingsMessage::AppProviderEdit(message)  => SettingsRoute::AppProviderEdit(message),
            SettingsMessage::AppProviders(message)  => SettingsRoute::AppProviders(message),
            message @ (SettingsMessage::TabSelected(_)
        | SettingsMessage::SystemTabSelected(_)
        | SettingsMessage::SystemSearchTargetSelected(_)
        | SettingsMessage::ModelConfigTab(_)
        | SettingsMessage::AgentConfigTab(_)
        | SettingsMessage::SystemTabQueryChanged(_)
        | SettingsMessage::SystemHelpOpen(_)
        | SettingsMessage::SystemHelpClose
        | SettingsMessage::ToggleSettingsSidebar ) => SettingsRoute::TabSelected(message),
            message @ SettingsMessage::General(_) => SettingsRoute::General(message),
            message @ SettingsMessage::EmbeddingRoutes(_) => SettingsRoute::EmbeddingRoutes(message),
            message @ SettingsMessage::Hooks(_) => SettingsRoute::Hooks(message),
            message @ SettingsMessage::Storage(_) => SettingsRoute::Storage(message),
            message @ SettingsMessage::ModelRoutes(_) => SettingsRoute::ModelRoutes(message),
            message @ SettingsMessage::QueryClassification(_) => SettingsRoute::QueryClassification(message),
            message @ SettingsMessage::Runtime(_) => SettingsRoute::Runtime(message),
            message @ SettingsMessage::Browser(_) => SettingsRoute::Browser(message),
            message @ SettingsMessage::WebSearch(_) => SettingsRoute::WebSearch(message),
            message @ SettingsMessage::HttpRequest(_) => SettingsRoute::HttpRequest(message),
            message @ SettingsMessage::Memory(_) => SettingsRoute::Memory(message),
            message @ SettingsMessage::Channels(_) => SettingsRoute::Channels(message),
            message @ SettingsMessage::Multimodal(_) => SettingsRoute::Multimodal(message),
            message @ SettingsMessage::Acp(_) => SettingsRoute::Acp(message),
            message @ SettingsMessage::Mcp(_) => SettingsRoute::Mcp(message),
            message @ SettingsMessage::GatewayClient(_) => SettingsRoute::GatewayClient(message),
            message @ SettingsMessage::Gateway(_) => SettingsRoute::Gateway(message),
            message @ SettingsMessage::Tunnel(_) => SettingsRoute::Tunnel(message),
            message @ SettingsMessage::Themes(_) => SettingsRoute::Themes(message),
            message @ SettingsMessage::Agents(_) => SettingsRoute::Agents(message),
            message @ SettingsMessage::QuickMessages(_) => SettingsRoute::QuickMessages(message),
            SettingsMessage::AgentConfigSaved { tag, result } => SettingsRoute::AgentConfigSaved { tag, result },
            message @ (SettingsMessage::ProvidersRefresh
        | SettingsMessage::ProvidersRefreshed(_)
        | SettingsMessage::ProviderModelsSyncRemote
        | SettingsMessage::ProviderModelsSyncTick
        | SettingsMessage::ProviderModelsSyncDone(_)
        | SettingsMessage::ProviderConnectOpen(_)
        | SettingsMessage::ProviderConnectClose
        | SettingsMessage::ProviderConnectApiKeyChanged(_)
        | SettingsMessage::ProviderConnectSubmit
        | SettingsMessage::ProviderConnectDone(_)
        | SettingsMessage::ProviderDisconnectRequested(_)
        | SettingsMessage::ProviderDisconnectCanceled
        | SettingsMessage::ProviderDisconnectConfirmed(_)
        | SettingsMessage::ProviderDisconnect(_)
        | SettingsMessage::ProviderDisconnectDone(_)
        | SettingsMessage::CustomProviderOpen
        | SettingsMessage::CustomProviderEditOpen(_)
        | SettingsMessage::CustomProviderEditLoaded(_)
        | SettingsMessage::CustomProviderClose
        | SettingsMessage::CustomProviderTabSelected(_)
        | SettingsMessage::CustomProviderIdChanged(_)
        | SettingsMessage::CustomProviderNameChanged(_)
        | SettingsMessage::CustomProviderBaseUrlChanged(_)
        | SettingsMessage::CustomProviderApiKeyChanged(_)
        | SettingsMessage::CustomProviderHeaderAdd
        | SettingsMessage::CustomProviderHeaderRemove(_)
        | SettingsMessage::CustomProviderHeaderKeyChanged(_, _)
        | SettingsMessage::CustomProviderHeaderValueChanged(_, _)
        | SettingsMessage::CustomProviderModelOpen(_)
        | SettingsMessage::CustomProviderModelClose
        | SettingsMessage::CustomProviderModelTabSelected(_)
        | SettingsMessage::CustomProviderModelModalIdChanged(_)
        | SettingsMessage::CustomProviderModelModalNameChanged(_)
        | SettingsMessage::CustomProviderModelModalFamilyChanged(_)
        | SettingsMessage::CustomProviderModelModalContextLimitChanged(_)
        | SettingsMessage::CustomProviderModelModalInputLimitChanged(_)
        | SettingsMessage::CustomProviderModelModalOutputLimitChanged(_)
        | SettingsMessage::CustomProviderModelModalReasoningChanged(_)
        | SettingsMessage::CustomProviderModelModalToolCallChanged(_)
        | SettingsMessage::CustomProviderModelModalTemperatureChanged(_)
        | SettingsMessage::CustomProviderModelModalAttachmentChanged(_)
        | SettingsMessage::CustomProviderModelModalInterleavedChanged(_)
        | SettingsMessage::CustomProviderModelModalInputModalityToggled(_, _)
        | SettingsMessage::CustomProviderModelModalOutputModalityToggled(_, _)
        | SettingsMessage::CustomProviderModelModalSave
        | SettingsMessage::CustomProviderModelRemove(_)
        | SettingsMessage::CustomProviderModelEnabledToggled(_, _)
        | SettingsMessage::CustomProviderModelFilterSelected(_)
        | SettingsMessage::CustomProviderModelsDiscover
        | SettingsMessage::CustomProviderModelsDiscovered { .. }
        | SettingsMessage::CustomProviderSave
        | SettingsMessage::CustomProviderSaveDone(_)
        | SettingsMessage::PopularProviderRemove(_)
        | SettingsMessage::PopularProvidersSaved(_)
        | SettingsMessage::ProviderCatalogOpen
        | SettingsMessage::ProviderCatalogClose
        | SettingsMessage::ProviderCatalogQueryChanged(_)
        | SettingsMessage::ProviderCatalogLoaded(_)
        | SettingsMessage::ProviderCatalogAddToPopular(_) ) => SettingsRoute::ProvidersRefresh(message),
            message @ (SettingsMessage::ModelsRefresh
        | SettingsMessage::ModelsRefreshed(_)
        | SettingsMessage::ModelsSetAll(_)
        | SettingsMessage::ProviderModelsSetAll(_, _)
        | SettingsMessage::ProviderModelsReset(_)
        | SettingsMessage::ProviderModelsResetCompleted(_, _)
        | SettingsMessage::ProviderModelsExpansionToggled(_)
        | SettingsMessage::ModelStatusesSaved(_, _)
        | SettingsMessage::ModelQueryChanged(_)
        | SettingsMessage::ModelToggle(_, _, _)
        | SettingsMessage::ModelDetailOpen(_, _)
        | SettingsMessage::ModelDetailClose
        | SettingsMessage::ModelDetailToggleRaw ) => SettingsRoute::ModelsRefresh(message),
            message @ (SettingsMessage::PluginInstallEffect(_)
        | SettingsMessage::PluginsInstallOpen
        | SettingsMessage::PluginsUpdateOpen(_)
        | SettingsMessage::PluginsInstallSourceChanged(_)
        | SettingsMessage::PluginsInstallPrepare
        | SettingsMessage::PluginsInstallPrepared { .. }
        | SettingsMessage::PluginsInstallConfirm
        | SettingsMessage::PluginsInstallCompleted { .. }
        | SettingsMessage::PluginsInstallClose
        | SettingsMessage::PluginsPreviewDiscarded(_) ) => SettingsRoute::PluginInstallEffect(message),
            message @ (SettingsMessage::SkillsDirectoryProviderChanged(_)
        | SettingsMessage::SkillsProjectAutoLoadToggled(_)
        | SettingsMessage::SkillsPromptInjectionModeChanged(_)
        | SettingsMessage::SkillsSave ) => SettingsRoute::SkillsDirectoryProviderChanged(message),
            message @ (SettingsMessage::SkillsProjectSelected(_)
        | SettingsMessage::SkillsRefresh
        | SettingsMessage::SkillsLoaded(_)
        | SettingsMessage::PluginsRefresh
        | SettingsMessage::PluginsLoaded(_)
        | SettingsMessage::PluginsSelected(_)
        | SettingsMessage::PluginsSetEnabledRequested { .. }
        | SettingsMessage::PluginsSetEnabledCompleted { .. }
        | SettingsMessage::PluginsDeleteRequested(_)
        | SettingsMessage::PluginsDeleteCompleted { .. }
        | SettingsMessage::SkillsTabChanged(_)
        | SettingsMessage::SkillsDetailClosed
        | SettingsMessage::SkillsDetailRequested(_)
        | SettingsMessage::SkillsDetailLoaded { .. }
        | SettingsMessage::SkillsQueryChanged(_)
        | SettingsMessage::SkillsDirectoryScopeChanged(_)
        | SettingsMessage::SkillsCreateNewRequested
        | SettingsMessage::SkillsCreateNewCompleted(_)
        | SettingsMessage::SkillsCopyInstallCommand
        | SettingsMessage::SkillsInstallBuiltInRequested(_)
        | SettingsMessage::SkillsInstallBuiltInCompleted(_)
        | SettingsMessage::SkillsSetEnabledRequested { .. }
        | SettingsMessage::SkillsSetEnabledCompleted { .. }
        | SettingsMessage::SkillsDeleteRequested(_)
        | SettingsMessage::SkillsDeleteCompleted { .. }
        | SettingsMessage::SkillsBackups(_)
        | SettingsMessage::SkillsStorage(_)
        | SettingsMessage::SkillsBindings(_)
        | SettingsMessage::SkillsClaudeConfig(_)
        | SettingsMessage::SkillsJobs(_)
        | SettingsMessage::SkillsUpdates(_)
        | SettingsMessage::SkillsInstall(_)
        | SettingsMessage::SkillsMarket(_)
        | SettingsMessage::SkillsImport(_)
        | SettingsMessage::SkillRepositories(_)
        | SettingsMessage::SkillsHelpOpen
        | SettingsMessage::SkillsHelpClose ) => SettingsRoute::SkillsProjectSelected(message),
            message @ (SettingsMessage::DialogueFlowPermissionRefresh
        | SettingsMessage::DialogueFlowPermissionLoaded(_)
        | SettingsMessage::DialogueFlowPermissionReset
        | SettingsMessage::DialogueFlowUiSettingsLoaded(..)
        | SettingsMessage::DialogueFlowUiSettingsSave
        | SettingsMessage::DialogueFlowUiSettingsSaved(_)
        | SettingsMessage::DialogueFlowShowReasoningSummaryToggled(_)
        | SettingsMessage::DialogueFlowExpandShellToolSectionToggled(_)
        | SettingsMessage::DialogueFlowExpandEditToolSectionToggled(_) ) => SettingsRoute::DialogueFlowPermissionRefresh(message),
            message @ (SettingsMessage::RecentProjectQueryChanged(_)
        | SettingsMessage::RecentProjectDeleteRequested(_)
        | SettingsMessage::RecentProjectDeleteCanceled
        | SettingsMessage::RecentProjectDeleteConfirmed(_)
        | SettingsMessage::ProjectEnableWorktreeToggled(_, _)
        | SettingsMessage::ProjectWorktreeSaved(_) ) => SettingsRoute::RecentProjectQueryChanged(message),
            message @ (SettingsMessage::SessionDelete(_) | SettingsMessage::SessionCopy(_) ) => SettingsRoute::SessionDelete(message),
            message @ (SettingsMessage::HeartbeatEnabledToggled(_)
        | SettingsMessage::HeartbeatIntervalChanged(_)
        | SettingsMessage::HeartbeatMessageChanged(_)
        | SettingsMessage::HeartbeatTargetChanged(_)
        | SettingsMessage::HeartbeatToChanged(_)
        | SettingsMessage::HeartbeatSave
        | SettingsMessage::HeartbeatHelpOpen
        | SettingsMessage::HeartbeatHelpClose ) => SettingsRoute::HeartbeatEnabledToggled(message),
            SettingsMessage::CronHistory(event)  => SettingsRoute::CronHistory(event),
            message @ (SettingsMessage::CronFullAccessConfirm { .. }
        | SettingsMessage::CronFullAccessCancel { .. }
        | SettingsMessage::CronPickerOpen { .. }
        | SettingsMessage::CronPickerNavigate(_)
        | SettingsMessage::CronPickerDate(_)
        | SettingsMessage::CronPickerTime { .. }
        | SettingsMessage::CronPickerConfirm
        | SettingsMessage::CronPickerCancel
        | SettingsMessage::CronScheduleDraftChanged { .. }
        | SettingsMessage::CronSchedulePreview { .. }
        | SettingsMessage::CronSchedulePreviewLoaded { .. }
        | SettingsMessage::CronEnabledToggled(_)
        | SettingsMessage::CronMaxRunHistoryChanged(_)
        | SettingsMessage::CronTabSelected(_)
        | SettingsMessage::CronTemplatesToggled(_)
        | SettingsMessage::CronTemplateSelected(_)
        | SettingsMessage::CronListFilterChanged(_)
        | SettingsMessage::CronBatchModeToggled(_)
        | SettingsMessage::CronJobsRefresh
        | SettingsMessage::CronJobsLoaded(_)
        | SettingsMessage::CronJobSelectionToggled(_, _)
        | SettingsMessage::CronJobDetailsToggled(_, _)
        | SettingsMessage::CronJobsSelectAllToggled(_)
        | SettingsMessage::CronJobRunsOpen(_)
        | SettingsMessage::CronJobRunsLoaded(_, _)
        | SettingsMessage::CronJobRunsEditorAction(_)
        | SettingsMessage::CronJobRunsClose
        | SettingsMessage::CronJobEditStarted(_)
        | SettingsMessage::CronJobEditCanceled
        | SettingsMessage::CronJobEditNameChanged(_)
        | SettingsMessage::CronJobEditJobTypeChanged(_)
        | SettingsMessage::CronJobEditScheduleKindChanged(_)
        | SettingsMessage::CronJobEditScheduleChanged(_)
        | SettingsMessage::CronJobEditFrequencyChanged(_)
        | SettingsMessage::CronJobEditAtChanged(_)
        | SettingsMessage::CronJobEditEveryMsChanged(_)
        | SettingsMessage::CronJobEditCommandChanged(_)
        | SettingsMessage::CronJobEditCommandEditorAction(_)
        | SettingsMessage::CronJobEditPromptChanged(_)
        | SettingsMessage::CronJobEditPromptEditorAction(_)
        | SettingsMessage::CronJobEditAgentChanged(_)
        | SettingsMessage::CronJobEditAcpAgentChanged(_)
        | SettingsMessage::CronJobEditProjectPathChanged(_)
        | SettingsMessage::CronJobEditModelProviderChanged(_)
        | SettingsMessage::CronJobEditModelChanged(_)
        | SettingsMessage::CronJobEditWakeToggled(_)
        | SettingsMessage::CronJobEditFallbacksChanged(_)
        | SettingsMessage::CronJobEditFullAccessToggled(_)
        | SettingsMessage::CronJobEditTaskPoolToggled(_)
        | SettingsMessage::CronJobEditDeliveryEnabledToggled(_)
        | SettingsMessage::CronJobEditDeliveryChannelChanged(_)
        | SettingsMessage::CronJobEditDeliveryToChanged(_)
        | SettingsMessage::CronJobEditDeliveryBestEffortToggled(_)
        | SettingsMessage::CronJobEditDeleteAfterRunToggled(_)
        | SettingsMessage::CronJobEditSave
        | SettingsMessage::CronJobEnabledChanged(_, _)
        | SettingsMessage::CronJobDelete(_)
        | SettingsMessage::CronSelectedJobsEnable
        | SettingsMessage::CronSelectedJobsDisable
        | SettingsMessage::CronSelectedJobsDelete
        | SettingsMessage::CronAddNameChanged(_)
        | SettingsMessage::CronAddJobTypeChanged(_)
        | SettingsMessage::CronAddScheduleKindChanged(_)
        | SettingsMessage::CronAddScheduleChanged(_)
        | SettingsMessage::CronAddFrequencyChanged(_)
        | SettingsMessage::CronAddAtChanged(_)
        | SettingsMessage::CronAddEveryMsChanged(_)
        | SettingsMessage::CronAddCommandChanged(_)
        | SettingsMessage::CronAddCommandEditorAction(_)
        | SettingsMessage::CronAddPromptChanged(_)
        | SettingsMessage::CronAddPromptEditorAction(_)
        | SettingsMessage::CronAddSessionTargetChanged(_)
        | SettingsMessage::CronAddAgentChanged(_)
        | SettingsMessage::CronAddAcpAgentChanged(_)
        | SettingsMessage::CronAddProjectPathChanged(_)
        | SettingsMessage::CronAddModelProviderChanged(_)
        | SettingsMessage::CronAddModelChanged(_)
        | SettingsMessage::CronAddWakeToggled(_)
        | SettingsMessage::CronAddFallbacksChanged(_)
        | SettingsMessage::CronAddFullAccessToggled(_)
        | SettingsMessage::CronAddTaskPoolToggled(_)
        | SettingsMessage::CronAddDeliveryEnabledToggled(_)
        | SettingsMessage::CronAddDeliveryChannelChanged(_)
        | SettingsMessage::CronAddDeliveryToChanged(_)
        | SettingsMessage::CronAddDeliveryBestEffortToggled(_)
        | SettingsMessage::CronAddDeleteAfterRunToggled(_)
        | SettingsMessage::CronAddSubmit
        | SettingsMessage::CronEditorSaveCompleted { .. }
        | SettingsMessage::CronJobMutationCompleted(_)
        | SettingsMessage::CronSave
        | SettingsMessage::CronHelpOpen
        | SettingsMessage::CronHelpClose ) => SettingsRoute::CronFullAccessConfirm(message),
            message @ (SettingsMessage::SchedulerEnabledToggled(_)
        | SettingsMessage::SchedulerMaxTasksChanged(_)
        | SettingsMessage::SchedulerMaxConcurrentChanged(_)
        | SettingsMessage::SchedulerSave
        | SettingsMessage::SchedulerHelpOpen
        | SettingsMessage::SchedulerHelpClose ) => SettingsRoute::SchedulerEnabledToggled(message),
            message @ (SettingsMessage::ReliabilityProviderRetriesChanged(_)
        | SettingsMessage::ReliabilityProviderBackoffMsChanged(_)
        | SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(_)
        | SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(_)
        | SettingsMessage::ReliabilitySchedulerPollSecsChanged(_)
        | SettingsMessage::ReliabilitySchedulerRetriesChanged(_)
        | SettingsMessage::ReliabilitySave
        | SettingsMessage::ReliabilityHelpOpen
        | SettingsMessage::ReliabilityHelpClose ) => SettingsRoute::ReliabilityProviderRetriesChanged(message),
            message @ (SettingsMessage::SecurityBashSecurityEnabledToggled(_)
        | SettingsMessage::SecurityBashPolicyCheckToggled(_, _)
        | SettingsMessage::SecurityBashValidatorCheckToggled(_, _)
        | SettingsMessage::SecurityTabSelected(_)
        | SettingsMessage::SecurityListDraftChanged(_, _)
        | SettingsMessage::SecurityListAdd(_)
        | SettingsMessage::SecurityListRemoveRequested(_, _)
        | SettingsMessage::SecurityListRemoveConfirmed(_, _)
        | SettingsMessage::SecurityListRemoveCanceled
        | SettingsMessage::SecuritySandboxEnabledChanged(_)
        | SettingsMessage::SecuritySandboxBackendChanged(_)
        | SettingsMessage::SecurityResourcesMaxMemoryMbChanged(_)
        | SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(_)
        | SettingsMessage::SecurityResourcesMaxSubprocessesChanged(_)
        | SettingsMessage::SecurityResourcesMemoryMonitoringToggled(_)
        | SettingsMessage::SecurityAuditEnabledToggled(_)
        | SettingsMessage::SecurityAuditLogPathChanged(_)
        | SettingsMessage::SecurityAuditMaxSizeMbChanged(_)
        | SettingsMessage::SecurityAuditSignEventsToggled(_)
        | SettingsMessage::SecurityOtpEnabledToggled(_)
        | SettingsMessage::SecurityOtpMethodChanged(_)
        | SettingsMessage::SecurityOtpTokenTtlSecsChanged(_)
        | SettingsMessage::SecurityOtpCacheValidSecsChanged(_)
        | SettingsMessage::SecurityEstopEnabledToggled(_)
        | SettingsMessage::SecurityEstopStateFileChanged(_)
        | SettingsMessage::SecurityEstopRequireOtpToResumeToggled(_)
        | SettingsMessage::SecuritySyscallAnomalyEnabledToggled(_)
        | SettingsMessage::SecuritySyscallAnomalyStrictModeToggled(_)
        | SettingsMessage::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(_)
        | SettingsMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(_)
        | SettingsMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(_)
        | SettingsMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(_)
        | SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(_)
        | SettingsMessage::SecuritySyscallAnomalyLogPathChanged(_)
        | SettingsMessage::SecurityCanaryTokensToggled(_)
        | SettingsMessage::SecuritySemanticGuardToggled(_)
        | SettingsMessage::SecuritySemanticGuardCollectionChanged(_)
        | SettingsMessage::SecuritySemanticGuardThresholdChanged(_)
        | SettingsMessage::SecuritySave
        | SettingsMessage::SecurityHelpOpen
        | SettingsMessage::SecurityHelpClose ) => SettingsRoute::SecurityBashSecurityEnabledToggled(message),
            message @ (SettingsMessage::AutonomyTabSelected(_)
        | SettingsMessage::AutonomyLevelChanged(_)
        | SettingsMessage::AutonomyWorkspaceOnlyToggled(_)
        | SettingsMessage::AutonomyListDraftChanged(_, _)
        | SettingsMessage::AutonomyListAdd(_)
        | SettingsMessage::AutonomyListRemoveRequested(_, _)
        | SettingsMessage::AutonomyListRemoveConfirmed(_, _)
        | SettingsMessage::AutonomyListRemoveCanceled
        | SettingsMessage::AutonomyMaxActionsPerHourChanged(_)
        | SettingsMessage::AutonomyMaxCostPerDayCentsChanged(_)
        | SettingsMessage::AutonomyRequireApprovalForMediumRiskToggled(_)
        | SettingsMessage::AutonomyBlockHighRiskCommandsToggled(_)
        | SettingsMessage::AutonomyShellRedirectPolicyChanged(_)
        | SettingsMessage::AutonomyNonCliApprovalApproversChanged(_)
        | SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(_)
        | SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(_)
        | SettingsMessage::AutonomySave
        | SettingsMessage::AutonomyHelpOpen
        | SettingsMessage::AutonomyHelpClose ) => SettingsRoute::AutonomyTabSelected(message),
            message @ (SettingsMessage::ObservabilityBackendChanged(_)
        | SettingsMessage::ObservabilityOtelEndpointChanged(_)
        | SettingsMessage::ObservabilityOtelServiceNameChanged(_)
        | SettingsMessage::ObservabilityRuntimeTraceModeChanged(_)
        | SettingsMessage::ObservabilityRuntimeTracePathChanged(_)
        | SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(_)
        | SettingsMessage::ObservabilityHelpOpen
        | SettingsMessage::ObservabilityHelpClose ) => SettingsRoute::ObservabilityBackendChanged(message),
            message @ (SettingsMessage::ProxyEnabledToggled(_)
        | SettingsMessage::ProxyScopeTextChanged(_)
        | SettingsMessage::ProxyHttpChanged(_)
        | SettingsMessage::ProxyHttpsChanged(_)
        | SettingsMessage::ProxyAllChanged(_)
        | SettingsMessage::ProxyNoProxyChanged(_)
        | SettingsMessage::ProxyServicesChanged(_)
        | SettingsMessage::ProxyHelpOpen
        | SettingsMessage::ProxyHelpClose ) => SettingsRoute::ProxyEnabledToggled(message),
        }
    }
}
