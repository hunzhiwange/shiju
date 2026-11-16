//! 系统设置外壳、分类导航和当前设置页的分派。
//!
//! 分类列表和搜索结果共用跳转目标，具体配置及浮层状态由各设置页维护。

use crate::app::components::system_settings_common::{
    settings_close_button, settings_error_banner,
};
use crate::app::components::system_settings_help;
use crate::app::state::{GeneralSettingsTab, ThemesSettingsMode};
use crate::app::{App, Message, message};
use iced::widget::{column, container, responsive, row};
use iced::{Alignment, Element, Length, Theme};

pub(super) mod general_fields;
mod navigation;

#[cfg(test)]
#[path = "system_settings_archived_chats_tests.rs"]
mod archived_chats_tests;

pub(crate) use desktop_settings_shell::reveal_field;

pub(crate) use navigation::SETTINGS_NAVIGATION_SCROLL;

pub use desktop_settings_shell::{SystemTab, is_network_tab, system_tab_keywords};

#[cfg(test)]
pub(super) fn system_tab_matches_query(tab: SystemTab, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    let needle = query.to_lowercase();
    if tab.to_string().to_lowercase().contains(&needle)
        || system_tab_keywords(tab).iter().any(|text| text.to_lowercase().contains(&needle))
    {
        return true;
    }

    use crate::app::components::system_settings_search::SettingsTabDescriptor;
    fn descriptors_match<T>(descriptors: &[SettingsTabDescriptor<T>], needle: &str) -> bool {
        descriptors.iter().any(|descriptor| {
            descriptor.label.to_lowercase().contains(needle)
                || descriptor.keywords.iter().any(|text| text.to_lowercase().contains(needle))
        })
    }

    match tab {
        SystemTab::General => {
            descriptors_match(super::system_settings_general::GENERAL_TABS, &needle)
        }
        SystemTab::Models => super::system_settings_models::MODEL_TABS.iter().any(|descriptor| descriptor.label.to_lowercase().contains(&needle) || descriptor.keywords.iter().any(|text| text.to_lowercase().contains(&needle))),
        SystemTab::Cron => descriptors_match(super::system_settings_cron::CRON_TABS, &needle),
        SystemTab::Agents => {
            descriptors_match(super::system_settings_agents::AGENT_CONFIG_TABS, &needle)
                || descriptors_match(
                    &super::system_settings_channels::channel_tab_descriptors(),
                    &needle,
                )
                || descriptors_match(super::system_settings_mcp::MCP_TABS, &needle)
                || descriptors_match(super::system_settings_skills::SKILLS_TABS, &needle)
        }
        SystemTab::Channels => {
            descriptors_match(&super::system_settings_channels::channel_tab_descriptors(), &needle)
        }
        SystemTab::Memory => {
            descriptors_match(super::system_settings_memory::MEMORY_TABS, &needle)
                || descriptors_match(super::system_settings_memory::QDRANT_TAB, &needle)
        }
        SystemTab::Autonomy => {
            descriptors_match(super::system_settings_autonomy::AUTONOMY_TABS, &needle)
        }
        SystemTab::Security => {
            descriptors_match(super::system_settings_security::SECURITY_TABS, &needle)
        }
        SystemTab::Gateway => {
            descriptors_match(super::system_settings_gateway::GATEWAY_TABS, &needle)
        }
        SystemTab::WebSearch => {
            descriptors_match(super::system_settings_network::NETWORK_TABS, &needle)
        }
        SystemTab::HttpRequest | SystemTab::Browser | SystemTab::Proxy | SystemTab::Tunnel => {
            super::system_settings_network::NETWORK_TABS
                .iter()
                .find(|descriptor| descriptor.tab == tab)
                .is_some_and(|descriptor| {
                    descriptors_match(std::slice::from_ref(descriptor), &needle)
                })
        }
        _ => false,
    }
}

