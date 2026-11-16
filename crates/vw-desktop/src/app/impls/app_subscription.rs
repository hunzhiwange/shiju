//! 定义桌面应用的后台订阅。
//! 本模块集中注册定时器、网关轮询和窗口事件，保持订阅来源可审计。

use iced::Subscription;
use std::sync::Arc;

use super::agent_stream::agent_stream;
use super::message;
use super::{App, Message, Screen};
use crate::app::components::status_animation::ACTIVITY_ANIMATION_INTERVAL_MS;
use crate::apps::workflow::state::WorkflowRunSubscriptionRequest;

#[cfg(not(target_arch = "wasm32"))]
fn terminal_subscription(term: &iced_term::Terminal) -> Subscription<Message> {
    desktop_terminal::local::subscription(term).map(Message::Terminal)
}

fn cron_history_subscription(app: &App) -> Subscription<Message> {
    use crate::app::message::settings::cron_history;
    desktop_settings_cron::update::history::subscription(
        cron_history::visible(app),
        app.cron_settings.history.generation,
    )
    .map(|event| Message::Settings(message::SettingsMessage::CronHistory(event)))
}

fn needs_activity_animation_tick(app: &App, startup_transition_active: bool) -> bool {
    desktop_platform::subscriptions::Activity {
        startup_transition_active,
        update_busy: app.update_panel.show_cli_install_modal
            && app.update_panel.cli_install_modal_use_app_update_action
            && app.desktop_update.busy,
        conversation_requesting: app.navigation_tabs.screen == Screen::Project
            && app.any_session_requesting(),
        explore_animating: app.has_active_explore_summary_animation(),
        companion_visible: app.companion_navigation.task_pet_visible
            && app.task_pet_window_id.is_some(),
        companion_animating: app.pet_state.task_pet_avatar_kind
            != crate::app::state::TaskPetAvatarKind::None
            || app.pet_state.task_pet_expand_target.is_some(),
        diagnostic_active: (app.navigation_panels.show_settings
            || app.navigation_panels.show_system_settings)
            && app.acp_settings.diagnostic_id.is_some(),
    }
    .needs_tick()
}

#[cfg(not(target_arch = "wasm32"))]
type WorkflowRunBoxStream<T> = iced::futures::stream::BoxStream<'static, T>;
#[cfg(target_arch = "wasm32")]
type WorkflowRunBoxStream<T> = iced::futures::stream::LocalBoxStream<'static, T>;

