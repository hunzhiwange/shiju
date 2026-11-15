//! 运行时、安全与代理设置 API 契约。

pub mod settings;

pub use settings::{
    AutonomyLevelDto,
    ShellRedirectPolicyDto,
    NetworkAccessDto,
    SandboxBackendDto,
    SecuritySettingsDto,
    RuntimeSettingsDto,
    ProxySettingsDto,
    MemorySettingsDto,
    WebSearchSettingsDto,
    ChannelsSettingsDto,
    BrowserSettingsDto,
    DelegateAgentDto,
    SettingsDto,
    GetSettingsResponse,
    SecuritySettingsPatchDto,
    RuntimeSettingsPatchDto,
    ProxySettingsPatchDto,
    MemorySettingsPatchDto,
    WebSearchSettingsPatchDto,
    ChannelsSettingsPatchDto,
    BrowserSettingsPatchDto,
    SettingsPatchDto,
    DelegateAgentsResponse,
    PatchDelegateAgentsRequest,
    PatchDelegateAgentDto,
};

#[cfg(test)]
mod settings_tests;