pub(super) fn active_tab_help_modal_open(app: &App, active_tab: SystemTab) -> bool {
    let help_tab = active_settings_help_tab(app, active_tab);
    crate::app::components::system_settings_help::help_open_for_tab(
        app.navigation_panels.system_settings_help_tab,
        help_tab,
    ) || match active_tab {
        SystemTab::Agents => match app.agents_settings.active_config_tab {
            crate::app::state::AgentConfigTab::Heartbeat => app.heartbeat_settings.show_help_modal,
            crate::app::state::AgentConfigTab::Skills => app.skills_settings.show_help_modal,
            crate::app::state::AgentConfigTab::Reliability => {
                app.reliability_settings.show_help_modal
            }
            _ => false,
        },
        SystemTab::Cron => {
            app.cron_settings.show_help_modal || app.scheduler_settings.show_help_modal
        }
        SystemTab::Autonomy => app.autonomy_settings.show_help_modal,
        SystemTab::Security => app.security_settings.show_help_modal,
        SystemTab::GatewayClient => app.gateway_client_settings.show_help_modal,
        SystemTab::Gateway => app.gateway_settings.show_help_modal,
        SystemTab::Observability => app.observability_settings.show_help_modal,
        SystemTab::Proxy => app.proxy_settings.show_help_modal,
        _ => false,
    }
}

fn active_settings_help_tab(app: &App, active_tab: SystemTab) -> SystemTab {
    if active_tab != SystemTab::Agents {
        return active_tab;
    }

    match app.agents_settings.active_config_tab {
        crate::app::state::AgentConfigTab::Channels => SystemTab::Channels,
        crate::app::state::AgentConfigTab::Runtime => SystemTab::Runtime,
        _ => active_tab,
    }
}

