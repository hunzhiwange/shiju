//! Cache expensive session projections across geometry-only UI events.
//! Unknown messages invalidate conservatively, including nested batches.
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use super::AgentChildSession;
use crate::app::message::{ChatMessage, ProjectMessage, ViewMessage};
use crate::app::{App, Message};
use iced::Task;

#[derive(Default)]
pub(crate) struct SessionUiProjection {
    pub(crate) hover_details:
        std::collections::HashMap<String, super::hover_details::SessionHoverDetails>,
    pub(crate) hover_details_loading: HashSet<String>,
    pub(crate) agent_sessions: RefCell<Option<Arc<Vec<AgentChildSession>>>>,
    pub(crate) agent_session_key: RefCell<(Option<String>, usize)>,
    pub(crate) sidebar: RefCell<Option<Arc<crate::app::views::project::WorkSidebarModel>>>,
    pub(crate) pinned: HashSet<(String, String)>,
    pending_pins: HashSet<(String, String)>,
    pins_generation: u64,
}

fn preserves_projection(message: &Message) -> bool {
    match message {
        Message::Chat(event) if crate::app::message::chat::input::is_composer_event(event) => true,
        // These handlers update local UI/transport state, not session metadata.
        Message::Terminal(crate::app::message::TerminalMessage::Event(_))
        | Message::View(ViewMessage::WorkBrowserRuntimeTick)
        | Message::GatewayHealthTick => true,
        Message::Chat(
            ChatMessage::SubmitTick
            | ChatMessage::SessionSaveAck
            | ChatMessage::QuestionPollTick
            | ChatMessage::PermissionPollTick
            | ChatMessage::TodoPollTick
            | ChatMessage::LoadInputPanelTodos
            | ChatMessage::QuestionListLoaded(_)
            | ChatMessage::PermissionListLoaded(_)
            | ChatMessage::InputPanelTodosLoaded(..),
        ) => true,
        Message::None
        | Message::CopyDone
        | Message::Batch(_)
        | Message::SessionPinsLoaded(..)
        | Message::SessionDetailsHovered { .. }
        | Message::SessionDetailsLoaded(..) => true,
        // Queued tokens and save acknowledgements do not change either view.
        // Applying a delta invalidates the agent projection at the mutation site;
        // sidebar metadata remains unchanged until a lifecycle/session event.
        Message::Chat(
            ChatMessage::AgentStreamDelta(..)
            | ChatMessage::AgentStreamDeltaFlush(_)
            | ChatMessage::AgentStreamSessionSaveFlush(_)
            | ChatMessage::AgentStreamSessionDeltaSaved { .. },
        ) => true,
        Message::View(
            ViewMessage::PointerMoved(..)
            | ViewMessage::ActivityAnimationTick
            | ViewMessage::GlobalCursorLeft
            | ViewMessage::GlobalMouseReleased
            | ViewMessage::WindowResized(..)
            | ViewMessage::WindowMoved(..)
            | ViewMessage::WindowFocused(_)
            | ViewMessage::WindowUnfocused(_)
            | ViewMessage::FullscreenLayoutSettled
            | ViewMessage::TabHovered(_)
            | ViewMessage::WorkSidebarProjectNavHovered(_)
            | ViewMessage::WorkSidebarFilterToggled
            | ViewMessage::WorkSidebarFilterClosed
            | ViewMessage::WorkSidebarMoreOpened
            | ViewMessage::WorkSidebarMoreClosed,
        ) => true,
        Message::Preview(_) | Message::Editor(_) | Message::ChatMarkdownSelectionChanged { .. } => {
            true
        }
        Message::Chat(
            ChatMessage::ScrollChanged { .. }
            | ChatMessage::VisibleMessageMeasurements(_)
            | ChatMessage::ScrollToBottom
            | ChatMessage::TodoAnimTick
            | ChatMessage::ThinkHover(..)
            | ChatMessage::ThinkHoverLeave
            | ChatMessage::ToolHover(..)
            | ChatMessage::ToolHoverLeave
            | ChatMessage::ToolFileHover(_)
            | ChatMessage::ToolFileHoverLeave
            | ChatMessage::FullscreenOverlayEntered
            | ChatMessage::FullscreenOverlayExited
            | ChatMessage::InputAreaDragHoverChanged(_)
            | ChatMessage::FileReferenceHoverChanged(_)
            | ChatMessage::InputChanged(_)
            | ChatMessage::InputEditorAction(_)
            | ChatMessage::MessageEditorAction(..)
            | ChatMessage::SpecialTextEditorAction(..)
            | ChatMessage::ToolTextEditorAction(..)
            | ChatMessage::MarkdownBlockEditorAction(..)
            | ChatMessage::ThinkEditorAction(..),
        ) => true,
        Message::Project(ProjectMessage::SessionUiPrepared { .. }) => true,
        #[cfg(not(target_arch = "wasm32"))]
        Message::PreviewLspTick => true,
        _ => false,
    }
}

impl App {
    pub(crate) fn set_session_pin(
        &mut self,
        project_id: String,
        id: String,
        pinned: bool,
    ) -> Task<Message> {
        let key = (project_id, id);
        if !self.session_ui_projection.pending_pins.insert(key.clone()) {
            return Task::none();
        }
        let (project_id, id) = key.clone();
        Task::perform(
            async move {
                crate::app::message::spawn_blocking_opt(move || {
                    Some((|| {
                        let data_dir = pins_data_dir()?;
                        let scope = pins_scope(&project_id);
                        vw_shared::session::ui_store::set_thread_pinned(
                            &data_dir, &scope, &id, pinned,
                        )
                        .map_err(|error| error.to_string())?;
                        Ok(pinned)
                    })())
                })
                .await
                .unwrap_or_else(|| Err("pin write task failed".to_owned()))
            },
            move |result| Message::Project(ProjectMessage::SessionPinSaved(key.clone(), result)),
        )
    }

