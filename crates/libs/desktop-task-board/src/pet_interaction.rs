//! 任务宠物的派生数据与局部交互状态。
use super::pet_state::*;
use iced::Point;
use std::{cmp::Reverse, collections::HashSet, path::Path};

const PET_PANEL_WIDTH: f32 = 360.0;
const PET_PANEL_HEIGHT: f32 = 404.0;
const PET_COLLAPSED_WIDTH: f32 = 160.0;
const PET_COLLAPSED_HEIGHT: f32 = 168.0;
const PET_MARGIN: f32 = 12.0;
const PET_DOUBLE_CLICK_MS: u128 = 500;
const PET_EXPAND_ANIMATION_STEP: f32 = 0.24;
const PET_WALK_ANIMATION_MS: u64 = 360;

#[derive(Debug, Clone)]
pub struct TaskPetSnapshot {
    pub title: String,
    pub detail: Option<String>,
}

pub fn task_pet_title(query: &str) -> String {
    let title = query.lines().find(|line| !line.trim().is_empty()).unwrap_or("继续执行任务").trim();
    truncate_for_pet(title, 26)
}

pub fn task_pet_detail(value: &str) -> String {
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

pub fn current_open_think_body(value: &str) -> Option<String> {
    let (_, _, thinking_open) = desktop_chat_view::message_view::think::split_think(value);
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


pub struct TaskPetState<'a> {
    pub task_pet_avatar_kind: &'a mut TaskPetAvatarKind,
    pub task_pet_collapsed: &'a mut bool,
    pub task_pet_dismissed_request_ids: &'a mut HashSet<u64>,
    pub task_pet_drag_anchor: &'a mut Option<Point>,
    pub task_pet_drag_direction: &'a mut i8,
    pub task_pet_drag_start: &'a mut Point,
    pub task_pet_dragging: &'a mut bool,
    pub task_pet_expand_progress: &'a mut f32,
    pub task_pet_expand_target: &'a mut Option<f32>,
    pub task_pet_hovered_request_id: &'a mut Option<u64>,
    pub task_pet_items: &'a mut Vec<TaskPetItem>,
    pub task_pet_position: &'a mut Point,
    pub task_pet_reply_input: &'a mut String,
    pub task_pet_reply_request_id: &'a mut Option<u64>,
    pub task_pet_robot_hovered: &'a mut bool,
    pub task_pet_scale: &'a mut f32,
    pub task_pet_size_draft: &'a mut Option<f32>,
    pub task_pet_walk_until_ms: &'a mut u64,
    pub window_size: (f32, f32),
    pub now_ms: u64,
}

impl TaskPetState<'_> {
    pub fn task_pet_preview_scale(&self) -> f32 {
        task_pet_preview_scale(&*self.task_pet_scale, &*self.task_pet_size_draft)
    }

    pub fn task_pet_avatar_bounds(&self) -> f32 {
        task_pet_avatar_bounds(&*self.task_pet_avatar_kind, &*self.task_pet_scale, &*self.task_pet_size_draft)
    }

    pub fn task_pet_window_size(&self) -> iced::Size {
        task_pet_window_size(&*self.task_pet_avatar_kind, &*self.task_pet_expand_progress, &*self.task_pet_items, &*self.task_pet_reply_request_id, &*self.task_pet_scale, &*self.task_pet_size_draft)
    }

    pub fn task_pet_render_collapsed(&self) -> bool {
        task_pet_render_collapsed(&*self.task_pet_collapsed, &*self.task_pet_expand_progress)
    }

    pub fn request_task_pet_resize(&mut self) {
        // Reuse the existing animation subscription for one geometry update.
        if (*self.task_pet_expand_target).is_none() {
            *self.task_pet_expand_target = Some((*self.task_pet_expand_progress));
        }
    }

    pub fn task_pet_active_count(&self) -> usize {
        task_pet_active_count(&*self.task_pet_items)
    }

    pub fn toggle_task_pet_collapsed(&mut self) {
        *self.task_pet_collapsed = !(*self.task_pet_collapsed);
        *self.task_pet_expand_target = Some(if (*self.task_pet_collapsed) { 0.0 } else { 1.0 });
    }

    pub fn advance_task_pet_expand_animation(
    &mut self,
    ) -> Option<(iced::Size, iced::Point)> {
        let target = (*self.task_pet_expand_target)?;
        let old_size = self.task_pet_window_size();
        let delta = target - (*self.task_pet_expand_progress);
        if delta.abs() <= PET_EXPAND_ANIMATION_STEP {
            *self.task_pet_expand_progress = target;
            *self.task_pet_expand_target = None;
        } else {
            *self.task_pet_expand_progress += PET_EXPAND_ANIMATION_STEP.copysign(delta);
        }
        *self.task_pet_expand_progress = (*self.task_pet_expand_progress).clamp(0.0, 1.0);

        let new_size = self.task_pet_window_size();
        let new_position = iced::Point::new(
        ((*self.task_pet_position).x + old_size.width - new_size.width).max(0.0),
        ((*self.task_pet_position).y + old_size.height - new_size.height).max(0.0),
        );
        *self.task_pet_position = new_position;
        Some((new_size, new_position))
    }

    pub fn start_task_pet_drag(&mut self) {
        *self.task_pet_dragging = true;
        *self.task_pet_drag_anchor = None;
        *self.task_pet_drag_start = (*self.task_pet_position);
        *self.task_pet_walk_until_ms = self.now_ms + PET_WALK_ANIMATION_MS;
    }

    pub fn drag_task_pet_to(&mut self, x: f32, y: f32) {
        if (*self.task_pet_drag_anchor).is_none() {
            *self.task_pet_drag_anchor = Some(iced::Point::new(x, y));
            return;
        }
        let anchor = (*self.task_pet_drag_anchor).unwrap_or(iced::Point::new(x, y));
        let panel_width = self.task_pet_window_size().width;
        let max_x = (self.window_size.0 - panel_width - PET_MARGIN).max(PET_MARGIN);
        let max_y =
        (self.window_size.1 - self.task_pet_window_size().height - PET_MARGIN).max(PET_MARGIN);
        let position = iced::Point::new(
        ((*self.task_pet_drag_start).x + x - anchor.x).clamp(PET_MARGIN, max_x),
        ((*self.task_pet_drag_start).y + y - anchor.y).clamp(PET_MARGIN, max_y),
        );
        self.move_task_pet_to(position);
    }

    pub fn finish_task_pet_drag(&mut self) {
        *self.task_pet_dragging = false;
        *self.task_pet_drag_anchor = None;
        *self.task_pet_walk_until_ms = self.now_ms + PET_WALK_ANIMATION_MS / 2;
    }

    pub fn pulse_task_pet_motion(&mut self) {
        *self.task_pet_walk_until_ms = self.now_ms + PET_WALK_ANIMATION_MS;
    }

    pub fn move_task_pet_window_to(&mut self, x: f32, y: f32) {
        self.move_task_pet_to(iced::Point::new(x, y));
    }

    pub fn move_task_pet_to(&mut self, position: iced::Point) {
        let delta_x = position.x - (*self.task_pet_position).x;
        if delta_x.abs() >= 0.5 {
            *self.task_pet_drag_direction = if delta_x < 0.0 { -1 } else { 1 };
            *self.task_pet_walk_until_ms = self.now_ms + PET_WALK_ANIMATION_MS;
        }
        *self.task_pet_position = position;
    }

    pub fn task_pet_is_walking(&self) -> bool {
        task_pet_is_walking(&*self.task_pet_dragging, &*self.task_pet_walk_until_ms, self.now_ms)
    }

    pub fn task_pet_facing_left(&self) -> bool {
        task_pet_facing_left(&*self.task_pet_drag_direction)
    }

    pub fn set_task_pet_robot_hovered(&mut self, hovered: bool) {
        *self.task_pet_robot_hovered = hovered;
    }

    pub fn cycle_task_pet_avatar(&mut self) {
        *self.task_pet_avatar_kind = (*self.task_pet_avatar_kind).next();
    }

    pub fn refresh_task_pet_session_title(&mut self, session_id: &str, title: &str) {
        let title = task_pet_title(title);
        for item in &mut (*self.task_pet_items) {
            if item.session_id == session_id {
                item.title = title.clone();
            }
        }
    }

    pub fn dismiss_task_pet_item(&mut self, request_id: u64) {
        (*self.task_pet_dismissed_request_ids).insert(request_id);
        (*self.task_pet_items).retain(|item| item.request_id != request_id);
        if *self.task_pet_hovered_request_id == Some(request_id) {
            *self.task_pet_hovered_request_id = None;
        }
        if *self.task_pet_reply_request_id == Some(request_id) {
            *self.task_pet_reply_request_id = None;
            (*self.task_pet_reply_input).clear();
        }
        self.request_task_pet_resize();
    }

    pub fn set_task_pet_hovered(&mut self, request_id: Option<u64>) {
        *self.task_pet_hovered_request_id = request_id;
    }

    pub fn open_task_pet_reply(&mut self, request_id: u64) {
        self.request_task_pet_resize();
        if *self.task_pet_reply_request_id == Some(request_id) {
            *self.task_pet_reply_request_id = None;
            (*self.task_pet_reply_input).clear();
            return;
        }
        (*self.task_pet_reply_input).clear();
        *self.task_pet_reply_request_id = Some(request_id);
        *self.task_pet_hovered_request_id = Some(request_id);
    }

    pub fn update_task_pet_reply_input(&mut self, value: String) {
        *self.task_pet_reply_input = value;
    }

    pub fn take_task_pet_reply(&mut self) -> Option<(String, String)> {
        let request_id = (*self.task_pet_reply_request_id)?;
        let item = (*self.task_pet_items).iter().find(|item| item.request_id == request_id)?;
        let input = (*self.task_pet_reply_input).trim().to_string();
        if input.is_empty() {
            return None;
        }
        let session_id = item.session_id.clone();
        *self.task_pet_reply_request_id = None;
        (*self.task_pet_reply_input).clear();
        self.request_task_pet_resize();
        Some((session_id, input))
    }

    pub fn task_pet_item_clicked(&mut self, request_id: u64) -> Option<String> {
        let now = web_time::Instant::now();
        let Some(item) = (*self.task_pet_items).iter_mut().find(|item| item.request_id == request_id)
        else {
            return None;
        };
        let session_id = item.session_id.clone();
        let double_clicked = item
        .last_click_at
        .is_some_and(|last| now.duration_since(last).as_millis() <= PET_DOUBLE_CLICK_MS);
        item.last_click_at = if double_clicked { None } else { Some(now) };
        double_clicked.then_some(session_id)
    }
}