fn map_global_event(
    event: iced::Event,
    status: iced::event::Status,
    id: iced::window::Id,
) -> Option<Message> {
    desktop_navigation::global_events::map_global_event(event, status, id).map(adapt_global_event)
}
fn adapt_global_event(event: desktop_navigation::global_events::Message) -> Message {
    use desktop_navigation::global_events::Message as Event;
    match event {
        Event::HoveredFilePath(path) => Message::View(message::ViewMessage::HoveredFilePath(path)),
        Event::PointerMoved(x, y) => Message::View(message::ViewMessage::PointerMoved(x, y)),
        Event::DroppedFilePath(path) => Message::View(message::ViewMessage::DroppedFilePath(path)),
        Event::HoveredFilesLeft => Message::View(message::ViewMessage::HoveredFilesLeft),
        Event::GlobalMouseReleased => Message::View(message::ViewMessage::GlobalMouseReleased),
        Event::GlobalCursorLeft => Message::View(message::ViewMessage::GlobalCursorLeft),
        Event::WindowResized(id, w, h) => {
            Message::View(message::ViewMessage::WindowResized(id, w, h))
        }
        Event::WindowMoved(id, x, y) => Message::View(message::ViewMessage::WindowMoved(id, x, y)),
        Event::CloseRequested(id) => Message::View(message::ViewMessage::CloseRequested(id)),
        Event::WindowClosed(id) => Message::View(message::ViewMessage::WindowClosed(id)),
        Event::WindowFocused(id) => Message::View(message::ViewMessage::WindowFocused(id)),
        Event::FileTreeDragCancel => Message::Project(message::ProjectMessage::FileTreeDragCancel),
        Event::WindowUnfocused(id) => Message::View(message::ViewMessage::WindowUnfocused(id)),
        Event::GlobalKeyPressed(key, modifiers) => {
            Message::View(message::ViewMessage::GlobalKeyPressed(key, modifiers))
        }
        Event::ContextMenuClosed => {
            Message::TaskBoard(message::TaskBoardMessage::ContextMenuClosed)
        }
        Event::FileTreeRenameCancel => {
            Message::Project(message::ProjectMessage::FileTreeRenameCancel)
        }
        Event::CloseSearch => Message::Editor(message::EditorMessage::CloseSearch),
        #[cfg(not(target_arch = "wasm32"))]
        Event::LspCompletionClosed => {
            Message::Preview(message::PreviewMessage::LspCompletionClosed)
        }
        Event::OpenReplace => Message::Editor(message::EditorMessage::OpenReplace),
        Event::OpenSearch => Message::Editor(message::EditorMessage::OpenSearch),
        Event::TraverseFormFocus(id, backwards) => {
            Message::View(message::ViewMessage::TraverseFormFocus(id, backwards))
        }
        Event::PreviewWindowUnfocused => Message::Preview(message::PreviewMessage::WindowUnfocused),
        #[cfg(not(target_arch = "wasm32"))]
        Event::VoiceChatInterrupt => Message::Voice(message::voice::Event::ChatInterrupt),
        Event::FileTreeFindCaseSensitiveShortcut => {
            Message::Project(message::ProjectMessage::FileTreeFindCaseSensitiveShortcut)
        }
        Event::FileTreeFindWholeWordShortcut => {
            Message::Project(message::ProjectMessage::FileTreeFindWholeWordShortcut)
        }
        Event::FileTreeFindRegexShortcut => {
            Message::Project(message::ProjectMessage::FileTreeFindRegexShortcut)
        }
        Event::Batch(events) => {
            Message::Batch(events.into_iter().map(adapt_global_event).collect())
        }
    }
}

impl App {
    /// 公开函数，执行 subscription 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn subscription(&self) -> Subscription<Message> {
        let agent = self.agent_stream_subscription();

        let workflow_run = workflow_run_subscription_request(self)
            .map(|req| Subscription::run_with(req, workflow_run_stream))
            .unwrap_or_else(Subscription::none);

        let events = Subscription::batch([
            crate::app::components::daily_memory::subscription(&self.apps_catalog.daily_memory),
            {
                #[cfg(not(target_arch = "wasm32"))]
                if self.voice.settings.recording_hotkey.is_some() {
                    crate::app::voice::hotkeys::record_subscription()
                } else {
                    iced::event::listen_with(map_global_event)
                }
                #[cfg(target_arch = "wasm32")]
                iced::event::listen_with(map_global_event)
            },
            crate::app::components::system_settings_apps::drag::subscription(
                &self.apps_catalog.drag,
                self.main_window_id,
            ),
        ]);

        #[cfg(not(target_arch = "wasm32"))]
        let terminal_subs = {
            let mut subs: Vec<Subscription<Message>> = Vec::new();
            for t in &self.terminal.tabs {
                subs.push(terminal_subscription(&t.term));
            }
            Subscription::batch(subs)
        };
        #[cfg(target_arch = "wasm32")]
        let terminal_subs = { Subscription::none() };

        let question_poll = if self.any_session_requesting()
            || self.session_state.question_modal_request_id.is_some()
        {
            iced::time::every(std::time::Duration::from_secs(10))
                .map(|_| Message::Chat(message::ChatMessage::QuestionPollTick))
        } else {
            Subscription::none()
        };

        let permission_poll = if self.any_session_requesting()
            || self.session_state.permission_modal_request_id.is_some()
        {
            iced::time::every(std::time::Duration::from_secs(10))
                .map(|_| Message::Chat(message::ChatMessage::PermissionPollTick))
        } else {
            Subscription::none()
        };

