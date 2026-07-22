//! 定义系统设置异步加载结果类型别名，统一各设置模块的任务返回值。

use vw_shared::provider::types as model_provider;

use crate::app::state::{ModelCatalogEntry, ProviderModelsSummary, ProviderSummary};
use vw_config_types::staff::StaffConfig;

/// `ProvidersLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type ProvidersLoaded = Result<
    (
        std::collections::HashMap<String, model_provider::Info>,
        Vec<String>,
        bool,
        Vec<ModelCatalogEntry>,
    ),
    String,
>;

/// `ModelsLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type ModelsLoaded = Result<Vec<ProviderModelsSummary>, String>;
/// `SkillsLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type SkillsLoaded = Result<Vec<vw_gateway_client::DesktopSkillCatalogEntryDto>, String>;
/// `PluginsLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type PluginsLoaded = Result<Vec<vw_gateway_client::DesktopPluginCatalogEntryDto>, String>;
/// `AgentsLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type AgentsLoaded =
    Result<(Vec<ProviderSummary>, Vec<ProviderModelsSummary>, Vec<String>), String>;
/// `StaffLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type StaffLoaded = Result<StaffConfig, String>;
/// `DialogueFlowPermissionLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type DialogueFlowPermissionLoaded = Result<String, String>;
/// `DialogueFlowUiSettingsLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type DialogueFlowUiSettingsLoaded = Result<(bool, bool, bool), String>;
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