fn running(request_id: u64, session_id: String, root: Option<&str>, snapshot: TaskPetSnapshot) -> TaskPetItem {
    TaskPetItem {
        request_id,
        session_id,
        title: snapshot.title,
        detail_feedback: snapshot
        .detail
        .is_none()
        .then(|| desktop_i18n::input::Feedback::new("思考中...", vec![])),
        detail: snapshot.detail.unwrap_or_else(|| "思考中...".to_string()),
        project: task_pet_project_label(root),
        status: TaskPetStatus::Running,
        last_click_at: None,
    }
}

pub fn sync_items(items: &mut Vec<TaskPetItem>, active: Vec<(u64, String, Option<String>, TaskPetSnapshot)>) {
    let active_ids = active.iter().map(|(request_id, _, _, _)| *request_id).collect::<HashSet<_>>();
    for item in items.iter_mut() {
        if item.status == TaskPetStatus::Running && !active_ids.contains(&item.request_id) {
            item.status = TaskPetStatus::Completed;
            if item.detail.trim().is_empty() {
                item.detail_feedback =
                Some(desktop_i18n::input::Feedback::new("已完成", vec![]));
            }
            item.detail = if item.detail.trim().is_empty() {
                "已完成".to_string()
            } else {
                item.detail.clone()
            };
        }
    }

    for (request_id, session_id, root, snapshot) in active {
        let existing_idx =
        items.iter().position(|item| item.request_id == request_id).or_else(
        || {
            items.iter().position(|item| {
                item.session_id == session_id && item.title == snapshot.title
            })
        },
        );
        if let Some(idx) = existing_idx {
            let existing = &mut items[idx];
            existing.request_id = request_id;
            existing.title = snapshot.title;
            existing.session_id = session_id.clone();
            if let Some(detail) = snapshot.detail {
                existing.detail_feedback = None;
                existing.detail = detail;
            }
            existing.project = task_pet_project_label(root.as_deref());
            existing.status = TaskPetStatus::Running;
        } else {
            items.push(running(request_id, session_id, root.as_deref(), snapshot));
        }
    }

    items
    .sort_by_key(|item| (item.status != TaskPetStatus::Running, Reverse(item.request_id)));
}

