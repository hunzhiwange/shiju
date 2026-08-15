//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

mod acp;
mod agents;
mod agents_ipc;
mod autonomy;
mod browser;
mod channels;
mod coordination;
mod cron;
mod dialogue_flow;
mod embedding_routes;
mod gateway;
mod gateway_client;
mod general;
mod heartbeat;
mod hooks;
mod http_request;
mod mcp;
mod memory;
mod messages;
mod model_routes;
mod models;
mod multimodal;
mod observability;
mod projects;
mod providers;
mod proxy;
mod query_classification;
mod quick_messages;
mod reliability;
mod runtime;
mod scheduler;
mod security;
mod sessions;
mod skills;
mod staff;
mod storage;
mod tabs;
mod themes;
mod tunnel;
mod types;
mod util;
mod web_search;

use crate::app::config::server_config_unreachable_error;
use crate::app::{App, Message};
use iced::Task;

pub(crate) use messages::{
    AcpMessage, AgentsMessage, BashPolicyCheck, BashValidatorCheck, BrowserMessage,
    ChannelsMessage, EmbeddingRoutesMessage, GatewayClientMessage, GatewayMessage,
    GeneralSettingsMessage, HooksMessage, HttpRequestMessage, McpMessage, MemoryMessage,
    ModelRoutesMessage, MultimodalMessage, QueryClassificationMessage, QuickMessagesMessage,
    RuntimeMessage, SettingsMessage, StaffMessage, StaffResourceMessage, StaffTeamDraftField,
    StaffTeamMemberDraftField, StaffWorkerDraftField, StorageMessage, ThemeChromeField,
    ThemeColorPickerTarget, ThemePaletteField, ThemesMessage, TunnelMessage, WebSearchMessage,
};

pub(crate) fn reload_agents_config_task(project_path: Option<String>) -> Task<Message> {
    agents::config_task(project_path)
}

fn apply_agent_config_saved(app: &mut App, tag: &'static str, result: Result<(), String>) {
    let save_error = result.err().map(server_config_unreachable_error);

    match tag {
        "heartbeat" => app.heartbeat_settings.save_error = save_error,
        "cron" => app.cron_settings.save_error = save_error,
        "scheduler" => app.scheduler_settings.save_error = save_error,
        "reliability" => app.reliability_settings.save_error = save_error,
        "memory" => app.memory_settings.save_error = save_error,
        "security" => app.security_settings.save_error = save_error,
        "channels_config" => app.channels_settings.save_error = save_error,
        "observability" => app.observability_settings.save_error = save_error,
        "storage" => app.storage_settings.save_error = save_error,
        "proxy" => app.proxy_settings.save_error = save_error,
        "browser" => app.browser_settings.save_error = save_error,
        "http_request" => app.http_request_settings.save_error = save_error,
        "multimodal" => app.multimodal_settings.save_error = save_error,
        "web_search" => app.web_search_settings.save_error = save_error,
        "tunnel" => app.tunnel_settings.save_error = save_error,
        "hooks" => app.hooks_settings.save_error = save_error,
        "skills" => app.skills_settings.save_error = save_error,
        "staff" => {
            let result = match save_error {
                Some(error) => Err(error),
                None => Ok(()),
            };
            staff::apply_saved_result(app, result);
        }
        "runtime" => app.runtime_settings.save_error = save_error,
        "model_routes" => app.model_routes_settings.save_error = save_error,
        "embedding_routes" => app.embedding_routes_settings.save_error = save_error,
        "query_classification" => app.query_classification_settings.save_error = save_error,
        "autonomy" => app.autonomy_settings.save_error = save_error,
        "agents_ipc" => app.agents_ipc_settings.save_error = save_error,
        "coordination" => app.coordination_settings.save_error = save_error,
        _ => {}
    }
}