        let todo_poll = if self.current_session_is_requesting() {
            iced::time::every(std::time::Duration::from_secs(10))
                .map(|_| Message::Chat(message::ChatMessage::TodoPollTick))
        } else {
            Subscription::none()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let startup_transition_active =
            self.startup_state == crate::app::state::NativeStartupState::Starting;
        #[cfg(target_arch = "wasm32")]
        let startup_transition_active = false;

        // Thinking headers and session indicators need ticks even while awaiting tokens.
        // Restrict request-driven ticks to the conversation screen; tick messages preserve
        // the expensive session projections in prepare_session_ui_projection.
        let activity_animation_tick =
            if needs_activity_animation_tick(self, startup_transition_active) {
                iced::time::every(std::time::Duration::from_millis(ACTIVITY_ANIMATION_INTERVAL_MS))
                    .map(|_| Message::View(message::ViewMessage::ActivityAnimationTick))
            } else {
                Subscription::none()
            };

        let task_board_ui_tick = if self.navigation_tabs.screen == Screen::TaskBoard {
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::TaskBoard(message::TaskBoardMessage::UiTick))
        } else {
            Subscription::none()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let preview_lsp_tick = if self.preview_state.lsp_disabled {
            Subscription::none()
        } else if self.navigation_tabs.active_preview_path.as_ref().is_some_and(|path| {
            self.navigation_tabs.preview_tabs.iter().any(|tab| tab.path == *path)
        }) {
            iced::time::every(std::time::Duration::from_millis(33)).map(|_| Message::PreviewLspTick)
        } else {
            Subscription::none()
        };
        #[cfg(target_arch = "wasm32")]
        let preview_lsp_tick = Subscription::none();

        let design_generation_stream_tick = Subscription::none();

        let provider_models_sync_tick =
            if self.navigation_panels.show_settings && self.provider_settings.models_syncing {
                iced::time::every(std::time::Duration::from_millis(120))
                    .map(|_| Message::Settings(message::SettingsMessage::ProviderModelsSyncTick))
            } else {
                Subscription::none()
            };

        let web_install_progress_tick = if self.web_install.progress_rx.is_some() {
            iced::time::every(std::time::Duration::from_millis(50)).map(|_| {
                Message::View(message::ViewMessage::WebInstall(
                    crate::app::web_install::Event::ProgressTick,
                ))
            })
        } else {
            Subscription::none()
        };

        let app_update_progress_tick = if self.update_panel.app_update_progress_rx.is_some() {
            iced::time::every(std::time::Duration::from_millis(50))
                .map(|_| Message::View(message::ViewMessage::AppUpdateProgressTick))
        } else {
            Subscription::none()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let work_browser_runtime_tick = if self.navigation_tabs.screen == Screen::Project
            && self.navigation_mode.workspace_mode == crate::app::state::WorkspaceMode::Work
            && self.navigation_workspace.work_resources_visible
            && self.navigation_workspace.work_resource_mode
                == crate::app::state::WorkResourceMode::Browser
        {
            iced::time::every(std::time::Duration::from_millis(100))
                .map(|_| Message::View(message::ViewMessage::WorkBrowserRuntimeTick))
        } else {
            Subscription::none()
        };
        #[cfg(target_arch = "wasm32")]
        let work_browser_runtime_tick = Subscription::none();

        let selected_gateway_is_healthy = self
            .gateway_client_settings
            .servers
            .iter()
            .find(|server| server.id == self.gateway_client_settings.selected_server_id)
            .and_then(crate::app::message::gateway_health::server_health_key)
            .and_then(|key| self.gateway_client_settings.health.get(&key))
            .copied()
            .unwrap_or(false);
        let gateway_health_interval = if selected_gateway_is_healthy {
            std::time::Duration::from_secs(60)
        } else {
            std::time::Duration::from_secs(2)
        };
        let gateway_health_tick =
            iced::time::every(gateway_health_interval).map(|_| Message::GatewayHealthTick);

        let usage_center_tick =
            if self.usage_center.open && self.navigation_tabs.screen == crate::app::Screen::Usage {
                iced::time::every(std::time::Duration::from_secs(60)).map(|_| {
                    Message::View(message::ViewMessage::UsageCenter(
                        crate::app::views::usage::center::Event::Tick,
                    ))
                })
            } else {
                Subscription::none()
            };
        Subscription::batch(vec![
            usage_center_tick,
            cron_history_subscription(self),
            #[cfg(not(target_arch = "wasm32"))]
            iced::time::every(self.voice.hotkeys.interval())
                .map(|_| Message::Voice(message::voice::Event::HotkeyTick)),
            #[cfg(not(target_arch = "wasm32"))]
            if self.voice.chat_active() {
                iced::time::every(std::time::Duration::from_millis(50))
                    .map(|_| Message::Voice(message::voice::Event::ChatTick))
            } else {
                Subscription::none()
            },
            agent,
            #[cfg(not(target_arch = "wasm32"))]
            if self
                .session_state
                .session_runtime_states
                .values()
                .any(|runtime| !runtime.mcp_app_panels.is_empty())
            {
                iced::time::every(std::time::Duration::from_millis(250))
                    .map(|_| Message::Chat(message::ChatMessage::McpAppPanelTick))
            } else {
                Subscription::none()
            },
            #[cfg(not(target_arch = "wasm32"))]
            iced::time::every(crate::app::desktop_update::UPDATE_CHECK_INTERVAL)
                .map(|_| Message::View(message::ViewMessage::AutoCheckAppUpdate)),
            #[cfg(target_os = "macos")]
            crate::app::native_menu::subscription(),
            #[cfg(target_os = "macos")]
            iced::time::every(std::time::Duration::from_millis(200)).map(|_| {
                Message::Settings(message::SettingsMessage::General(
                    message::GeneralSettingsMessage::InstanceTick,
                ))
            }),
            if self.lifecycle.close_to_tray {
                iced::time::every(std::time::Duration::from_millis(200)).map(|_| {
                    Message::Settings(message::SettingsMessage::General(
                        message::GeneralSettingsMessage::TrayTick,
                    ))
                })
            } else {
                Subscription::none()
            },
            events,
            terminal_subs,
            question_poll,
            permission_poll,
            todo_poll,
            activity_animation_tick,
            task_board_ui_tick,
            #[cfg(not(target_arch = "wasm32"))]
            if self.agent_apps_auth.login.as_ref().is_some_and(|login| {
                matches!(login.phase, crate::app::state::agent_apps_auth::LoginPhase::Pending(_))
            }) {
                iced::time::every(std::time::Duration::from_secs(1)).map(|_| {
                    use crate::app::message::agent_apps_auth::{Event, message};
                    message(Event::LoginTick)
                })
            } else {
                Subscription::none()
            },
            preview_lsp_tick,
            design_generation_stream_tick,
            provider_models_sync_tick,
            app_update_progress_tick,
            web_install_progress_tick,
            work_browser_runtime_tick,
            workflow_run,
            gateway_health_tick,
            if self.account.active_account().is_some() {
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::Account(message::AccountMessage::RefreshMonthlyQuota))
            } else {
                Subscription::none()
            },
        ])
    }
}