pub(super) fn active_tab_save_error(app: &App, active_tab: SystemTab) -> Option<&str> {
    match active_tab {
        SystemTab::Providers => app.provider_settings.save_error.as_deref(),
        SystemTab::Models => {
            crate::app::components::system_settings_models::active_tab_save_error(app)
        }
        SystemTab::Cron => {
            app.cron_settings.save_error.as_deref().or(app.scheduler_settings.save_error.as_deref())
        }
        SystemTab::Agents => match app.agents_settings.active_config_tab {
            crate::app::state::AgentConfigTab::Agents => app.agents_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Channels => {
                app.channels_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::Runtime => {
                app.runtime_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::Heartbeat => {
                app.heartbeat_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::Acp => app.acp_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Mcp => app.mcp_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Skills => app.skills_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Plugins => None,
            crate::app::state::AgentConfigTab::Reliability => {
                app.reliability_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::DialogueFlow
            | crate::app::state::AgentConfigTab::QuickMessages => None,
        },
        SystemTab::Channels => app.channels_settings.save_error.as_deref(),
        SystemTab::Memory => match app.memory_settings.active_tab {
            crate::app::state::MemorySettingsTab::Storage => {
                app.storage_settings.save_error.as_deref()
            }
            _ => app.memory_settings.save_error.as_deref(),
        },
        SystemTab::Runtime => app.runtime_settings.save_error.as_deref(),
        SystemTab::Autonomy => app.autonomy_settings.save_error.as_deref(),
        SystemTab::Security => app.security_settings.save_error.as_deref(),
        SystemTab::GatewayClient => app.gateway_client_settings.save_error.as_deref(),
        SystemTab::Gateway => app.gateway_settings.save_error.as_deref(),
        SystemTab::Observability => app.observability_settings.save_error.as_deref(),
        SystemTab::Proxy => app.proxy_settings.save_error.as_deref(),
        SystemTab::Tunnel => app.tunnel_settings.save_error.as_deref(),
        SystemTab::Hooks => app.hooks_settings.save_error.as_deref(),
        SystemTab::WebSearch => app.web_search_settings.save_error.as_deref(),
        SystemTab::HttpRequest => app.http_request_settings.save_error.as_deref(),
        SystemTab::Browser => app.browser_settings.save_error.as_deref(),
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::ComputerControl => None,
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Auth => None,
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Voice => None,
        SystemTab::Apps
        | SystemTab::Editor
        | SystemTab::General
        | SystemTab::Projects
        | SystemTab::ArchivedChats
        | SystemTab::Worktrees
        | SystemTab::Themes => None,
    }
}

pub(super) fn tabbed_page_owns_save_error(active_tab: SystemTab) -> bool {
    matches!(
        active_tab,
        SystemTab::Models
            | SystemTab::Cron
            | SystemTab::Agents
            | SystemTab::Channels
            | SystemTab::Memory
            | SystemTab::Autonomy
            | SystemTab::Security
            | SystemTab::Gateway
            | SystemTab::WebSearch
            | SystemTab::HttpRequest
            | SystemTab::Browser
            | SystemTab::Proxy
            | SystemTab::Tunnel
    )
}

pub(crate) use desktop_settings_shell::SETTINGS_CONTENT_SCROLL;
#[cfg(test)]
use desktop_settings_shell::{SETTINGS_CONTENT_MAX_WIDTH, SETTINGS_SCROLL_GUTTER};

fn settings_content(app: &App, active_tab: SystemTab) -> Element<'_, Message> {
    let content: Element<'_, Message> = match active_tab {
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::ComputerControl => super::system_settings_computer_control::view(app),
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Voice => super::system_settings_voice::view(app),
        SystemTab::Apps => super::system_settings_data::recovery_ui::overlay(app, super::system_settings_data::export_ui::overlay(app, column![
            super::system_settings_apps::view(
                &app.apps_catalog,
                crate::app::i18n::I18n::new(app.desktop_locale),
            ),
            crate::app::components::system_settings_data::migration_preview_ui::view(app),
            crate::app::message::settings::backup_inventory::view(app),
            super::system_settings_data::export_ui::view(app),
            super::system_settings_data::backup_list_ui::view(app),
            super::system_settings_data::recovery_ui::view(app),
        ].spacing(12).into())),
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Auth => super::system_settings_auth::view(app),
        SystemTab::General => crate::app::components::system_settings_general::view(app),
        SystemTab::Themes => crate::app::components::system_settings_themes::view(app),
        SystemTab::Editor => crate::app::components::system_settings_editor::view(app),
        SystemTab::Projects => crate::app::components::system_settings_projects::view(app),
        SystemTab::ArchivedChats => super::system_settings_archived_chats::view(app),
        SystemTab::Worktrees => super::system_settings_worktrees::view(app),
        SystemTab::Providers => {
            use super::system_settings_common::{rounded_action_btn_style, settings_muted_text_style};
            use iced::widget::{button, text};

            column![
                text("应用供应商由应用目录管理；选择目标应用后可查看执行主机上的配置与状态。")
                    .size(13)
                    .style(settings_muted_text_style),
                button(text("管理应用供应商"))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press(Message::Settings(message::SettingsMessage::SystemTabSelected(
                        SystemTab::Apps,
                    ))),
                crate::app::components::system_settings_providers::connected::view(app),
            ]
            .spacing(12)
            .width(Length::Fill)
            .into()
        }
        SystemTab::Models => crate::app::components::system_settings_models::view(app),
        SystemTab::Cron => crate::app::components::system_settings_cron::view(app),
        SystemTab::Agents => crate::app::components::system_settings_agents::view(app),
        SystemTab::Channels => crate::app::components::system_settings_channels::view(app),
        SystemTab::Memory => crate::app::components::system_settings_memory::view(app),
        SystemTab::Runtime => crate::app::components::system_settings_runtime::view(app),
        SystemTab::Autonomy => crate::app::components::system_settings_autonomy::view(app),
        SystemTab::Security => crate::app::components::system_settings_security::view(app),
        SystemTab::GatewayClient => {
            crate::app::components::system_settings_gateway_client::view(app)
        }
        SystemTab::Gateway => crate::app::components::system_settings_gateway::view(app),
        SystemTab::Observability => {
            crate::app::components::system_settings_observability::view(app)
        }
        SystemTab::Hooks => crate::app::components::system_settings_hooks::view(app),
        SystemTab::WebSearch
        | SystemTab::HttpRequest
        | SystemTab::Browser
        | SystemTab::Proxy
        | SystemTab::Tunnel => crate::app::components::system_settings_network::view(app),
    };

    let mut content_column = column![].spacing(12).width(Length::Fill);
    if !tabbed_page_owns_save_error(active_tab)
        && let Some(err) = active_tab_save_error(app, active_tab)
    {
        let localized = match active_tab {
            SystemTab::Providers => app.provider_settings.save_error.as_ref(),
            SystemTab::GatewayClient => app.gateway_client_settings.save_error.as_ref(),
            SystemTab::Gateway => app.gateway_settings.save_error.as_ref(),
            _ => None,
        }
        .map(|error| error.display(app.desktop_locale))
        .unwrap_or_else(|| {
            if active_tab == SystemTab::Runtime
                || (active_tab == SystemTab::Agents
                    && app.agents_settings.active_config_tab
                        == crate::app::state::AgentConfigTab::Runtime)
            {
                crate::app::i18n::settings_runtime::validation_error(app.desktop_locale, err)
            } else {
                err.to_owned()
            }
        });
        content_column = content_column.push(settings_error_banner(localized));
    }
    let content_column = content_column.push(content);
    let owns_scroll = matches!(active_tab, SystemTab::Agents | SystemTab::ArchivedChats)
        || (active_tab == SystemTab::Cron
            && matches!(
                app.cron_settings.active_tab,
                crate::app::state::CronSettingsTab::Jobs
                    | crate::app::state::CronSettingsTab::History
            ));
    #[cfg(not(target_arch = "wasm32"))]
    let owns_scroll = owns_scroll
        || matches!(active_tab, SystemTab::Auth | SystemTab::ComputerControl);
    let owns_scroll = owns_scroll || active_tab_help_modal_open(app, active_tab);
    desktop_settings_shell::content_page(content_column, active_tab == SystemTab::Cron, owns_scroll)
}

fn settings_shell(
    app: &App,
    width: f32,
    active_tab: SystemTab,
    help_tab: SystemTab,
) -> Element<'_, Message> {
    if app.mobile_layout.mode() == crate::app::state::LayoutMode::Mobile {
        use super::system_settings_common::{rounded_action_btn_style, settings_muted_text_style};
        use iced::widget::{button, text};
        let back = button(text("返回分类"))
            .padding([10, 12])
            .style(rounded_action_btn_style)
            .on_press(Message::View(message::ViewMessage::MobileBack));
        let mut header = row![back].spacing(8).align_y(Alignment::Center);
        if let Some(help) = system_settings_help::help_button(app.desktop_locale, help_tab) {
            header = header.push(help);
        }
        let mut page = column![header, settings_content(app, active_tab)]
            .spacing(8).width(Length::Fill).height(Length::Fill);
        if active_tab == SystemTab::GatewayClient {
            // Gateway edits already validate and persist through the existing settings chain.
            // Keep retry reachable even when the form is scrolled or the keyboard is open.
            page = page.push(column![
                text("有效输入自动保存；保存不代表已连接。连接失败时检查地址、认证及浏览器跨域配置。")
                    .size(12).style(settings_muted_text_style),
                button(text("刷新连接 / 重试"))
                    .padding([10, 12]).style(rounded_action_btn_style)
                    .on_press(Message::GatewayHealthTick),
            ].spacing(6));
        }
        return page.into();
    }
    let close_message = Message::View(message::ViewMessage::ToggleSystemSettings);
    let mut actions = row![super::system_settings_device_sync::entry_points()]
        .spacing(6)
        .align_y(Alignment::Center);
    if let Some(help) = system_settings_help::help_button(app.desktop_locale, help_tab) {
        actions = actions.push(help);
    }
    let actions = container(actions.push(settings_close_button(close_message)))
        .align_right(Length::Fill)
        .padding([8, 12]);

    desktop_settings_shell::shell(width, actions.into(), settings_content(app, active_tab), |sidebar_width| navigation::view(app, sidebar_width))
}

/// 渲染全屏设置、分类导航及当前分类的浮层。
pub fn view(app: &App) -> Element<'_, Message> {
    if !app.navigation_panels.show_system_settings {
        return container(column![]).into();
    }

    let active_tab = app.navigation_panels.system_settings_tab;
    let help_tab = active_settings_help_tab(app, active_tab);
    let shell = responsive(move |size| settings_shell(app, size.width, active_tab, help_tab));
    let modal_content =
        container(shell).width(Length::Fill).height(Length::Fill).style(|theme: &Theme| {
            container::Style {
                text_color: Some(theme.palette().text),
                background: Some(theme.palette().background.into()),
                ..Default::default()
            }
        });

    let themes_active = active_tab == SystemTab::Themes
        || (active_tab == SystemTab::General
            && app.general_settings_tab == GeneralSettingsTab::Themes);
    let editing_theme = themes_active && app.themes_settings.mode == ThemesSettingsMode::Edit;
    let select_focus_available = app.navigation_panels.system_settings_help_tab.is_none()
        && (active_tab == SystemTab::Editor
            || (active_tab == SystemTab::Autonomy && !app.autonomy_settings.show_help_modal)
            || (active_tab == SystemTab::Security && !app.security_settings.show_help_modal)
            || (active_tab == SystemTab::Observability
                && !app.observability_settings.show_help_modal)
            || (active_tab == SystemTab::General
                && matches!(
                    app.general_settings_tab,
                    GeneralSettingsTab::Interface
                        | GeneralSettingsTab::Terminal
                        | GeneralSettingsTab::Editor
                )));

    let mut dialog: Element<'_, Message> = if select_focus_available {
        modal_content.id(SELECT_FOCUS_SCOPE).into()
    } else {
        modal_content.into()
    };

    #[cfg(not(target_arch = "wasm32"))]
    if active_tab == SystemTab::ComputerControl {
        dialog = super::system_settings_computer_control::view_overlays(app, dialog);
    }

    if active_tab == SystemTab::Worktrees {
        dialog = super::system_settings_worktrees::view_overlays(app, dialog);
    }

    if editing_theme {
        dialog = super::system_settings_themes::editor_overlay(app, dialog);
        dialog = container(dialog).id(SELECT_FOCUS_SCOPE).into();
    }

    // 为 Providers 标签页添加额外的浮层视图
    // 这些浮层用于显示自定义提供商、目录、连接等子对话框
    if app.navigation_panels.system_settings_tab == SystemTab::Providers {
        dialog = crate::app::components::system_settings_providers::custom_provider::view_overlays(
            app, dialog,
        );
        dialog =
            crate::app::components::system_settings_providers::catalog::view_overlays(app, dialog);
        dialog =
            crate::app::components::system_settings_providers::connect::view_overlays(app, dialog);
        dialog =
            crate::app::components::system_settings_providers::custom_model_modal::view_overlays(
                app, dialog,
            );
    }

    // 为 Models 标签页添加额外的浮层视图
    if app.navigation_panels.system_settings_tab == SystemTab::Models {
        dialog = crate::app::components::system_settings_models::view_overlays(app, dialog);
    }

    if app.navigation_panels.system_settings_tab == SystemTab::Agents {
        dialog = match app.agents_settings.active_config_tab {
            crate::app::state::AgentConfigTab::Agents => {
                super::system_settings_agents::view_overlays(app, dialog)
            }
            crate::app::state::AgentConfigTab::Heartbeat => {
                crate::app::components::system_settings_heartbeat::view_overlays(app, dialog)
            }
            crate::app::state::AgentConfigTab::Mcp => {
                crate::app::components::system_settings_mcp::view_overlays(app, dialog)
            }
            crate::app::state::AgentConfigTab::Skills => {
                crate::app::components::system_settings_skills::view_overlays(app, dialog)
            }
            crate::app::state::AgentConfigTab::Reliability => {
                crate::app::components::system_settings_reliability::view_overlays(app, dialog)
            }
            _ => dialog,
        };
    }

    if app.navigation_panels.system_settings_tab == SystemTab::Cron {
        dialog = crate::app::components::system_settings_cron::view_overlays(app, dialog);
    }

    dialog = super::system_settings_channels::view_overlays(app, dialog);

    if app.navigation_panels.system_settings_tab == SystemTab::Autonomy {
        dialog = crate::app::components::system_settings_autonomy::view_overlays(app, dialog);
    }

    if app.navigation_panels.system_settings_tab == SystemTab::Security {
        dialog = crate::app::components::system_settings_security::view_overlays(app, dialog);
    }

    #[cfg(not(target_arch = "wasm32"))]
    if active_tab == SystemTab::Auth {
        dialog = super::system_settings_auth::overlay(&app.agent_apps_auth, dialog);
    }

    if app.navigation_panels.system_settings_tab == SystemTab::Apps {
        dialog = super::system_settings_apps::view_overlays(
            &app.apps_catalog,
            crate::app::i18n::I18n::new(app.desktop_locale),
            &app.app_theme,
            dialog,
        );
        let provider_help = super::app_providers::list::Help::new(
            crate::app::i18n::I18n::new(app.desktop_locale),
            &app.app_theme,
        );
        #[cfg(not(target_arch = "wasm32"))]
        let provider_help = provider_help.with_usage(app);
        dialog = super::app_providers::list::overlay(&app.app_providers, provider_help, dialog);
        if crate::app::message::agent_apps_high_availability::can_show(app) {
            dialog = super::agent_apps_high_availability::queue::overlay(&app.app_providers.failover, dialog);
        }
        if crate::app::message::agent_apps_diagnostics::model::can_show(app) {
            dialog = super::agent_apps_diagnostics::model_panel::overlay(
                &app.apps_catalog.model_diagnostics,
                dialog,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            dialog = super::apps_usage_settings::overlay(app, dialog);
        }
    }
    if app.navigation_panels.system_settings_tab == SystemTab::GatewayClient {
        dialog = crate::app::components::system_settings_gateway_client::view_overlays(app, dialog);
    }

    if app.navigation_panels.system_settings_tab == SystemTab::Gateway {
        dialog = crate::app::components::system_settings_gateway::view_overlays(app, dialog);
    }

    if app.navigation_panels.system_settings_tab == SystemTab::Observability {
        dialog = crate::app::components::system_settings_observability::view_overlays(app, dialog);
    }

    if app.navigation_panels.system_settings_tab == SystemTab::Proxy {
        dialog = crate::app::components::system_settings_proxy::view_overlays(app, dialog);
    }

    #[cfg(not(target_arch = "wasm32"))]
    if active_tab == SystemTab::Voice {
        dialog = super::system_settings_voice::overlay(app, dialog);
    }

    dialog = crate::app::components::system_settings_help::with_help_modal(
        app,
        dialog,
        help_tab,
        app.navigation_panels.system_settings_help_tab,
    );

    let dialog = crate::app::message::app_providers::leave::overlay(app, dialog);
    #[cfg(target_os = "macos")]
    let dialog = crate::app::provider_link_preview::overlay(dialog);
    #[cfg(target_os = "macos")]
    let dialog = super::system_settings_data::backup_list_ui::overlay(app, dialog);
    let dialog = crate::app::mcp_link_preview::overlay(app, dialog);
    #[cfg(target_os = "macos")]
    let dialog = crate::app::skill_link_preview::overlay(app, dialog);
    #[cfg(target_os = "macos")]
    let dialog = crate::app::prompt_link_preview::overlay(app, dialog);
    super::system_settings_device_sync::overlay(app, dialog)
}

pub(super) use desktop_settings_shell::SELECT_FOCUS_SCOPE;

pub(crate) use desktop_settings_shell::traverse_select_focus;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "system_settings_ui_refresh_04_tests.rs"]
mod ui_refresh_04_tests;