#[cfg(test)]
mod tests;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::TabSelected(_)
        | SettingsMessage::SystemTabSelected(_)
        | SettingsMessage::SystemTabQueryChanged(_)
        | SettingsMessage::SystemHelpOpen(_)
        | SettingsMessage::SystemHelpClose
        | SettingsMessage::ToggleSettingsSidebar => tabs::update(app, message),
        SettingsMessage::General(_) => general::update(app, message),
        SettingsMessage::EmbeddingRoutes(_) => embedding_routes::update(app, message),
        SettingsMessage::Hooks(_) => hooks::update(app, message),
        SettingsMessage::Storage(_) => storage::update(app, message),
        SettingsMessage::ModelRoutes(_) => model_routes::update(app, message),
        SettingsMessage::QueryClassification(_) => query_classification::update(app, message),
        SettingsMessage::Runtime(_) => runtime::update(app, message),
        SettingsMessage::Browser(_) => browser::update(app, message),
        SettingsMessage::WebSearch(_) => web_search::update(app, message),
        SettingsMessage::HttpRequest(_) => http_request::update(app, message),
        SettingsMessage::Memory(_) => memory::update(app, message),
        SettingsMessage::Channels(_) => channels::update(app, message),
        SettingsMessage::Multimodal(_) => multimodal::update(app, message),
        SettingsMessage::Acp(_) => acp::update(app, message),
        SettingsMessage::Mcp(_) => mcp::update(app, message),
        SettingsMessage::GatewayClient(_) => gateway_client::update(app, message),
        SettingsMessage::Gateway(_) => gateway::update(app, message),
        SettingsMessage::Tunnel(_) => tunnel::update(app, message),
        SettingsMessage::Themes(_) => themes::update(app, message),
        SettingsMessage::Agents(_) => agents::update(app, message),
        SettingsMessage::Staff(_) => staff::update(app, message),
        SettingsMessage::QuickMessages(_) => quick_messages::update(app, message),
        SettingsMessage::AgentConfigSaved { tag, result } => {
            if tag == "agent" {
                agents::handle_agents_config_saved(app, result)
            } else {
                apply_agent_config_saved(app, tag, result);
                Task::none()
            }
        }
        SettingsMessage::ProvidersRefresh
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
        | SettingsMessage::ProviderCatalogAddToPopular(_) => providers::update(app, message),
        SettingsMessage::ModelsRefresh
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
        | SettingsMessage::ModelDetailToggleRaw => models::update(app, message),
        SettingsMessage::SkillsProjectSelected(_)
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
        | SettingsMessage::SkillsDirectoryProviderChanged(_)
        | SettingsMessage::SkillsProjectAutoLoadToggled(_)
        | SettingsMessage::SkillsOpenEnabledToggled(_)
        | SettingsMessage::SkillsOpenDirChanged(_)
        | SettingsMessage::SkillsPromptInjectionModeChanged(_)
        | SettingsMessage::SkillsSave
        | SettingsMessage::SkillsHelpOpen
        | SettingsMessage::SkillsHelpClose => skills::update(app, message),
        SettingsMessage::DialogueFlowPermissionRefresh
        | SettingsMessage::DialogueFlowPermissionLoaded(_)
        | SettingsMessage::DialogueFlowPermissionReset
        | SettingsMessage::DialogueFlowUiSettingsLoaded(_)
        | SettingsMessage::DialogueFlowUiSettingsSave
        | SettingsMessage::DialogueFlowUiSettingsSaved(_)
        | SettingsMessage::DialogueFlowShowReasoningSummaryToggled(_)
        | SettingsMessage::DialogueFlowExpandShellToolSectionToggled(_)
        | SettingsMessage::DialogueFlowExpandEditToolSectionToggled(_) => {
            dialogue_flow::update(app, message)
        }
        SettingsMessage::RecentProjectRenameChanged(_, _)
        | SettingsMessage::RecentProjectRenameSave(_)
        | SettingsMessage::RecentProjectDeleteRequested(_)
        | SettingsMessage::RecentProjectDeleteCanceled
        | SettingsMessage::RecentProjectDeleteConfirmed(_)
        | SettingsMessage::ProjectEnableWorktreeToggled(_, _) => projects::update(app, message),
        SettingsMessage::SessionDelete(_) | SettingsMessage::SessionCopy(_) => {
            sessions::update(app, message)
        }
        SettingsMessage::HeartbeatEnabledToggled(_)
        | SettingsMessage::HeartbeatIntervalChanged(_)
        | SettingsMessage::HeartbeatMessageChanged(_)
        | SettingsMessage::HeartbeatTargetChanged(_)
        | SettingsMessage::HeartbeatToChanged(_)
        | SettingsMessage::HeartbeatSave
        | SettingsMessage::HeartbeatHelpOpen
        | SettingsMessage::HeartbeatHelpClose => heartbeat::update(app, message),
        SettingsMessage::CronEnabledToggled(_)
        | SettingsMessage::CronMaxRunHistoryChanged(_)
        | SettingsMessage::CronTabSelected(_)
        | SettingsMessage::CronJobsRefresh
        | SettingsMessage::CronJobsLoaded(_)
        | SettingsMessage::CronJobSelectionToggled(_, _)
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
        | SettingsMessage::CronJobMutationCompleted(_)
        | SettingsMessage::CronSave
        | SettingsMessage::CronHelpOpen
        | SettingsMessage::CronHelpClose => cron::update(app, message),
        SettingsMessage::SchedulerEnabledToggled(_)
        | SettingsMessage::SchedulerMaxTasksChanged(_)
        | SettingsMessage::SchedulerMaxConcurrentChanged(_)
        | SettingsMessage::SchedulerSave
        | SettingsMessage::SchedulerHelpOpen
        | SettingsMessage::SchedulerHelpClose => scheduler::update(app, message),
        SettingsMessage::AgentsIpcEnabledToggled(_)
        | SettingsMessage::AgentsIpcDbPathChanged(_)
        | SettingsMessage::AgentsIpcStalenessSecsChanged(_)
        | SettingsMessage::AgentsIpcSave
        | SettingsMessage::AgentsIpcHelpOpen
        | SettingsMessage::AgentsIpcHelpClose => agents_ipc::update(app, message),
        SettingsMessage::CoordinationEnabledToggled(_)
        | SettingsMessage::CoordinationLeadAgentChanged(_)
        | SettingsMessage::CoordinationMaxInboxMessagesPerAgentChanged(_)
        | SettingsMessage::CoordinationMaxDeadLettersChanged(_)
        | SettingsMessage::CoordinationMaxContextEntriesChanged(_)
        | SettingsMessage::CoordinationMaxSeenMessageIdsChanged(_)
        | SettingsMessage::CoordinationSave
        | SettingsMessage::CoordinationHelpOpen
        | SettingsMessage::CoordinationHelpClose => coordination::update(app, message),
        SettingsMessage::ReliabilityProviderRetriesChanged(_)
        | SettingsMessage::ReliabilityProviderBackoffMsChanged(_)
        | SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(_)
        | SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(_)
        | SettingsMessage::ReliabilitySchedulerPollSecsChanged(_)
        | SettingsMessage::ReliabilitySchedulerRetriesChanged(_)
        | SettingsMessage::ReliabilitySave
        | SettingsMessage::ReliabilityHelpOpen
        | SettingsMessage::ReliabilityHelpClose => reliability::update(app, message),
        SettingsMessage::SecurityBashSecurityEnabledToggled(_)
        | SettingsMessage::SecurityBashPolicyCheckToggled(_, _)
        | SettingsMessage::SecurityBashValidatorCheckToggled(_, _)
        | SettingsMessage::SecurityTabSelected(_)
        | SettingsMessage::SecurityListDraftChanged(_, _)
        | SettingsMessage::SecurityListAdd(_)
        | SettingsMessage::SecurityListRemove(_, _)
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
        | SettingsMessage::SecurityHelpClose => security::update(app, message),
        SettingsMessage::AutonomyTabSelected(_)
        | SettingsMessage::AutonomyLevelChanged(_)
        | SettingsMessage::AutonomyWorkspaceOnlyToggled(_)
        | SettingsMessage::AutonomyListDraftChanged(_, _)
        | SettingsMessage::AutonomyListAdd(_)
        | SettingsMessage::AutonomyListRemove(_, _)
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
        | SettingsMessage::AutonomyHelpClose => autonomy::update(app, message),
        SettingsMessage::ObservabilityBackendChanged(_)
        | SettingsMessage::ObservabilityOtelEndpointChanged(_)
        | SettingsMessage::ObservabilityOtelServiceNameChanged(_)
        | SettingsMessage::ObservabilityRuntimeTraceModeChanged(_)
        | SettingsMessage::ObservabilityRuntimeTracePathChanged(_)
        | SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(_)
        | SettingsMessage::ObservabilityHelpOpen
        | SettingsMessage::ObservabilityHelpClose => observability::update(app, message),
        SettingsMessage::ProxyEnabledToggled(_)
        | SettingsMessage::ProxyScopeTextChanged(_)
        | SettingsMessage::ProxyHttpChanged(_)
        | SettingsMessage::ProxyHttpsChanged(_)
        | SettingsMessage::ProxyAllChanged(_)
        | SettingsMessage::ProxyNoProxyChanged(_)
        | SettingsMessage::ProxyServicesChanged(_)
        | SettingsMessage::ProxyHelpOpen
        | SettingsMessage::ProxyHelpClose => proxy::update(app, message),
    }
}