    pub(crate) fn apply_session_pin(
        &mut self,
        key: (String, String),
        result: Result<bool, String>,
    ) -> Task<Message> {
        self.session_ui_projection.pending_pins.remove(&key);
        match result {
            Ok(pinned) => {
                if pinned {
                    self.session_ui_projection.pinned.insert(key);
                } else {
                    self.session_ui_projection.pinned.remove(&key);
                }
                self.session_ui_projection.sidebar.get_mut().take();
                self.refresh_session_pins_task()
            }
            Err(error) => self.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "保存会话置顶状态失败：%s",
                vec![error.into()],
            ),
        }
    }

    pub(crate) fn prepare_session_ui_projection(&mut self, message: &Message) -> bool {
        if !preserves_projection(message) {
            self.session_ui_projection.agent_sessions.get_mut().take();
            if !matches!(
                message,
                Message::Chat(
                    ChatMessage::AgentToolLifecycle(..)
                        | ChatMessage::AgentWorkflowNodeUpdate(..)
                        | ChatMessage::AgentCompactionStatus(..)
                        | ChatMessage::AgentContextLoaded(..)
                )
            ) {
                if self.session_ui_projection.sidebar.get_mut().take().is_some() {
                    // Only enum tags: never log keystrokes, prompts, or tool payloads.
                    match message {
                        Message::Chat(event) => {
                            tracing::debug!(target: "vw_desktop::session_ui", family = "chat", event = ?std::mem::discriminant(event), "sidebar invalidated")
                        }
                        Message::View(event) => {
                            tracing::debug!(target: "vw_desktop::session_ui", family = "view", event = ?std::mem::discriminant(event), "sidebar invalidated")
                        }
                        Message::Project(event) => {
                            tracing::debug!(target: "vw_desktop::session_ui", family = "project", event = ?std::mem::discriminant(event), "sidebar invalidated")
                        }
                        _ => {
                            tracing::debug!(target: "vw_desktop::session_ui", family = "app", event = ?std::mem::discriminant(message), "sidebar invalidated")
                        }
                    }
                }
            }
        }
        matches!(
            message,
            Message::Project(
                ProjectMessage::SessionsLoaded(_)
                    | ProjectMessage::SessionBootstrapLoaded { .. }
                    | ProjectMessage::ProjectSessionsLoaded(..)
                    | ProjectMessage::RecentProjectsLoaded(_)
            ) | Message::GatewayHealthTick
                | Message::Chat(
                    ChatMessage::AgentPostToolRound(..) | ChatMessage::AgentStreamDone(..)
                )
        )
    }

    pub(crate) fn refresh_session_pins_task(&mut self) -> Task<Message> {
        self.session_ui_projection.pins_generation =
            self.session_ui_projection.pins_generation.wrapping_add(1);
        let generation = self.session_ui_projection.pins_generation;
        let projects: HashSet<String> = self
            .sessions
            .iter()
            .chain(self.project_sessions.values().flatten())
            .map(|session| session.project_id.clone())
            .collect();
        Task::perform(
            async move {
                crate::app::message::spawn_blocking_opt(move || Some(load_pins(projects)))
                    .await
                    .unwrap_or_else(|| Err("读取置顶会话任务失败".to_string()))
            },
            move |result| Message::SessionPinsLoaded(generation, result),
        )
    }

    pub(crate) fn apply_session_pins(
        &mut self,
        generation: u64,
        result: Result<HashSet<(String, String)>, String>,
    ) {
        if generation != self.session_ui_projection.pins_generation {
            return;
        }
        match result {
            Ok(pinned) => {
                if pinned != self.session_ui_projection.pinned {
                    self.session_ui_projection.pinned = pinned;
                    self.session_ui_projection.sidebar.get_mut().take();
                }
            }
            Err(error) => tracing::warn!(%error, "Failed to refresh pinned sessions"),
        }
    }
}

fn pins_data_dir() -> Result<std::path::PathBuf, String> {
    let home = directories::BaseDirs::new().ok_or_else(|| "无法定位置顶会话目录".to_string())?;
    Ok(vw_config_types::paths::home_config_dir(home.home_dir()))
}

fn pins_scope(project: &str) -> String {
    let tenant = vw_gateway_client::vw_api_types::id::TenantScope::default();
    vw_shared::session::ui_store::pinned_threads_scope(
        tenant.platform_id().as_ref(),
        tenant.company_id().as_ref(),
        project,
    )
}

fn load_pins(projects: HashSet<String>) -> Result<HashSet<(String, String)>, String> {
    let data_dir = pins_data_dir()?;
    let mut pinned = HashSet::new();
    for project in projects {
        let scope = pins_scope(&project);
        let ids = vw_shared::session::ui_store::load_pinned_threads(&data_dir, &scope)
            .map_err(|error| error.to_string())?;
        pinned.extend(ids.into_iter().map(|id| (project.clone(), id)));
    }
    Ok(pinned)
}

#[cfg(test)]
#[path = "ui_projection_tests.rs"]
mod tests;
