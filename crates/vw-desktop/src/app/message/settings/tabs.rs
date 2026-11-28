//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::components::system_settings::{
    SETTINGS_CONTENT_SCROLL, SETTINGS_NAVIGATION_SCROLL, SystemTab, reveal_field,
};
use crate::app::components::system_settings_search::SystemSettingsSearchTarget;
use crate::app::state::AgentConfigTab;
use crate::app::{App, AppTab, Message, Screen};
use desktop_ui_controls::settings_navigation::AgentManagementPage;
use iced::Task;

use super::messages::{
    AcpMessage, AgentConfigTabMessage, AgentsMessage, BrowserMessage, ChannelsMessage,
    GatewayMessage, GeneralSettingsMessage, HooksMessage, HttpRequestMessage, McpMessage,
    MemoryMessage, ModelConfigTabMessage, ModelRoutesMessage, RuntimeMessage, SettingsMessage,
    WebSearchMessage,
};
use super::models::models_refresh_task;
use super::providers::refresh_task;

pub(crate) fn open_agent_management(
    app: &mut App,
    page: Option<AgentManagementPage>,
) -> Task<Message> {
    if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
        return Task::none();
    }

    const TAB_ID: &str = "agent_management";
    let page = page.unwrap_or(AgentManagementPage::Agents);
    let screen = Screen::AgentManagement(page);
    if let Some(tab) = app.navigation_tabs.open_tabs.iter_mut().find(|tab| tab.id == TAB_ID) {
        tab.screen = screen;
    } else {
        let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
        app.navigation_tabs.open_tabs.push(AppTab {
            id: TAB_ID.into(),
            title: i18n.__("智能体").into(),
            screen,
            project_path: None,
        });
    }

    let was_visible = crate::app::components::system_settings_agents::visible_page(app) == Some(page);
    app.navigation_tabs.active_tab_id = Some(TAB_ID.into());
    app.navigation_tabs.screen = screen;
    app.navigation_panels.show_system_settings = false;
    app.navigation_panels.system_settings_help_tab = None;
    app.navigation_panels.close_menu();
    app.popover_visibility.acp = false;
    app.navigation_mode.code_projects_menu_open = false;
    app.navigation_sidebar.work_sidebar_more_open = false;
    app.workspace_state.project_tools_menu_path = None;
    app.mobile_layout.drawer_open = false;
    app.mobile_layout.mine_open = false;

    if was_visible {
        return Task::none();
    }
    let refresh = match page {
        AgentManagementPage::Agents => {
            app.agents_settings.loading = true;
            app.agents_settings.save_error = None;
            SettingsMessage::Agents(AgentsMessage::Refresh)
        }
        AgentManagementPage::Acp => {
            app.acp_settings.loading = true;
            app.acp_settings.save_error = None;
            SettingsMessage::Acp(AcpMessage::Refresh)
        }
        AgentManagementPage::Mcp => {
            app.mcp_settings.loading = true;
            app.mcp_settings.save_error = None;
            SettingsMessage::Mcp(McpMessage::Refresh)
        }
        AgentManagementPage::Plugins => {
            app.skills_settings.plugin_loading = true;
            SettingsMessage::PluginsRefresh
        }
        AgentManagementPage::Channels => {
            app.channels_settings.save_error = None;
            SettingsMessage::Channels(ChannelsMessage::Refresh)
        }
    };
    Task::done(Message::Settings(refresh))
}

