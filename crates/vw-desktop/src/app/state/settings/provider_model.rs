pub use desktop_settings_providers::provider_state::*;

pub use desktop_settings_providers::state::{
    ModelConfigTab, ModelSettingsState,
};

/// 嵌入路由草稿
///
/// 表示系统设置界面中一条可编辑的嵌入路由输入。
pub(crate) use desktop_settings_providers::app_init::state::EmbeddingRoutesSettingsState;


pub use desktop_settings_providers::state::ModelRoutesSettingsState;

/// 心跳设置面板状态
///
/// 管理心跳功能的配置，包括定时消息发送设置。
pub(crate) use desktop_settings_cron::app_init::state::HeartbeatSettingsState;

pub(crate) use desktop_settings_cron::state::{
    CronSettingsState,
    CronSettingsTab,
};

/// 调度器设置面板状态
///
/// 管理任务调度器的配置，包括并发限制等。
pub(crate) use desktop_settings_cron::app_init::state::SchedulerSettingsState;



pub(crate) use desktop_settings_runtime::state::{
    QueryClassificationRuleInput, QueryClassificationSettingsState,
};

#[cfg(test)]
pub use desktop_settings_providers::state::ModelDetailModalState;
#[cfg(test)]
pub use desktop_settings_providers::state::ModelDetailRow;
#[cfg(test)]
pub use desktop_settings_providers::state::ModelRoute;
#[cfg(test)]
pub use desktop_settings_providers::state::ModelSummary;
#[cfg(test)]
pub use desktop_settings_providers::state::ProviderModelsSummary;
#[cfg(test)]
pub(crate) use desktop_settings_cron::state::CronAddJobType;
#[cfg(test)]
pub(crate) use desktop_settings_cron::state::CronAddScheduleKind;
#[cfg(test)]
pub(crate) use desktop_settings_cron::state::CronJobDraft;
#[cfg(test)]
pub(crate) use desktop_settings_cron::state::CronListFilter;
#[cfg(test)]
pub(crate) use desktop_settings_providers::app_init::state::EmbeddingRouteDraft;
