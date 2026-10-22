//! 定义桌面应用的后台订阅。
//! 本模块集中注册定时器、网关轮询和窗口事件，保持订阅来源可审计。

use iced::Subscription;
use std::sync::Arc;

use super::agent_stream::agent_stream;
use super::message;
use super::{App, Message, Screen};
use crate::app::components::status_animation::ACTIVITY_ANIMATION_INTERVAL_MS;
use crate::apps::workflow::state::WorkflowRunSubscriptionRequest;

fn needs_activity_animation_tick(app: &App, startup_transition_active: bool) -> bool {
    startup_transition_active
        || app.has_active_explore_summary_animation()
        || app.task_pet_window_id.is_some()
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
    match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::View(message::ViewMessage::PointerMoved(position.x, position.y)))
        }
        iced::Event::Window(iced::window::Event::FileHovered(path)) => Some(Message::View(
            message::ViewMessage::HoveredFilePath(path.to_string_lossy().to_string()),
        )),
        iced::Event::Window(iced::window::Event::FilesHoveredLeft) => {
            Some(Message::View(message::ViewMessage::HoveredFilesLeft))
        }
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
            Some(Message::View(message::ViewMessage::GlobalMouseReleased))
        }
        iced::Event::Mouse(iced::mouse::Event::CursorLeft) => {
            Some(Message::View(message::ViewMessage::GlobalCursorLeft))
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::View(message::ViewMessage::WindowResized(id, size.width, size.height)))
        }
        iced::Event::Window(iced::window::Event::Moved(pos)) => {
            Some(Message::View(message::ViewMessage::WindowMoved(id, pos.x, pos.y)))
        }
        iced::Event::Window(iced::window::Event::CloseRequested) => {
            Some(Message::View(message::ViewMessage::CloseRequested(id)))
        }
        iced::Event::Window(iced::window::Event::Closed) => {
            Some(Message::View(message::ViewMessage::WindowClosed(id)))
        }
        iced::Event::Window(iced::window::Event::Focused) => {
            Some(Message::View(message::ViewMessage::WindowFocused(id)))
        }
        iced::Event::Window(iced::window::Event::Unfocused) => Some(Message::Batch(vec![
            Message::View(message::ViewMessage::WindowUnfocused(id)),
            Message::Preview(message::PreviewMessage::WindowUnfocused),
        ])),
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)) {
                let escape_key_message =
                    Message::View(message::ViewMessage::GlobalKeyPressed(key.clone(), modifiers));
                #[cfg(not(target_arch = "wasm32"))]
                let mut messages = vec![
                    Message::TaskBoard(message::TaskBoardMessage::ContextMenuClosed),
                    Message::Editor(message::EditorMessage::CloseSearch),
                    escape_key_message,
                ];
                #[cfg(target_arch = "wasm32")]
                let messages = vec![
                    Message::TaskBoard(message::TaskBoardMessage::ContextMenuClosed),
                    Message::Editor(message::EditorMessage::CloseSearch),
                    escape_key_message,
                ];
                #[cfg(not(target_arch = "wasm32"))]
                messages.push(Message::Preview(message::PreviewMessage::LspCompletionClosed));
                return Some(Message::Batch(messages));
            }

            if modifiers.command() && modifiers.alt() && !modifiers.shift() {
                let shortcut = match &key {
                    iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("c") => {
                        Some(message::ProjectMessage::FileTreeFindCaseSensitiveShortcut)
                    }
                    iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("w") => {
                        Some(message::ProjectMessage::FileTreeFindWholeWordShortcut)
                    }
                    iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("x") => {
                        Some(message::ProjectMessage::FileTreeFindRegexShortcut)
                    }
                    _ => None,
                };
                if let Some(shortcut) = shortcut {
                    return Some(Message::Project(shortcut));
                }
            }

            if modifiers.command()
                && matches!(key, iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("f"))
            {
                return Some(if modifiers.alt() {
                    Message::Editor(message::EditorMessage::OpenReplace)
                } else {
                    Message::Editor(message::EditorMessage::OpenSearch)
                });
            }

            if status == iced::event::Status::Captured {
                return None;
            }
            Some(Message::View(message::ViewMessage::GlobalKeyPressed(key, modifiers)))
        }
        _ => None,
    }
}

impl App {
    /// 公开函数，执行 subscription 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn subscription(&self) -> Subscription<Message> {
        let agent_reqs = collect_agent_subscription_requests(self);

        let agent = if agent_reqs.is_empty() {
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
        };

        let workflow_run = workflow_run_subscription_request(self)
            .map(|req| Subscription::run_with(req, workflow_run_stream))
            .unwrap_or_else(Subscription::none);

        let events = iced::event::listen_with(map_global_event);

        #[cfg(not(target_arch = "wasm32"))]
        let terminal_subs = {
            let mut subs: Vec<Subscription<Message>> = Vec::new();
            for t in &self.terminal.tabs {
                subs.push(
                    t.term
                        .subscription()
                        .map(|e| Message::Terminal(message::TerminalMessage::Event(e))),
                );
            }
            Subscription::batch(subs)
        };
        #[cfg(target_arch = "wasm32")]
        let terminal_subs = { Subscription::none() };

