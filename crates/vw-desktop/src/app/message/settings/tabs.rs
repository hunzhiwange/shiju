//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::components::system_settings_search::SystemSettingsSearchTarget;
use crate::app::{App, Message, components::system_settings::SystemTab};
use iced::Task;

use super::messages::{
    AcpMessage, AgentConfigTabMessage, AgentsMessage, BrowserMessage, ChannelsMessage,
    GatewayMessage, GeneralSettingsMessage, HooksMessage, HttpRequestMessage, McpMessage,
    MemoryMessage, ModelConfigTabMessage, ModelRoutesMessage, RuntimeMessage, SettingsMessage,
    WebSearchMessage,
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
            app.system_settings_tab = match tab {
                SystemTab::Editor => {
                    app.general_settings_tab = crate::app::state::GeneralSettingsTab::Editor;
                    SystemTab::General
                }
                SystemTab::Themes => {
                    app.general_settings_tab = crate::app::state::GeneralSettingsTab::Themes;
                    SystemTab::General
                }
                SystemTab::Channels => {
                    app.agents_settings.active_config_tab =
                        crate::app::state::AgentConfigTab::Channels;
                    SystemTab::Agents
                }
                SystemTab::Runtime => {
                    app.agents_settings.active_config_tab =
                        crate::app::state::AgentConfigTab::Runtime;
                    SystemTab::Agents
                }
                _ => tab,
            };
            app.system_settings_help_tab = None;
            if tab == SystemTab::Themes {
                app.themes_settings.error = None;
                Task::done(Message::Settings(SettingsMessage::Themes(
                    super::messages::ThemesMessage::Refresh,
                )))
            } else if tab == SystemTab::Providers {
                app.provider_settings.loading = true;
                app.provider_settings.connect_error = None;
                app.provider_settings.save_error = None;
                refresh_task()
            } else if tab == SystemTab::Models {
                app.model_settings.active_tab = crate::app::state::ModelConfigTab::Models;
                app.model_settings.loading = true;
                app.model_settings.save_error = None;
                models_refresh_task()
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
            } else if tab == SystemTab::Hooks {
                Task::batch([
                    Task::done(Message::Settings(SettingsMessage::Hooks(
                        HooksMessage::CatalogRefresh,
                    ))),
                    Task::done(Message::Settings(SettingsMessage::Hooks(
                        HooksMessage::AuditRefresh,
                    ))),
                ])
            } else if tab == SystemTab::Agents {
                app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Agents;
                app.agents_settings.loading = true;
                app.agents_settings.save_error = None;
                app.skills_settings.loading = true;
                Task::batch(vec![
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
            } else if tab == SystemTab::Gateway {
                app.gateway_settings.save_error = None;
                Task::done(Message::Settings(SettingsMessage::Gateway(GatewayMessage::Refresh)))
            } else if tab == SystemTab::Tunnel {
                app.tunnel_settings.save_error = None;
                Task::none()
            } else {
                Task::none()
            }
        }
        SettingsMessage::SystemSearchTargetSelected(target) => {
            let parent = match target {
                SystemSettingsSearchTarget::TopLevel(tab)
                | SystemSettingsSearchTarget::Network(tab) => tab,
                SystemSettingsSearchTarget::General(_) => SystemTab::General,
                SystemSettingsSearchTarget::Models(_) => SystemTab::Models,
                SystemSettingsSearchTarget::Cron(_) => SystemTab::Cron,
                SystemSettingsSearchTarget::Agents(_)
                | SystemSettingsSearchTarget::AgentMcp(_)
                | SystemSettingsSearchTarget::AgentSkills(_) => SystemTab::Agents,
                SystemSettingsSearchTarget::Channels(_) => SystemTab::Agents,
                SystemSettingsSearchTarget::Memory(_) => SystemTab::Memory,
                SystemSettingsSearchTarget::Autonomy(_) => SystemTab::Autonomy,
                SystemSettingsSearchTarget::Security(_) => SystemTab::Security,
                SystemSettingsSearchTarget::Gateway(_) => SystemTab::Gateway,
            };
            let mut tasks = vec![update(app, SettingsMessage::SystemTabSelected(parent))];

            let child_task = match target {
                SystemSettingsSearchTarget::TopLevel(_)
                | SystemSettingsSearchTarget::Network(_) => Task::none(),
                SystemSettingsSearchTarget::General(tab) => super::general::update(
                    app,
                    SettingsMessage::General(GeneralSettingsMessage::TabSelected(tab)),
                ),
                SystemSettingsSearchTarget::Models(tab) => update(
                    app,
                    SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(tab)),
                ),
                SystemSettingsSearchTarget::Cron(tab) => {
                    super::cron::update(app, SettingsMessage::CronTabSelected(tab))
                }
                SystemSettingsSearchTarget::Agents(tab) => update(
                    app,
                    SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(tab)),
                ),
                SystemSettingsSearchTarget::Channels(tab) => {
                    tasks.push(update(
                        app,
                        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                            crate::app::state::AgentConfigTab::Channels,
                        )),
                    ));
                    super::channels::update(
                        app,
                        SettingsMessage::Channels(ChannelsMessage::TabSelected(tab)),
                    )
                }
                SystemSettingsSearchTarget::Memory(tab) => super::memory::update(
                    app,
                    SettingsMessage::Memory(MemoryMessage::TabSelected(tab)),
                ),
                SystemSettingsSearchTarget::Autonomy(tab) => {
                    super::autonomy::update(app, SettingsMessage::AutonomyTabSelected(tab))
                }
                SystemSettingsSearchTarget::Security(tab) => {
                    super::security::update(app, SettingsMessage::SecurityTabSelected(tab))
                }
                SystemSettingsSearchTarget::Gateway(tab) => super::gateway::update(
                    app,
                    SettingsMessage::Gateway(GatewayMessage::TabSelected(tab)),
                ),
                SystemSettingsSearchTarget::AgentMcp(tab) => {
                    tasks.push(update(
                        app,
                        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                            crate::app::state::AgentConfigTab::Mcp,
                        )),
                    ));
                    super::mcp::update(app, SettingsMessage::Mcp(McpMessage::TabSelected(tab)))
                }
                SystemSettingsSearchTarget::AgentSkills(tab) => {
                    tasks.push(update(
                        app,
                        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                            crate::app::state::AgentConfigTab::Skills,
                        )),
                    ));
                    super::skills::update(app, SettingsMessage::SkillsTabChanged(tab))
                }
            };
            tasks.push(child_task);
            Task::batch(tasks)
        }
        SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(tab)) => {
            app.model_settings.active_tab = tab;
            app.system_settings_help_tab = None;
            match tab {
                crate::app::state::ModelConfigTab::Models => {
                    app.model_settings.loading = true;
                    app.model_settings.save_error = None;
                    models_refresh_task()
                }
                crate::app::state::ModelConfigTab::ModelRoutes
                | crate::app::state::ModelConfigTab::QueryClassification => Task::done(
                    Message::Settings(SettingsMessage::ModelRoutes(ModelRoutesMessage::Refresh)),
                ),
                crate::app::state::ModelConfigTab::Multimodal => {
                    app.multimodal_settings.save_error = None;
                    app.model_settings.loading = true;
                    app.model_settings.save_error = None;
                    models_refresh_task()
                }
            }
        }
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(tab)) => {
            app.agents_settings.active_config_tab = tab;
            app.system_settings_help_tab = None;
            match tab {
                crate::app::state::AgentConfigTab::Acp => {
                    app.acp_settings.loading = true;
                    app.acp_settings.save_error = None;
                    Task::done(Message::Settings(SettingsMessage::Acp(AcpMessage::Refresh)))
                }
                crate::app::state::AgentConfigTab::Mcp => {
                    app.mcp_settings.loading = true;
                    app.mcp_settings.save_error = None;
                    Task::done(Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh)))
                }
                crate::app::state::AgentConfigTab::Skills => {
                    app.skills_settings.loading = true;
                    app.skills_settings.save_error = None;
                    Task::done(Message::Settings(SettingsMessage::SkillsRefresh))
                }
                crate::app::state::AgentConfigTab::Plugins => {
                    app.skills_settings.plugin_loading = true;
                    Task::done(Message::Settings(SettingsMessage::PluginsRefresh))
                }
                crate::app::state::AgentConfigTab::Channels => {
                    app.channels_settings.save_error = None;
                    Task::done(Message::Settings(SettingsMessage::Channels(
                        ChannelsMessage::Refresh,
                    )))
                }
                crate::app::state::AgentConfigTab::Runtime => {
                    app.runtime_settings.save_error = None;
                    Task::done(Message::Settings(SettingsMessage::Runtime(RuntimeMessage::Refresh)))
                }
                crate::app::state::AgentConfigTab::Reliability => Task::none(),
                crate::app::state::AgentConfigTab::DialogueFlow => {
                    Task::done(Message::Settings(SettingsMessage::DialogueFlowPermissionRefresh))
                }
                crate::app::state::AgentConfigTab::Heartbeat => {
                    app.heartbeat_settings.save_error = None;
                    Task::none()
                }
                crate::app::state::AgentConfigTab::Agents
                | crate::app::state::AgentConfigTab::QuickMessages => Task::none(),
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