pub fn task_pet_is_walking(task_pet_dragging: &bool, task_pet_walk_until_ms: &u64, now_ms: u64) -> bool {
    (*task_pet_dragging) || now_ms <= (*task_pet_walk_until_ms)
}

pub fn task_pet_window_size(task_pet_avatar_kind: &TaskPetAvatarKind, task_pet_expand_progress: &f32, task_pet_items: &Vec<TaskPetItem>, task_pet_reply_request_id: &Option<u64>, task_pet_scale: &f32, task_pet_size_draft: &Option<f32>) -> iced::Size {
    let progress = smooth_progress((*task_pet_expand_progress).clamp(0.0, 1.0));
    let mut size = task_pet_window_size_for_progress((*task_pet_expand_progress));
    let count = (*task_pet_items).len();
    if count == 0 {
        size.height = PET_COLLAPSED_HEIGHT;
    } else {
        size.width = 320.0 + (PET_PANEL_WIDTH - 320.0) * progress;
        let reply_extra = if (*task_pet_reply_request_id).is_some() {
            TASK_PET_CARD_REPLY_HEIGHT - TASK_PET_CARD_HEIGHT
        } else {
            0.0
        };
        let collapsed_height = TASK_PET_CARD_HEIGHT
        + (count.min(TASK_PET_STACK_DEPTH) - 1) as f32 * TASK_PET_STACK_OFFSET
        + if (*task_pet_items).first().map(|item| item.request_id)
        == (*task_pet_reply_request_id)
        {
            reply_extra
        } else {
            0.0
        };
        let expanded_height = (count as f32 * (TASK_PET_CARD_HEIGHT + TASK_PET_CARD_SPACING)
        - TASK_PET_CARD_SPACING
        + reply_extra)
        .min(TASK_PET_LIST_HEIGHT);
        size.height = PET_COLLAPSED_HEIGHT
        + 6.0
        + collapsed_height * (1.0 - progress)
        + expanded_height * progress;
    }
    let extra = task_pet_avatar_bounds(task_pet_avatar_kind, task_pet_scale, task_pet_size_draft) - 104.0;
    size.width = size.width.max(task_pet_avatar_bounds(task_pet_avatar_kind, task_pet_scale, task_pet_size_draft) + 16.0);
    size.height += extra;
    size
}