pub(crate) fn management_page(tab: AgentConfigTab) -> Option<AgentManagementPage> {
    Some(match tab {
        AgentConfigTab::Agents => AgentManagementPage::Agents,
        AgentConfigTab::Acp => AgentManagementPage::Acp,
        AgentConfigTab::Mcp => AgentManagementPage::Mcp,
        AgentConfigTab::Plugins => AgentManagementPage::Plugins,
        AgentConfigTab::Channels => AgentManagementPage::Channels,
        _ => return None,
    })
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::TabSelected(tab) => {
            crate::app::components::third_party_settings::invalidate(app);
            app.apps_catalog.external_sessions.cancel_reader();
            app.apps_catalog.daily_memory.invalidate();
            app.apps_catalog.hermes_memory.invalidate();
            app.apps_catalog.project_profiles.invalidate();
            if app.navigation_panels.system_settings_tab == SystemTab::Worktrees {
                super::worktrees::recovery::invalidate(app);
            }
            app.apps_catalog.drag = Default::default();
            app.navigation_panels.settings_tab = tab;
            app.navigation_panels.system_settings_help_tab = None;
            Task::none()
        }
        SettingsMessage::SystemTabSelected(SystemTab::Apps) => {
            crate::app::message::view::update(app, crate::app::message::ViewMessage::OpenThirdPartyAgents)
        }
        SettingsMessage::SystemTabSelected(SystemTab::Channels) => {
            open_agent_management(app, Some(AgentManagementPage::Channels))
        }
        SettingsMessage::SystemTabSelected(tab) => {
            crate::app::components::third_party_settings::invalidate(app);
            #[cfg(not(target_arch = "wasm32"))]
            if tab == SystemTab::Auth {
                let open = crate::app::message::view::update(app, crate::app::message::ViewMessage::OpenThirdPartyAgents);
                return Task::batch([open, crate::app::components::third_party_settings::navigate(app,
                    desktop_ui_controls::settings_navigation::ThirdPartyPage::Authorization)]);
            }
            app.apps_catalog.external_sessions.cancel_reader();
            app.apps_catalog.daily_memory.invalidate();
            app.apps_catalog.hermes_memory.invalidate();
            app.apps_catalog.project_profiles.invalidate();
            if app.navigation_panels.system_settings_tab == SystemTab::Worktrees {
                super::worktrees::recovery::invalidate(app);
            }
            app.apps_catalog.drag = Default::default();
            app.navigation_panels.system_settings_tab = match tab {
                SystemTab::Editor => {
                    app.general_settings.general_settings_tab = crate::app::state::GeneralSettingsTab::Editor;
                    SystemTab::General
                }
                SystemTab::Themes => {
                    app.general_settings.general_settings_tab = crate::app::state::GeneralSettingsTab::Themes;
                    SystemTab::General
                }
                SystemTab::Runtime => {
                    app.agents_settings.active_config_tab =
                        crate::app::state::AgentConfigTab::Runtime;
                    SystemTab::Agents
                }
                _ => tab,
            };
            app.navigation_panels.system_settings_help_tab = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.agent_apps_auth.form = None;
                if tab == SystemTab::ComputerControl {
                    return super::computer_control::update(app, super::computer_control::Event::Refresh);
                }
                if tab == SystemTab::Auth {
                    return super::super::agent_apps_auth::update(
                        app,
                        super::super::agent_apps_auth::Event::Refresh,
                    );
                }
            }
            if matches!(tab, SystemTab::TaskWorkflow | SystemTab::TaskLabels
                | SystemTab::TaskProperties | SystemTab::TaskQuickActions)
            {
                crate::app::components::system_settings::task_settings::load(app, tab)
            } else if tab == SystemTab::Worktrees {
                super::worktrees::update(app, super::worktrees::WorktreeSettingsMessage::Refresh)
            } else if tab == SystemTab::ArchivedChats {
                super::archived_chats::update(
                    app,
                    super::archived_chats::ArchivedChatsMessage::Refresh,
                )
            } else if tab == SystemTab::Apps {
                super::apps_catalog::update(app, super::AppsCatalogMessage::EnsureLoaded)
            } else if tab == SystemTab::Themes {
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
                update(
                    app,
                    SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                        AgentConfigTab::DialogueFlow,
                    )),
                )
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
            if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            if let SystemSettingsSearchTarget::Agents(tab) = target
                && let Some(page) = management_page(tab)
            {
                return open_agent_management(app, Some(page));
            }
            match target {
                SystemSettingsSearchTarget::Channels(tab) => {
                    let open = open_agent_management(app, Some(AgentManagementPage::Channels));
                    let select = super::channels::update(
                        app,
                        SettingsMessage::Channels(ChannelsMessage::TabSelected(tab)),
                    );
                    return Task::batch([open, select]);
                }
                SystemSettingsSearchTarget::AgentMcp(tab) => {
                    let open = open_agent_management(app, Some(AgentManagementPage::Mcp));
                    let select = super::mcp::update(
                        app,
                        SettingsMessage::Mcp(McpMessage::TabSelected(tab)),
                    );
                    return Task::batch([open, select]);
                }
                _ => {}
            }
            let mut tasks = vec![update(app, SettingsMessage::SystemTabSelected(target.page()))];

            let child_task = match target {
                SystemSettingsSearchTarget::TopLevel(_)
                | SystemSettingsSearchTarget::Network(_) => Task::none(),
                SystemSettingsSearchTarget::ThirdParty(page) => crate::app::components::third_party_settings::navigate(app, page),
                SystemSettingsSearchTarget::General(tab) => super::general::update(
                    app,
                    SettingsMessage::General(GeneralSettingsMessage::TabSelected(tab)),
                ),
                SystemSettingsSearchTarget::GeneralField(field) => super::general::update(
                    app,
                    SettingsMessage::General(GeneralSettingsMessage::TabSelected(field.tab)),
                ),
                SystemSettingsSearchTarget::Models(tab) => update(
                    app,
                    SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(tab)),
                ),
                SystemSettingsSearchTarget::Agents(tab) => update(
                    app,
                    SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(tab)),
                ),
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
                SystemSettingsSearchTarget::Channels(_) | SystemSettingsSearchTarget::AgentMcp(_) => {
                    unreachable!()
                }
            };
            tasks.push(child_task);
            let scroll = match target {
                SystemSettingsSearchTarget::GeneralField(field) => reveal_field(field.id),
                _ => iced::widget::operation::snap_to(
                    SETTINGS_CONTENT_SCROLL,
                    iced::widget::scrollable::RelativeOffset::START,
                ),
            };
            tasks.push(scroll);
            Task::batch(tasks)
        }
        SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(tab)) => {
            app.model_settings.active_tab = tab;
            app.navigation_panels.system_settings_help_tab = None;
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
                    #[cfg(not(target_arch = "wasm32"))]
                    return Task::batch([models_refresh_task(), super::multimodal::refresh_media_catalog(app)]);
                    #[cfg(target_arch = "wasm32")]
                    models_refresh_task()
                }
            }
        }
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(tab)) => {
            if let Some(page) = management_page(tab) {
                return open_agent_management(app, Some(page));
            }
            app.agents_settings.active_config_tab = tab;
            app.navigation_panels.system_settings_help_tab = None;
            match tab {
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
                AgentConfigTab::QuickMessages => Task::none(),
                _ => unreachable!(),
            }
        }
        SettingsMessage::SystemTabQueryChanged(query) => {
            app.general_settings.system_settings_query = query;
            iced::widget::operation::snap_to(
                SETTINGS_NAVIGATION_SCROLL,
                iced::widget::scrollable::RelativeOffset::START,
            )
        }
        SettingsMessage::SystemHelpOpen(tab) => {
            app.navigation_panels.system_settings_help_tab = Some(tab);
            Task::none()
        }
        SettingsMessage::SystemHelpClose => {
            app.navigation_panels.system_settings_help_tab = None;
            Task::none()
        }
        SettingsMessage::ToggleSettingsSidebar => {
            app.general_settings.settings_sidebar_collapsed = !app.general_settings.settings_sidebar_collapsed;
            Task::none()
        }
        _ => Task::none(),
    }
}