        let question_poll =
            if self.any_session_requesting() || self.question_modal_request_id.is_some() {
                iced::time::every(std::time::Duration::from_secs(10))
                    .map(|_| Message::Chat(message::ChatMessage::QuestionPollTick))
            } else {
                Subscription::none()
            };

        let permission_poll =
            if self.any_session_requesting() || self.permission_modal_request_id.is_some() {
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

        // Session streams already rebuild the UI when visible state changes. Driving the global
        // application tree from an additional 90 ms timer makes an otherwise idle long chat
        // continuously repeat layout and text shaping, including for background sessions.
        let activity_animation_tick =
            if needs_activity_animation_tick(self, startup_transition_active) {
                iced::time::every(std::time::Duration::from_millis(
                    ACTIVITY_ANIMATION_INTERVAL_MS,
                ))
                .map(|_| Message::View(message::ViewMessage::ActivityAnimationTick))
            } else {
                Subscription::none()
            };

        let task_board_ui_tick = if self.screen == Screen::TaskBoard {
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::TaskBoard(message::TaskBoardMessage::UiTick))
        } else {
            Subscription::none()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let preview_lsp_tick = if self.lsp_disabled {
            Subscription::none()
        } else if self
            .active_preview_path
            .as_ref()
            .is_some_and(|path| self.preview_tabs.iter().any(|tab| tab.path == *path))
        {
            iced::time::every(std::time::Duration::from_millis(33)).map(|_| Message::PreviewLspTick)
        } else {
            Subscription::none()
        };
        #[cfg(target_arch = "wasm32")]
        let preview_lsp_tick = Subscription::none();

        let design_generation_stream_tick = Subscription::none();

        let provider_models_sync_tick =
            if self.show_settings && self.provider_settings.models_syncing {
                iced::time::every(std::time::Duration::from_millis(120))
                    .map(|_| Message::Settings(message::SettingsMessage::ProviderModelsSyncTick))
            } else {
                Subscription::none()
            };

        let app_update_progress_tick = if self.app_update_progress_rx.is_some() {
            iced::time::every(std::time::Duration::from_millis(50))
                .map(|_| Message::View(message::ViewMessage::AppUpdateProgressTick))
        } else {
            Subscription::none()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let work_browser_runtime_tick = if self.screen == Screen::Project
            && self.workspace_mode == crate::app::state::WorkspaceMode::Work
            && self.work_resources_visible
            && self.work_resource_mode == crate::app::state::WorkResourceMode::Browser
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

        Subscription::batch(vec![
            agent,
            #[cfg(target_arch = "wasm32")]
            crate::app::wasm_ime::subscription(),
            #[cfg(target_os = "macos")]
            crate::app::native_menu::subscription(),
            events,
            terminal_subs,
            question_poll,
            permission_poll,
            todo_poll,
            activity_animation_tick,
            task_board_ui_tick,
            preview_lsp_tick,
            design_generation_stream_tick,
            provider_models_sync_tick,
            app_update_progress_tick,
            work_browser_runtime_tick,
            workflow_run,
            gateway_health_tick,
        ])
    }
}

fn collect_agent_subscription_requests(app: &App) -> Vec<Arc<crate::app::AgentRequest>> {
    let mut requests = Vec::new();
    if let Some(active_id) = &app.active_session_id
        && let Some(request) = app
            .session_runtime_states
            .get(active_id)
            .and_then(|runtime| runtime.active_agent_request.as_ref().map(Arc::clone))
    {
        requests.push(request);
    }
    for (session_id, runtime) in &app.session_runtime_states {
        if app.active_session_id.as_ref() == Some(session_id) {
            continue;
        }
        if let Some(request) = runtime.active_agent_request.as_ref().map(Arc::clone) {
            requests.push(request);
        }
    }
    requests
}

pub(super) fn workflow_run_subscription_request(
    app: &App,
) -> Option<WorkflowRunSubscriptionRequest> {
    app.workflow_state.saved_app_run_subscription_request()
}

fn workflow_run_stream(req: &WorkflowRunSubscriptionRequest) -> WorkflowRunBoxStream<Message> {
    use iced::futures::SinkExt;
    use vw_gateway_client::{GatewayWorkflowResponseMode, GatewayWorkflowStreamRequest};

    let req = req.clone();
    Box::pin(iced::stream::channel(
        100,
        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            let result = match crate::app::config::gateway_client() {
                Ok(client) => {
                    let body = GatewayWorkflowStreamRequest {
                        query: req.query.clone(),
                        inputs: req.inputs.clone(),
                        response_mode: GatewayWorkflowResponseMode::Streaming,
                        user: req.user.clone(),
                        conversation_id: None,
                    };
                    client
                        .stream_workflow_application(&req.uuid, &body, |event| {
                            output
                                .try_send(Message::WorkflowTool(
                                    crate::apps::workflow::WorkflowMessage::SavedAppRunEvent {
                                        uuid: req.uuid.clone(),
                                        event,
                                    },
                                ))
                                .is_ok()
                        })
                        .await
                }
                Err(error) => Err(error),
            };

            let _ = output
                .send(Message::WorkflowTool(
                    crate::apps::workflow::WorkflowMessage::SavedAppRunFinished {
                        uuid: req.uuid,
                        result,
                    },
                ))
                .await;
        },
    ))
}
#[cfg(test)]
#[path = "app_subscription_tests.rs"]
mod app_subscription_tests;
