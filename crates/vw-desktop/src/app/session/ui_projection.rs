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
    pub(crate) agent_sessions: RefCell<Option<Arc<Vec<AgentChildSession>>>>,
    pub(crate) sidebar: RefCell<Option<Arc<crate::app::views::project::WorkSidebarModel>>>,
    pub(crate) pinned: HashSet<(String, String)>,
    pins_generation: u64,
}

fn preserves_projection(message: &Message) -> bool {
    match message {
        Message::None | Message::CopyDone | Message::Batch(_) => true,
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
            | ViewMessage::TabHovered(_),
        ) => true,
        Message::Preview(_) | Message::Editor(_) => true,
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
    pub(crate) fn prepare_session_ui_projection(&mut self, message: &Message) -> bool {
        if !preserves_projection(message) {
            if self.session_ui_projection.sidebar.borrow().is_some() {
                match message {
                    Message::Chat(inner) => {
                        eprintln!("PROJECTION_INVALIDATE Chat {:?}", std::mem::discriminant(inner))
                    }
                    Message::View(inner) => {
                        eprintln!("PROJECTION_INVALIDATE View {:?}", std::mem::discriminant(inner))
                    }
                    Message::Project(inner) => eprintln!(
                        "PROJECTION_INVALIDATE Project {:?}",
                        std::mem::discriminant(inner)
                    ),
                    _ => eprintln!(
                        "PROJECTION_INVALIDATE Other {:?}",
                        std::mem::discriminant(message)
                    ),
                }
            }
            self.session_ui_projection.agent_sessions.get_mut().take();
            self.session_ui_projection.sidebar.get_mut().take();
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

fn load_pins(projects: HashSet<String>) -> Result<HashSet<(String, String)>, String> {
    let home = directories::BaseDirs::new().ok_or_else(|| "无法定位置顶会话目录".to_string())?;
    let data_dir = vw_config_types::paths::home_config_dir(home.home_dir());
    let tenant = vw_gateway_client::vw_api_types::id::TenantScope::default();
    let mut pinned = HashSet::new();
    for project in projects {
        let scope = vw_shared::session::ui_store::pinned_threads_scope(
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            &project,
        );
        let ids = vw_shared::session::ui_store::load_pinned_threads(&data_dir, &scope)
            .map_err(|error| error.to_string())?;
        pinned.extend(ids.into_iter().map(|id| (project.clone(), id)));
    }
    Ok(pinned)
}

#[cfg(test)]
#[path = "ui_projection_tests.rs"]
mod tests;
