use super::*;
pub use desktop_settings_providers::provider_state::*;

pub use desktop_settings_providers::state::{ModelSummary, ProviderModelsSummary, ModelDetailRow, ModelDetailModalState, ModelConfigTab, ModelSettingsState};

/// 嵌入路由草稿
///
/// 表示系统设置界面中一条可编辑的嵌入路由输入。
pub(crate) use desktop_settings_providers::app_init::state::{EmbeddingRouteDraft, EmbeddingRoutesSettingsState};

pub use desktop_settings_providers::state::ModelRoute;

/// 模型路由设置面板状态。
#[derive(Debug, Clone, Default)]
pub(crate) struct ModelRoutesSettingsState {
    pub(crate) persistence: crate::app::message::settings::routing_persistence::RoutingPersistence,
    /// 路由列表。
    pub(crate) routes: Vec<ModelRoute>,
    /// 保存错误信息。
    pub(crate) save_error: Option<crate::app::i18n::settings_models::messages::Feedback>,
}

/// 心跳设置面板状态
///
/// 管理心跳功能的配置，包括定时消息发送设置。
pub(crate) use desktop_settings_cron::app_init::state::{HeartbeatSettingsState};

pub(crate) use desktop_settings_cron::state::{
    CronSettingsTab, CronAddJobType, CronAddScheduleKind, CronJobDraft, CronListFilter,
    CronSettingsState,
};

/// 调度器设置面板状态
///
/// 管理任务调度器的配置，包括并发限制等。
pub(crate) use desktop_settings_cron::app_init::state::{SchedulerSettingsState};

#[cfg(test)]
#[path = "provider_model_tests.rs"]
mod provider_model_tests;

pub(crate) use desktop_settings_runtime::state::{QueryClassificationRuleInput, QueryClassificationSettingsState};
