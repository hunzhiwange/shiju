//! 浏览器与网关设置的局部消息。
use iced::widget::text_editor;
use crate::state::GatewaySettingsTab;

#[derive(Debug, Clone)]
pub enum BrowserMessage {
    Refresh,
    Install,
    InstallCompleted(Result<String, String>),
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
