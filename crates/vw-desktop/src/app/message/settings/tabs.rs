//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::{App, Message, components::system_settings::SystemTab};
use iced::Task;

use super::messages::{
    AcpMessage, AgentsMessage, BrowserMessage, ChannelsMessage, GatewayMessage, HttpRequestMessage,
    McpMessage, MemoryMessage, ModelRoutesMessage, QueryClassificationMessage, RuntimeMessage,
    SettingsMessage, StaffMessage, StaffResourceMessage, WebSearchMessage,
};
use super::models::models_refresh_task;
use super::providers::refresh_task;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::TabSelected(tab) => {
            app.settings_tab = tab;
            app.system_settings_help_tab = None;
            Task::none()
        }
        SettingsMessage::SystemTabSelected(tab) => {
            app.system_settings_tab = tab;
            app.system_settings_help_tab = None;
            if tab == SystemTab::Themes {
                Task::done(Message::Settings(SettingsMessage::Themes(
                    super::messages::ThemesMessage::Refresh,
                )))
            } else if tab == SystemTab::DialogueFlow {
                Task::done(Message::Settings(SettingsMessage::DialogueFlowPermissionRefresh))
            } else if tab == SystemTab::Providers {
                app.provider_settings.loading = true;
                app.provider_settings.connect_error = None;
                app.provider_settings.save_error = None;
                refresh_task()
            } else if tab == SystemTab::Models {
                app.model_settings.loading = true;
                app.model_settings.save_error = None;
                models_refresh_task()
            } else if tab == SystemTab::Skills {
                app.skills_settings.loading = true;
                Task::done(Message::Settings(SettingsMessage::SkillsRefresh))
            } else if tab == SystemTab::EmbeddingRoutes {
                app.embedding_routes_settings.save_error = None;
                app.embedding_routes_settings.save_success = false;
                Task::none()
            } else if tab == SystemTab::ModelRoutes {
                Task::done(Message::Settings(SettingsMessage::ModelRoutes(
                    ModelRoutesMessage::Refresh,
                )))
            } else if tab == SystemTab::QueryClassification {
                Task::done(Message::Settings(SettingsMessage::QueryClassification(
                    QueryClassificationMessage::Refresh,
                )))
            } else if tab == SystemTab::GoalLoop {
                app.goal_loop_settings.save_error = None;
                Task::none()
            } else if tab == SystemTab::Cron {
                app.cron_settings.save_error = None;
                app.agents_settings.loading = true;
                Task::batch(vec![
                    Task::done(Message::Settings(SettingsMessage::CronJobsRefresh)),
                    Task::done(Message::Settings(SettingsMessage::Agents(AgentsMessage::Refresh))),
                ])
            } else if tab == SystemTab::Runtime {
                app.runtime_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::Runtime(RuntimeMessage::Refresh)))
            } else if tab == SystemTab::Agents {
                app.agents_settings.loading = true;
                app.agents_settings.save_error = None;
                app.skills_settings.loading = true;
                Task::batch(vec![
                    Task::done(Message::Settings(SettingsMessage::Agents(AgentsMessage::Refresh))),
                    Task::done(Message::Settings(SettingsMessage::SkillsRefresh)),
                ])
            } else if tab == SystemTab::Staff {
                app.staff_settings.loading = true;
                app.staff_settings.save_error = None;
                app.staff_settings.save_success = false;
                app.agents_settings.loading = true;
                app.skills_settings.loading = true;
                Task::batch(vec![
                    Task::done(Message::Settings(SettingsMessage::Staff(StaffMessage::Refresh))),
                    Task::done(Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
                        StaffResourceMessage::Refresh,
                    )))),
                    Task::done(Message::Settings(SettingsMessage::Agents(AgentsMessage::Refresh))),
                    Task::done(Message::Settings(SettingsMessage::SkillsRefresh)),
                ])
            } else if tab == SystemTab::Browser {
                app.browser_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::Browser(BrowserMessage::Refresh)))
            } else if tab == SystemTab::WebSearch {
                app.web_search_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::Refresh)))
            } else if tab == SystemTab::HttpRequest {
                app.http_request_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::HttpRequest(
                    HttpRequestMessage::Refresh,
                )))
            } else if tab == SystemTab::Memory {
                app.memory_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::Memory(MemoryMessage::Refresh)))
            } else if tab == SystemTab::Channels {
                app.channels_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::Channels(ChannelsMessage::Refresh)))
            } else if tab == SystemTab::Multimodal {
                app.multimodal_settings.save_error = None;
                app.model_settings.loading = true;
                app.model_settings.save_error = None;
                models_refresh_task()
            } else if tab == SystemTab::Acp {
                app.acp_settings.loading = true;
                app.acp_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::Acp(AcpMessage::Refresh)))
            } else if tab == SystemTab::Mcp {
                app.mcp_settings.loading = true;
                app.mcp_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh)))
            } else if tab == SystemTab::Gateway {
                app.gateway_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::Gateway(GatewayMessage::Refresh)))
            } else if tab == SystemTab::Tunnel {
                app.tunnel_settings.save_error = None;
                Task::none()
            } else if tab == SystemTab::Themes {
                app.themes_settings.error = None;
                Task::done(Message::Settings(SettingsMessage::Themes(
                    super::messages::ThemesMessage::Refresh,
                )))
            } else {
                Task::none()
            }
        }
        SettingsMessage::SystemTabQueryChanged(query) => {
            app.system_settings_query = query;
            Task::none()
        }
        SettingsMessage::SystemHelpOpen(tab) => {
            app.system_settings_help_tab = Some(tab);
            Task::none()
        }
        SettingsMessage::SystemHelpClose => {
            app.system_settings_help_tab = None;
            Task::none()
        }
        SettingsMessage::ToggleSettingsSidebar => {
            app.settings_sidebar_collapsed = !app.settings_sidebar_collapsed;
            Task::none()
        }
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