pub fn task_pet_preview_scale(task_pet_scale: &f32, task_pet_size_draft: &Option<f32>) -> f32 {
    (*task_pet_size_draft).unwrap_or((*task_pet_scale))
}

pub fn task_pet_active_count(task_pet_items: &Vec<TaskPetItem>) -> usize {
    (*task_pet_items).iter().filter(|item| item.status == TaskPetStatus::Running).count()
}

pub fn task_pet_render_collapsed(task_pet_collapsed: &bool, task_pet_expand_progress: &f32) -> bool {
    (*task_pet_collapsed) && (*task_pet_expand_progress) <= 0.0
}

pub fn task_pet_facing_left(task_pet_drag_direction: &i8) -> bool {
    (*task_pet_drag_direction) < 0
}

pub fn task_pet_avatar_bounds(task_pet_avatar_kind: &TaskPetAvatarKind, task_pet_scale: &f32, task_pet_size_draft: &Option<f32>) -> f32 {
    let scale = if (*task_pet_avatar_kind) == TaskPetAvatarKind::None {
        1.0
    } else {
        task_pet_preview_scale(task_pet_scale, task_pet_size_draft).max(1.0)
    };
    104.0 * scale
}

#[cfg(test)]
#[path = "pet_interaction_tests.rs"]
mod tests;

pub fn newest_open_thinking_text<'a>(
    messages: impl DoubleEndedIterator<Item = (bool, &'a str)>,
) -> Option<String> {
    messages.rev().find_map(|(assistant, content)| {
        if assistant {
            current_open_think_body(content).map(|content| task_pet_detail(&content))
        } else {
            None
        }
    })
}

pub fn preview_position(position: Point, previous: iced::Size, size: iced::Size) -> Point {
    Point::new(
        (position.x + previous.width - size.width).max(0.0),
        (position.y + previous.height - size.height).max(0.0),
    )
}

pub fn session_title<'a>(
    session_id: &str,
    fallback: &str,
    sessions: impl IntoIterator<Item = (&'a str, &'a str)>,
    project_sessions: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> String {
    let title = sessions
        .into_iter()
        .find(|(id, _)| *id == session_id)
        .map(|(_, title)| title.trim())
        .or_else(|| {
            project_sessions
                .into_iter()
                .find(|(id, _)| *id == session_id)
                .map(|(_, title)| title.trim())
        })
        .filter(|title| !title.is_empty());
    match title {
        Some(title) => task_pet_title(title),
        None => task_pet_title(fallback.trim()),
    }
}
