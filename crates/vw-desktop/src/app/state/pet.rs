//! 管理任务宠物窗口的条目、头像、动画与窗口交互状态。

use super::*;
use crate::app::models::ChatRole;

pub use desktop_task_board::pet_state::TaskPetAvatarKind;
pub(crate) use desktop_task_board::pet_state::{TaskPetItem, TaskPetStatus};

use desktop_task_board::pet_interaction::{self, *};

fn newest_open_thinking_text<'a>(
    messages: impl DoubleEndedIterator<Item = &'a ChatMessage>,
) -> Option<String> {
    messages.rev().find_map(|message| {
        if message.role == ChatRole::Assistant {
            current_open_think_body(&message.content).map(|content| task_pet_detail(&content))
        } else {
            None
        }
    })
}

impl App {
    fn task_pet_state(&mut self) -> TaskPetState<'_> {
        TaskPetState {
            task_pet_avatar_kind: &mut self.task_pet_avatar_kind,
            task_pet_collapsed: &mut self.task_pet_collapsed,
            task_pet_dismissed_request_ids: &mut self.task_pet_dismissed_request_ids,
            task_pet_drag_anchor: &mut self.task_pet_drag_anchor,
            task_pet_drag_direction: &mut self.task_pet_drag_direction,
            task_pet_drag_start: &mut self.task_pet_drag_start,
            task_pet_dragging: &mut self.task_pet_dragging,
            task_pet_expand_progress: &mut self.task_pet_expand_progress,
            task_pet_expand_target: &mut self.task_pet_expand_target,
            task_pet_hovered_request_id: &mut self.task_pet_hovered_request_id,
            task_pet_items: &mut self.task_pet_items,
            task_pet_position: &mut self.task_pet_position,
            task_pet_reply_input: &mut self.task_pet_reply_input,
            task_pet_reply_request_id: &mut self.task_pet_reply_request_id,
            task_pet_robot_hovered: &mut self.companion_navigation.task_pet_robot_hovered,
            task_pet_scale: &mut self.task_pet_scale,
            task_pet_size_draft: &mut self.task_pet_size_draft,
            task_pet_walk_until_ms: &mut self.task_pet_walk_until_ms,
            window_size: self.window_size,
            now_ms: crate::app::time::now_ms(),
        }
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
        desktop_task_board::pet_interaction::task_pet_preview_scale(&self.task_pet_scale, &self.task_pet_size_draft)
    }

    pub(crate) fn task_pet_avatar_bounds(&self) -> f32 {
        desktop_task_board::pet_interaction::task_pet_avatar_bounds(&self.task_pet_avatar_kind, &self.task_pet_scale, &self.task_pet_size_draft)
    }

    pub(crate) fn resize_task_pet_preview(
        &mut self,
        previous_size: iced::Size,
    ) -> iced::Task<crate::app::Message> {
        self.companion_navigation.task_pet_robot_hovered = false;
        let Some(id) = self.task_pet_window_id else {
            return iced::Task::none();
        };
        let size = self.task_pet_window_size();
        self.task_pet_position = iced::Point::new(
            (self.task_pet_position.x + previous_size.width - size.width).max(0.0),
            (self.task_pet_position.y + previous_size.height - size.height).max(0.0),
        );
        iced::window::resize(id, size).chain(iced::window::move_to(id, self.task_pet_position))
    }

    pub fn task_pet_window_size(&self) -> iced::Size {
        desktop_task_board::pet_interaction::task_pet_window_size(&self.task_pet_avatar_kind, &self.task_pet_expand_progress, &self.task_pet_items, &self.task_pet_reply_request_id, &self.task_pet_scale, &self.task_pet_size_draft)
    }

    pub(crate) fn task_pet_render_collapsed(&self) -> bool {
        desktop_task_board::pet_interaction::task_pet_render_collapsed(&self.task_pet_collapsed, &self.task_pet_expand_progress)
    }

    /// 从运行时请求重建宠物条目，同时保留仍有效的完成项与交互选择。
    pub(crate) fn sync_task_pet_from_runtime(&mut self) {
        let previous_size = self.task_pet_window_size();
        let active = self
            .session_runtime_states
            .values()
            .filter_map(|runtime| {
                if !runtime.request_lifecycle.is_requesting {
                    return None;
                }
                let request = runtime.request_lifecycle.active_agent_request.as_ref()?;
                if self.task_pet_dismissed_request_ids.contains(&request.id) {
                    return None;
                }
                Some((request.clone(), self.task_pet_snapshot_for_request(request)))
            })
            .collect::<Vec<_>>();

        pet_interaction::sync_items(&mut self.task_pet_items, active.into_iter().map(|(request, snapshot)| (request.id, request.session.clone(), request.root.clone(), snapshot)).collect());
        if self.task_pet_window_size() != previous_size {
            self.request_task_pet_resize();
        }
    }

    fn request_task_pet_resize(&mut self) {
        self.task_pet_state().request_task_pet_resize()
    }

    pub(crate) fn task_pet_active_count(&self) -> usize {
        desktop_task_board::pet_interaction::task_pet_active_count(&self.task_pet_items)
    }

    pub(crate) fn toggle_task_pet_collapsed(&mut self) {
        self.task_pet_state().toggle_task_pet_collapsed()
    }

    /// 推进折叠动画；尺寸稳定后返回 `None` 以停止继续调度帧。
    pub(crate) fn advance_task_pet_expand_animation(
        &mut self,
    ) -> Option<(iced::Size, iced::Point)> {
        self.task_pet_state().advance_task_pet_expand_animation()
    }

    pub(crate) fn start_task_pet_drag(&mut self) {
        self.task_pet_state().start_task_pet_drag()
    }

    /// 根据拖动偏移移动窗口，并将位置限制在当前显示器工作区内。
    pub(crate) fn drag_task_pet_to(&mut self, x: f32, y: f32) {
        self.task_pet_state().drag_task_pet_to(x, y)
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

    fn move_task_pet_to(&mut self, position: iced::Point) {
        self.task_pet_state().move_task_pet_to(position)
    }

    pub(crate) fn task_pet_is_walking(&self) -> bool {
        desktop_task_board::pet_interaction::task_pet_is_walking(&self.task_pet_dragging, &self.task_pet_walk_until_ms, crate::app::time::now_ms())
    }

    pub(crate) fn task_pet_facing_left(&self) -> bool {
        desktop_task_board::pet_interaction::task_pet_facing_left(&self.task_pet_drag_direction)
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
        let from_active_chat = self
            .active_session_id
            .as_ref()
            .filter(|session_id| *session_id == &request.session)
            .and_then(|_| newest_open_thinking_text(self.chat.iter()));
        let from_cached = self
            .session_chat_cache
            .get(&request.session)
            .and_then(|messages| newest_open_thinking_text(messages.iter()));
        let detail = from_active_chat.or(from_cached);
        TaskPetSnapshot { title, detail }
    }

    fn task_pet_session_title(&self, session_id: &str, fallback: &str) -> String {
        let title = self
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| session.title.trim())
            .or_else(|| {
                self.project_sessions
                    .values()
                    .flat_map(|sessions| sessions.iter())
                    .find(|session| session.id == session_id)
                    .map(|session| session.title.trim())
            })
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| fallback.trim());
        task_pet_title(title)
    }
}

#[cfg(test)]
#[path = "pet_tests.rs"]
mod pet_tests;
