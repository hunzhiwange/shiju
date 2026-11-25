//! 管理任务宠物窗口的条目、头像、动画与窗口交互状态。

use super::*;
use crate::app::models::ChatRole;

pub use desktop_task_board::pet_state::TaskPetAvatarKind;

use desktop_task_board::pet_interaction::{self, *};


impl App {
    fn task_pet_state(&mut self) -> TaskPetState<'_> {
        self.pet_state.interaction(
            &mut self.companion_navigation.task_pet_robot_hovered,
            self.window_size,
            crate::app::time::now_ms(),
        )
    }

    /// 注册主窗口与宠物窗口标识，并初始化宠物窗口位置。
    pub fn register_window_ids(
        &mut self,
        main_window_id: iced::window::Id,
        task_pet_window_id: Option<iced::window::Id>,
    ) {
        self.main_window_id = Some(main_window_id);
        self.task_pet_window_id = task_pet_window_id;
    }

    pub(crate) fn task_pet_preview_scale(&self) -> f32 {
        desktop_task_board::pet_interaction::task_pet_preview_scale(&self.pet_state.task_pet_scale, &self.pet_state.task_pet_size_draft)
    }

    pub(crate) fn task_pet_avatar_bounds(&self) -> f32 {
        desktop_task_board::pet_interaction::task_pet_avatar_bounds(&self.pet_state.task_pet_avatar_kind, &self.pet_state.task_pet_scale, &self.pet_state.task_pet_size_draft)
    }

    pub fn task_pet_window_size(&self) -> iced::Size {
        let mut size = self.pet_state.window_size(self.task_pet_feedback().is_some(), iced::time::Instant::now());
        if self.companion_navigation.task_pet_context_menu_pos.is_some() {
            size.height = size.height.max(146.0);
        }
        size
    }

    /// Resize only at transition boundaries, keeping the dock centered on its previous anchor.
    pub(crate) fn refresh_task_pet_window(&mut self) -> iced::Task<crate::app::Message> {
        let Some(id) = self.task_pet_window_id.filter(|_| self.companion_navigation.task_pet_visible) else {
            return iced::Task::none();
        };
        let voice = self.task_pet_voice();
        let reminders = self.pet_state.reminder_height();
        let controls = &mut self.pet_state.controls;
        if self.pet_state.task_pet_items.is_empty() {
            controls.reminders_open = false;
            self.pet_state.task_pet_collapsed = true;
        }
        if voice.is_none() {
            controls.voice_owned = false;
        }
        controls.update(voice, reminders, iced::time::Instant::now());
        let size = self.task_pet_window_size();
        let Some(previous) = self.pet_state.controls.native_size.replace(size) else {
            return iced::Task::none();
        };
        if previous == size {
            return iced::Task::none();
        }
        let old_position = self.pet_state.task_pet_position;
        let position = pet_interaction::preview_position(old_position, previous, size);
        self.pet_state.task_pet_position = position;
        if let Some((x, y)) = &mut self.companion_navigation.task_pet_context_menu_pos {
            *x += old_position.x - position.x;
            *y += old_position.y - position.y;
        }
        iced::window::resize(id, size).chain(iced::window::move_to(id, position))
    }

    pub(crate) fn task_pet_feedback(&self) -> Option<&desktop_ui_controls::toast::Toast> {
        let id = self.pet_state.controls.feedback_id?;
        self.toast_state.active_toast.as_ref().filter(|toast| toast.id == id)
    }

    pub(crate) fn task_pet_voice(&self) -> Option<desktop_task_board::pet_view::PetVoice> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.pet_state.controls.voice_owned {
            use crate::app::message::voice::local_chat::Stage;
            use crate::app::voice::conversation::Phase;
            use desktop_task_board::pet_view::{PetVoice, VoiceOrbAction};
            if let Some(chat) = &self.voice.local_conversation {
                return Some(PetVoice {
                    muted: chat.muted,
                    microphone_enabled: true,
                    orb_action: match chat.stage {
                        Stage::Ready if !chat.muted => Some(VoiceOrbAction::Begin),
                        Stage::Preparing => Some(VoiceOrbAction::Interrupt),
                        Stage::Recording => Some(VoiceOrbAction::Finish),
                        _ => None,
                    },
                    can_interrupt: matches!(
                        chat.stage,
                        Stage::Recognizing | Stage::WaitingModel | Stage::Speaking
                    ),
                    connecting: chat.stage == Stage::Preparing,
                });
            }
            if let Some(chat) = &self.voice.conversation {
                let phase = chat.phase();
                return Some(PetVoice {
                    muted: phase == Phase::Muted,
                    microphone_enabled: matches!(phase, Phase::Listening | Phase::Speaking | Phase::Muted),
                    orb_action: Some(VoiceOrbAction::End),
                    can_interrupt: phase == Phase::Speaking,
                    connecting: phase == Phase::Connecting,
                });
            }
        }
        None
    }

    /// 从运行时请求重建宠物条目，同时保留仍有效的完成项与交互选择。
    pub(crate) fn sync_task_pet_from_runtime(&mut self) {
        let active = self.session_state.session_runtime_states
            .values()
            .filter_map(|runtime| {
                if !runtime.request_lifecycle.is_requesting {
                    return None;
                }
                let request = runtime.request_lifecycle.active_agent_request.as_ref()?;
                if self.pet_state.task_pet_dismissed_request_ids.contains(&request.id) {
                    return None;
                }
                Some((request.clone(), self.task_pet_snapshot_for_request(request)))
            })
            .collect::<Vec<_>>();

        pet_interaction::sync_items(&mut self.pet_state.task_pet_items, active.into_iter().map(|(request, snapshot)| (request.id, request.session.clone(), request.root.clone(), snapshot)).collect());
        if self.pet_state.task_pet_reply_request_id.is_some_and(|id| {
            !self.pet_state.task_pet_items.iter().any(|item| item.request_id == id)
        }) {
            self.pet_state.task_pet_reply_request_id = None;
            self.pet_state.task_pet_reply_input.clear();
        }
    }

    pub(crate) fn task_pet_active_count(&self) -> usize {
        desktop_task_board::pet_interaction::task_pet_active_count(&self.pet_state.task_pet_items)
    }

    pub(crate) fn toggle_task_pet_collapsed(&mut self) {
        self.task_pet_state().toggle_task_pet_collapsed();
        if self.pet_state.task_pet_collapsed {
            self.pet_state.task_pet_reply_request_id = None;
            self.pet_state.task_pet_reply_input.clear();
        }
    }

    pub(crate) fn start_task_pet_drag(&mut self) {
        self.task_pet_state().start_task_pet_drag()
    }

    /// 根据拖动偏移移动窗口，并将位置限制在当前显示器工作区内。
    pub(crate) fn drag_task_pet_to(&mut self, x: f32, y: f32) {
        let size = self.task_pet_window_size();
        self.task_pet_state().drag_task_pet_to(x, y, size)
    }

    pub(crate) fn finish_task_pet_drag(&mut self) {
        self.task_pet_state().finish_task_pet_drag()
    }

    pub(crate) fn pulse_task_pet_motion(&mut self) {
        self.task_pet_state().pulse_task_pet_motion()
    }

    pub(crate) fn move_task_pet_window_to(&mut self, x: f32, y: f32) {
        self.task_pet_state().move_task_pet_window_to(x, y)
    }

    #[cfg(test)]
    fn move_task_pet_to(&mut self, position: iced::Point) {
        self.task_pet_state().move_task_pet_to(position)
    }

    pub(crate) fn task_pet_is_walking(&self) -> bool {
        desktop_task_board::pet_interaction::task_pet_is_walking(&self.pet_state.task_pet_dragging, &self.pet_state.task_pet_walk_until_ms, crate::app::time::now_ms())
    }

    pub(crate) fn task_pet_facing_left(&self) -> bool {
        desktop_task_board::pet_interaction::task_pet_facing_left(&self.pet_state.task_pet_drag_direction)
    }

    pub(crate) fn set_task_pet_robot_hovered(&mut self, hovered: bool) {
        self.task_pet_state().set_task_pet_robot_hovered(hovered)
    }

    pub(crate) fn cycle_task_pet_avatar(&mut self) {
        self.task_pet_state().cycle_task_pet_avatar()
    }

    pub(crate) fn refresh_task_pet_session_title(&mut self, session_id: &str, title: &str) {
        self.task_pet_state().refresh_task_pet_session_title(session_id, title)
    }

    pub(crate) fn dismiss_task_pet_item(&mut self, request_id: u64) {
        self.task_pet_state().dismiss_task_pet_item(request_id)
    }

    pub(crate) fn set_task_pet_hovered(&mut self, request_id: Option<u64>) {
        self.task_pet_state().set_task_pet_hovered(request_id)
    }

    pub(crate) fn open_task_pet_reply(&mut self, request_id: u64) {
        self.task_pet_state().open_task_pet_reply(request_id)
    }

    pub(crate) fn update_task_pet_reply_input(&mut self, value: String) {
        self.task_pet_state().update_task_pet_reply_input(value)
    }

    /// 取出非空回复及目标会话，并同时关闭回复编辑状态。
    pub(crate) fn take_task_pet_reply(&mut self) -> Option<(String, String)> {
        self.task_pet_state().take_task_pet_reply()
    }

    /// 将单击解释为选择、双击解释为打开会话，并返回可导航会话标识。
    pub(crate) fn task_pet_item_clicked(&mut self, request_id: u64) -> Option<String> {
        self.task_pet_state().task_pet_item_clicked(request_id)
    }

    fn task_pet_snapshot_for_request(&self, request: &AgentRequest) -> TaskPetSnapshot {
        let title = self.task_pet_session_title(&request.session, &request.query);
        let from_active_chat = self.session_state.active_session_id
            .as_ref()
            .filter(|session_id| *session_id == &request.session)
            .and_then(|_| newest_open_thinking_text(self.chat_view.chat.iter().map(|message| (message.role == ChatRole::Assistant, message.content.as_str()))));
        let from_cached = self.session_state.session_chat_cache
            .get(&request.session)
            .and_then(|messages| newest_open_thinking_text(messages.iter().map(|message| (message.role == ChatRole::Assistant, message.content.as_str()))));
        let detail = from_active_chat.or(from_cached);
        TaskPetSnapshot { title, detail }
    }

    fn task_pet_session_title(&self, session_id: &str, fallback: &str) -> String {
        pet_interaction::session_title(
            session_id,
            fallback,
            self.session_state
                .sessions
                .iter()
                .map(|session| (session.id.as_str(), session.title.as_str())),
            self.workspace_state
                .project_sessions
                .values()
                .flatten()
                .map(|session| (session.id.as_str(), session.title.as_str())),
        )
    }
}

#[cfg(test)]
#[path = "pet_tests.rs"]
mod pet_tests;

#[cfg(test)]
pub(crate) use desktop_task_board::pet_state::TaskPetItem;
#[cfg(test)]
pub(crate) use desktop_task_board::pet_state::TaskPetStatus;
