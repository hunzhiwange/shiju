//! 系统设置中 providers 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

pub mod catalog;
/// `connect` 子模块，承载该区域拆分后的具体界面或辅助逻辑。
///
/// 公开模块边界便于父模块按职责组合页面，同时保持实现文件可以独立维护。
pub mod connect;
/// `connected` 子模块，承载该区域拆分后的具体界面或辅助逻辑。
///
/// 公开模块边界便于父模块按职责组合页面，同时保持实现文件可以独立维护。
pub mod connected;
/// `custom_model_modal` 子模块，承载该区域拆分后的具体界面或辅助逻辑。
///
/// 公开模块边界便于父模块按职责组合页面，同时保持实现文件可以独立维护。
pub mod custom_model_modal;
/// `custom_provider` 子模块，承载该区域拆分后的具体界面或辅助逻辑。
///
/// 公开模块边界便于父模块按职责组合页面，同时保持实现文件可以独立维护。
pub mod custom_provider;

#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod connect_tests;
#[cfg(test)]
mod connected_tests;
#[cfg(test)]
mod custom_model_modal_tests;
#[cfg(test)]
mod custom_provider_tests;
#[cfg(test)]
#[path = "system_settings_providers_ui_refresh_08_tests.rs"]
mod ui_refresh_08_tests;