fn collect_agent_subscription_requests(app: &App) -> Vec<Arc<crate::app::AgentRequest>> {
    desktop_session::session::agent::collect_agent_subscription_requests(
        &app.session_state.active_session_id,
        &app.session_state.session_runtime_states,
    )
}

pub(super) fn workflow_run_subscription_request(
    app: &App,
) -> Option<WorkflowRunSubscriptionRequest> {
    app.workflow_state.saved_app_run_subscription_request()
}

fn workflow_run_stream(req: &WorkflowRunSubscriptionRequest) -> WorkflowRunBoxStream<Message> {
    Box::pin(iced::futures::StreamExt::map(
        desktop_workflow_ui::message::workflow_run_stream(req, || {
            crate::app::config::gateway_client().map(|client| {
                gateway_workflow::GatewayWorkflowClient::new(client.transport().clone())
            })
        }),
        Message::WorkflowTool,
    ))
}

#[cfg(test)]
#[path = "app_subscription_tests.rs"]
mod app_subscription_tests;

impl App {
    /// 聊天流订阅同时供真实应用与显式窗口验收使用。
    pub(crate) fn agent_stream_subscription(&self) -> Subscription<Message> {
        let agent_reqs = collect_agent_subscription_requests(self);

        if agent_reqs.is_empty() {
            Subscription::none()
        } else {
            let subs = agent_reqs
                .into_iter()
                .map(|req| {
                    Subscription::run_with(req, |req: &Arc<crate::app::AgentRequest>| {
                        agent_stream(req)
                    })
                })
                .collect::<Vec<_>>();
            Subscription::batch(subs)
        }
    }
}
