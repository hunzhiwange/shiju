//! 管理任务宠物窗口的条目、头像、动画与窗口交互状态。

use super::*;
use crate::app::models::ChatRole;
use std::cmp::Reverse;
use std::path::Path;

const PET_PANEL_WIDTH: f32 = 360.0;
const PET_PANEL_HEIGHT: f32 = 346.0;
const PET_COLLAPSED_WIDTH: f32 = 112.0;
const PET_COLLAPSED_HEIGHT: f32 = 104.0;
const PET_MARGIN: f32 = 12.0;
const PET_DOUBLE_CLICK_MS: u128 = 500;
const PET_EXPAND_ANIMATION_STEP: f32 = 0.24;
const PET_WALK_ANIMATION_MS: u64 = 360;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 任务宠物可轮换的头像类别。
pub(crate) enum TaskPetAvatarKind {
    Robot,
    Beauty,
    Handsome,
}

impl TaskPetAvatarKind {
    fn next(self) -> Self {
        match self {
            Self::Robot => Self::Beauty,
            Self::Beauty => Self::Handsome,
            Self::Handsome => Self::Robot,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 任务宠物条目对应请求的执行状态。
pub(crate) enum TaskPetStatus {
    Running,
    Completed,
}

#[derive(Debug, Clone)]
/// 宠物窗口中单个代理请求的稳定展示快照。
pub(crate) struct TaskPetItem {
    pub(crate) request_id: u64,
    pub(crate) session_id: String,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) project: Option<String>,
    pub(crate) status: TaskPetStatus,
    pub(crate) last_click_at: Option<web_time::Instant>,
}

#[derive(Debug, Clone)]
struct TaskPetSnapshot {
    title: String,
    detail: Option<String>,
}

impl TaskPetItem {
    fn running(request: &AgentRequest, snapshot: TaskPetSnapshot) -> Self {
        Self {
            request_id: request.id,
            session_id: request.session.clone(),
            title: snapshot.title,
            detail: snapshot.detail.unwrap_or_else(|| "思考中...".to_string()),
            project: task_pet_project_label(request.root.as_deref()),
            status: TaskPetStatus::Running,
            last_click_at: None,
        }
    }
}

fn task_pet_title(query: &str) -> String {
    let title = query.lines().find(|line| !line.trim().is_empty()).unwrap_or("继续执行任务").trim();
    truncate_for_pet(title, 26)
}

fn task_pet_detail(value: &str) -> String {
    let compact = task_pet_compact_detail(value);
    if compact.trim().is_empty() {
        "思考中...".to_string()
    } else {
        truncate_for_pet(compact.trim(), 58)
    }
}

fn task_pet_compact_detail(value: &str) -> String {
    let mut lines = value.lines().map(str::trim).filter(|line| !line.is_empty());
    let first = lines.next().unwrap_or_default();
    let rest = lines.collect::<Vec<_>>().join(" ");
    if rest.is_empty() { first.to_string() } else { rest }
}

fn truncate_for_pet(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let collected = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{collected}...") } else { collected }
}

fn task_pet_project_label(root: Option<&str>) -> Option<String> {
    let root = root?.trim();
    if root.is_empty() {
        return None;
    }
    Path::new(root)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| truncate_for_pet(name.trim(), 18))
}

fn find_tool_start(value: &str) -> Option<usize> {
    if value.starts_with("tool ") {
        return Some(0);
    }
    value.find("\ntool ").map(|idx| idx + 1)
}

fn valid_think_tag_suffix(value: &str) -> bool {
    value.is_empty() || value.chars().next().is_some_and(char::is_whitespace)
}

fn current_open_think_body(value: &str) -> Option<String> {
    let (_, _, thinking_open) = crate::app::ui::chat::split_think(value);
    if !thinking_open {
        return None;
    }

    let mut search_from = 0usize;
    let mut last_open_end = None;
    while let Some(pos_rel) = value[search_from..].find("<think") {
        let pos = search_from + pos_rel;
        if value[pos..].starts_with("</think") {
            search_from = pos + 1;
            continue;
        }
        let Some(tag_end_rel) = value[pos..].find('>') else {
            break;
        };
        let tag_end = pos + tag_end_rel;
        let tag_suffix = &value[pos + "<think".len()..tag_end];
        if valid_think_tag_suffix(tag_suffix) {
            last_open_end = Some(tag_end + 1);
        }
        search_from = pos + 1;
    }

    let open_end = last_open_end?;
    let tail = &value[open_end..];
    let end = find_tool_start(tail).unwrap_or(tail.len());
    Some(tail[..end].trim_end().to_string())
}

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

fn task_pet_window_size_for_progress(progress: f32) -> iced::Size {
    let eased = smooth_progress(progress.clamp(0.0, 1.0));
    iced::Size::new(
        PET_COLLAPSED_WIDTH + (PET_PANEL_WIDTH - PET_COLLAPSED_WIDTH) * eased,
        PET_COLLAPSED_HEIGHT + (PET_PANEL_HEIGHT - PET_COLLAPSED_HEIGHT) * eased,
    )
}

fn smooth_progress(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

impl App {
    /// 注册主窗口与宠物窗口标识，并初始化宠物窗口位置。
    pub fn register_window_ids(
        &mut self,
        main_window_id: iced::window::Id,
        task_pet_window_id: Option<iced::window::Id>,
    ) {
        self.main_window_id = Some(main_window_id);
        self.task_pet_window_id = task_pet_window_id;
    }

    pub fn task_pet_window_size(&self) -> iced::Size {
        task_pet_window_size_for_progress(self.task_pet_expand_progress)
    }

    pub(crate) fn task_pet_render_collapsed(&self) -> bool {
        self.task_pet_collapsed && self.task_pet_expand_target.is_none()
    }

    /// 从运行时请求重建宠物条目，同时保留仍有效的完成项与交互选择。
    pub(crate) fn sync_task_pet_from_runtime(&mut self) {
        let active = self
            .session_runtime_states
            .values()
            .filter_map(|runtime| {
                if !runtime.is_requesting {
                    return None;
                }
                let request = runtime.active_agent_request.as_ref()?;
                if self.task_pet_dismissed_request_ids.contains(&request.id) {
                    return None;
                }
                Some((request.clone(), self.task_pet_snapshot_for_request(request)))
            })
            .collect::<Vec<_>>();

        let active_ids = active.iter().map(|(request, _)| request.id).collect::<HashSet<_>>();
        for item in &mut self.task_pet_items {
            if item.status == TaskPetStatus::Running && !active_ids.contains(&item.request_id) {
                item.status = TaskPetStatus::Completed;
                item.detail = if item.detail.trim().is_empty() {
                    "已完成".to_string()
                } else {
                    item.detail.clone()
                };
            }
        }

        for (request, snapshot) in active {
            let existing_idx =
                self.task_pet_items.iter().position(|item| item.request_id == request.id).or_else(
                    || {
                        self.task_pet_items.iter().position(|item| {
                            item.session_id == request.session && item.title == snapshot.title
                        })
                    },
                );
            if let Some(idx) = existing_idx {
                let existing = &mut self.task_pet_items[idx];
                existing.request_id = request.id;
                existing.title = snapshot.title;
                existing.session_id = request.session.clone();
                if let Some(detail) = snapshot.detail {
                    existing.detail = detail;
                }
                existing.project = task_pet_project_label(request.root.as_deref());
                existing.status = TaskPetStatus::Running;
            } else {
                self.task_pet_items.push(TaskPetItem::running(&request, snapshot));
            }
        }

        self.task_pet_items
            .sort_by_key(|item| (item.status != TaskPetStatus::Running, Reverse(item.request_id)));
    }

    pub(crate) fn task_pet_active_count(&self) -> usize {
        self.task_pet_items.iter().filter(|item| item.status == TaskPetStatus::Running).count()
    }

    pub(crate) fn task_pet_visible_count(&self) -> usize {
        self.task_pet_items.len()
    }

    pub(crate) fn toggle_task_pet_collapsed(&mut self) {
        self.task_pet_collapsed = !self.task_pet_collapsed;
        self.task_pet_expand_target = Some(if self.task_pet_collapsed { 0.0 } else { 1.0 });
    }

    /// 推进折叠动画；尺寸稳定后返回 `None` 以停止继续调度帧。
    pub(crate) fn advance_task_pet_expand_animation(
        &mut self,
    ) -> Option<(iced::Size, iced::Point)> {
        let target = self.task_pet_expand_target?;
        let old_size = self.task_pet_window_size();
        let delta = target - self.task_pet_expand_progress;
        if delta.abs() <= PET_EXPAND_ANIMATION_STEP {
            self.task_pet_expand_progress = target;
            self.task_pet_expand_target = None;
        } else {
            self.task_pet_expand_progress += PET_EXPAND_ANIMATION_STEP.copysign(delta);
        }
        self.task_pet_expand_progress = self.task_pet_expand_progress.clamp(0.0, 1.0);

        let new_size = self.task_pet_window_size();
        let new_position = iced::Point::new(
            (self.task_pet_position.x + old_size.width - new_size.width).max(0.0),
            (self.task_pet_position.y + old_size.height - new_size.height).max(0.0),
        );
        self.task_pet_position = new_position;
        Some((new_size, new_position))
    }

    pub(crate) fn start_task_pet_drag(&mut self) {
        self.task_pet_dragging = true;
        self.task_pet_drag_anchor = None;
        self.task_pet_drag_start = self.task_pet_position;
        self.task_pet_walk_until_ms = crate::app::time::now_ms() + PET_WALK_ANIMATION_MS;
    }

    /// 根据拖动偏移移动窗口，并将位置限制在当前显示器工作区内。
    pub(crate) fn drag_task_pet_to(&mut self, x: f32, y: f32) {
        if self.task_pet_drag_anchor.is_none() {
            self.task_pet_drag_anchor = Some(iced::Point::new(x, y));
            return;
        }
        let anchor = self.task_pet_drag_anchor.unwrap_or(iced::Point::new(x, y));
        let panel_width = self.task_pet_window_size().width;
        let max_x = (self.window_size.0 - panel_width - PET_MARGIN).max(PET_MARGIN);
        let max_y = (self.window_size.1 - PET_PANEL_HEIGHT - PET_MARGIN).max(PET_MARGIN);
        let position = iced::Point::new(
            (self.task_pet_drag_start.x + x - anchor.x).clamp(PET_MARGIN, max_x),
            (self.task_pet_drag_start.y + y - anchor.y).clamp(PET_MARGIN, max_y),
        );
        self.move_task_pet_to(position);
    }

    pub(crate) fn finish_task_pet_drag(&mut self) {
        self.task_pet_dragging = false;
        self.task_pet_drag_anchor = None;
        self.task_pet_walk_until_ms = crate::app::time::now_ms() + PET_WALK_ANIMATION_MS / 2;
    }

    pub(crate) fn pulse_task_pet_motion(&mut self) {
        self.task_pet_walk_until_ms = crate::app::time::now_ms() + PET_WALK_ANIMATION_MS;
    }

    pub(crate) fn move_task_pet_window_to(&mut self, x: f32, y: f32) {
        self.move_task_pet_to(iced::Point::new(x, y));
    }

    fn move_task_pet_to(&mut self, position: iced::Point) {
        let delta_x = position.x - self.task_pet_position.x;
        if delta_x.abs() >= 0.5 {
            self.task_pet_drag_direction = if delta_x < 0.0 { -1 } else { 1 };
            self.task_pet_walk_until_ms = crate::app::time::now_ms() + PET_WALK_ANIMATION_MS;
        }
        self.task_pet_position = position;
    }

    pub(crate) fn task_pet_is_walking(&self) -> bool {
        self.task_pet_dragging || crate::app::time::now_ms() <= self.task_pet_walk_until_ms
    }

    pub(crate) fn task_pet_facing_left(&self) -> bool {
        self.task_pet_drag_direction < 0
    }

    pub(crate) fn set_task_pet_robot_hovered(&mut self, hovered: bool) {
        self.task_pet_robot_hovered = hovered;
    }

    pub(crate) fn cycle_task_pet_avatar(&mut self) {
        self.task_pet_avatar_kind = self.task_pet_avatar_kind.next();
    }

    pub(crate) fn refresh_task_pet_session_title(&mut self, session_id: &str, title: &str) {
        let title = task_pet_title(title);
        for item in &mut self.task_pet_items {
            if item.session_id == session_id {
                item.title = title.clone();
            }
        }
    }

    pub(crate) fn dismiss_task_pet_item(&mut self, request_id: u64) {
        self.task_pet_dismissed_request_ids.insert(request_id);
        self.task_pet_items.retain(|item| item.request_id != request_id);
        if self.task_pet_hovered_request_id == Some(request_id) {
            self.task_pet_hovered_request_id = None;
        }
        if self.task_pet_reply_request_id == Some(request_id) {
            self.task_pet_reply_request_id = None;
            self.task_pet_reply_input.clear();
        }
    }

    pub(crate) fn set_task_pet_hovered(&mut self, request_id: Option<u64>) {
        self.task_pet_hovered_request_id = request_id;
    }

    pub(crate) fn open_task_pet_reply(&mut self, request_id: u64) {
        if self.task_pet_reply_request_id == Some(request_id) {
            self.task_pet_reply_request_id = None;
            self.task_pet_reply_input.clear();
            return;
        }
        self.task_pet_reply_request_id = Some(request_id);
        self.task_pet_hovered_request_id = Some(request_id);
    }

    pub(crate) fn update_task_pet_reply_input(&mut self, value: String) {
        self.task_pet_reply_input = value;
    }

    /// 取出非空回复及目标会话，并同时关闭回复编辑状态。
    pub(crate) fn take_task_pet_reply(&mut self) -> Option<(String, String)> {
        let request_id = self.task_pet_reply_request_id?;
        let item = self.task_pet_items.iter().find(|item| item.request_id == request_id)?;
        let input = self.task_pet_reply_input.trim().to_string();
        if input.is_empty() {
            return None;
        }
        let session_id = item.session_id.clone();
        self.task_pet_reply_request_id = None;
        self.task_pet_reply_input.clear();
        Some((session_id, input))
    }

    /// 将单击解释为选择、双击解释为打开会话，并返回可导航会话标识。
    pub(crate) fn task_pet_item_clicked(&mut self, request_id: u64) -> Option<String> {
        let now = web_time::Instant::now();
        let Some(item) = self.task_pet_items.iter_mut().find(|item| item.request_id == request_id)
        else {
            return None;
        };
        let session_id = item.session_id.clone();
        if item.status != TaskPetStatus::Completed {
            item.last_click_at = Some(now);
            return Some(session_id);
        }
        let remove = item
            .last_click_at
            .is_some_and(|last| now.duration_since(last).as_millis() <= PET_DOUBLE_CLICK_MS);
        if remove {
            self.dismiss_task_pet_item(request_id);
            None
        } else {
            item.last_click_at = Some(now);
            Some(session_id)
        }
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
