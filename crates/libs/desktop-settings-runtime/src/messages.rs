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
    ReasoningEnabledChanged(String),
    ReasoningLevelChanged(String),
}

#[derive(Debug, Clone)]
pub enum MultimodalMessage {
    MaxImagesChanged(u32),
    MaxImageSizeMbChanged(u32),
    AllowRemoteFetchToggled(bool),
    OcrEnabledToggled(bool),
    ModelSupportsImagesToggled(bool),
    OcrModelChanged(String),
}

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
pub type HookCatalogLoaded = Result<Vec<gateway_desktop_settings::HookCatalogEntryDto>, String>;
pub type HookAuditLoaded = Result<Vec<gateway_desktop_settings::HookAuditEntryDto>, String>;
pub type HookMutationCompleted = Result<gateway_desktop_settings::HookMutationResponseDto, String>;
#[derive(Debug, Clone)]
pub enum SettingsMessage {
ReliabilityProviderRetriesChanged(u32),
ReliabilityProviderBackoffMsChanged(u64),
ReliabilityChannelInitialBackoffSecsChanged(u64),
ReliabilityChannelMaxBackoffSecsChanged(u64),
ReliabilitySchedulerPollSecsChanged(u64),
ReliabilitySchedulerRetriesChanged(u32),
ReliabilitySave,
ReliabilityHelpOpen,
ReliabilityHelpClose,
ObservabilityBackendChanged(String),
ObservabilityOtelEndpointChanged(String),
ObservabilityOtelServiceNameChanged(String),
ObservabilityRuntimeTraceModeChanged(String),
ObservabilityRuntimeTracePathChanged(String),
ObservabilityRuntimeTraceMaxEntriesChanged(u32),
ObservabilityHelpOpen,
ObservabilityHelpClose,
Hooks(HooksMessage),
Runtime(RuntimeMessage),
Multimodal(MultimodalMessage),
QueryClassification(QueryClassificationMessage),
NavigateModelRoutes,
}
