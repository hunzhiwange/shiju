//! 汇总 App 的分片实现模块。
//! 本模块作为实现层入口，保持主应用类型的行为按领域拆分。

use super::*;

pub(super) use crate::app::components;
pub(super) use crate::app::config;
pub(super) use crate::app::message;
pub(super) use crate::app::models;
pub(super) use crate::app::views;
pub(super) use crate::app::{
    AgentRequest, App, AppTab, CookieConfig, DiffTheme, FocusArea, Message, RecentProjectMeta,
    Screen, SettingsTab, Shell, TerminalState, TerminalTheme, load_recent_projects,
    load_recent_projects_meta,
};

mod agent_stream;
pub(crate) use agent_stream::{desktop_stream_request_id, task_plan_run_active};
mod app_basic;
mod app_init;
mod app_subscription;
mod app_update;
#[cfg(test)]
pub(crate) use app_update::copy_chat_message;
mod app_view;
mod app_view_modals;
mod app_view_status;
pub(crate) use app_view_status::STATUS_BAR_HEIGHT;
#[cfg(test)]
mod tests;
