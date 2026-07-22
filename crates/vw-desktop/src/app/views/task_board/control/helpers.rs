//! 任务看板控制面板辅助函数。
//!
//! 本模块承载控制面板和像素办公室视图共享的纯辅助逻辑，
//! 主要包括状态标签、颜色映射、日志预览和简单格式化函数。

use iced::{Color, Theme};

use crate::app::App;

/// 将 Worktree 槽位状态转换为中文显示标签。
pub(super) fn worktree_state_label(state: crate::app::task::WorktreeState) -> &'static str {
    match state {
        crate::app::task::WorktreeState::Idle => "空闲",
        crate::app::task::WorktreeState::Busy => "忙碌",
        crate::app::task::WorktreeState::Tainted => "污染",
        crate::app::task::WorktreeState::Recycling => "回收中",
        crate::app::task::WorktreeState::Dead => "失效",
    }
}

/// 根据当前时间生成动态的点动画，用于表示运行中状态。
pub(super) fn running_dots(now_ms: u64) -> &'static str {
    match ((now_ms / 1000) % 3) as u8 {
        0 => "·",
        1 => "··",
        _ => "···",
    }
}

/// 格式化持续时间（毫秒）为可读字符串。
pub(super) fn format_duration_ms(duration_ms: u64) -> String {
    let secs = (duration_ms / 1000) as i64;
    let value = vw_shared::util::format_duration(secs);
    if value.is_empty() { "0s".to_string() } else { value }
}

/// 获取任务状态标签的颜色配置。
pub(super) fn task_status_tag_colors(status: crate::app::task::TaskStatus) -> (Color, Color) {
    match status {
        crate::app::task::TaskStatus::Pool => {
            (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246))
        }
        crate::app::task::TaskStatus::Pending => {
            (Color::from_rgb8(37, 99, 235), Color::from_rgb8(219, 234, 254))
        }
        crate::app::task::TaskStatus::Planning => {
            (Color::from_rgb8(79, 70, 229), Color::from_rgb8(224, 231, 255))
        }
        crate::app::task::TaskStatus::Running => {
            (Color::from_rgb8(147, 51, 234), Color::from_rgb8(243, 232, 255))
        }
        crate::app::task::TaskStatus::Failed => {
            (Color::from_rgb8(220, 38, 38), Color::from_rgb8(254, 226, 226))
        }
        crate::app::task::TaskStatus::Paused => {
            (Color::from_rgb8(202, 138, 4), Color::from_rgb8(254, 249, 195))
        }
        crate::app::task::TaskStatus::CodeComplete => {
            (Color::from_rgb8(5, 150, 105), Color::from_rgb8(209, 250, 229))
        }
        crate::app::task::TaskStatus::CodeReview => {
            (Color::from_rgb8(217, 119, 6), Color::from_rgb8(254, 243, 199))
        }
        crate::app::task::TaskStatus::PrSubmitted => {
            (Color::from_rgb8(8, 145, 178), Color::from_rgb8(207, 250, 254))
        }
        crate::app::task::TaskStatus::Completed => {
            (Color::from_rgb8(22, 163, 74), Color::from_rgb8(220, 252, 231))
        }
        crate::app::task::TaskStatus::Archived => {
            (Color::from_rgb8(100, 116, 139), Color::from_rgb8(241, 245, 249))
        }
        // 自定义状态暂用中性灰，待工作流迁移任务接管自定义列样式。
        crate::app::task::TaskStatus::Custom(_) => {
            (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246))
        }
    }
}

/// 从 Worktree 槽位快照中获取关联的任务对象。
pub(super) fn slot_task<'a>(
    app: &'a App,
    slot: &'a crate::app::task::WorktreeSlotSnapshot,
) -> Option<&'a crate::app::task::Task> {
    let task_id = slot.leased_task_id.as_deref()?;
    app.task_board_tasks.iter().find(|task| task.id == task_id)
}

/// 获取 Worktree 状态的调色板配置。
pub(super) fn worktree_state_palette(
    theme: &Theme,
    state: crate::app::task::WorktreeState,
) -> (Color, Color, Color) {
    let p = theme.extended_palette();
    let base = theme.palette().background;
    let is_dark = base.r + base.g + base.b < 1.5;
    match state {
        crate::app::task::WorktreeState::Idle => {
            if is_dark {
                (
                    Color::from_rgb(0.17, 0.29, 0.20),
                    Color::from_rgb(0.77, 0.92, 0.80),
                    Color::from_rgb(0.36, 0.69, 0.46),
                )
            } else {
                (
                    Color::from_rgb(0.69, 0.92, 0.73),
                    Color::from_rgb(0.16, 0.39, 0.22),
                    p.success.strong.color,
                )
            }
        }
        crate::app::task::WorktreeState::Busy => {
            if is_dark {
                (
                    Color::from_rgb(0.30, 0.24, 0.14),
                    Color::from_rgb(0.98, 0.89, 0.67),
                    Color::from_rgb(0.78, 0.61, 0.24),
                )
            } else {
                (
                    Color::from_rgb(0.98, 0.82, 0.46),
                    Color::from_rgb(0.42, 0.23, 0.05),
                    p.warning.strong.color,
                )
            }
        }
        crate::app::task::WorktreeState::Tainted => {
            if is_dark {
                (
                    Color::from_rgb(0.34, 0.17, 0.18),
                    Color::from_rgb(0.98, 0.80, 0.80),
                    Color::from_rgb(0.80, 0.39, 0.40),
                )
            } else {
                (
                    Color::from_rgb(0.98, 0.58, 0.58),
                    Color::from_rgb(0.42, 0.09, 0.10),
                    p.danger.strong.color,
                )
            }
        }
        crate::app::task::WorktreeState::Recycling => {
            if is_dark {
                (
                    Color::from_rgb(0.15, 0.24, 0.31),
                    Color::from_rgb(0.80, 0.90, 0.98),
                    Color::from_rgb(0.39, 0.63, 0.82),
                )
            } else {
                (
                    Color::from_rgb(0.57, 0.82, 0.98),
                    Color::from_rgb(0.10, 0.26, 0.42),
                    p.background.strong.color,
                )
            }
        }
        crate::app::task::WorktreeState::Dead => {
            if is_dark {
                (
                    Color::from_rgb(0.22, 0.24, 0.28),
                    Color::from_rgb(0.83, 0.85, 0.90),
                    Color::from_rgb(0.48, 0.51, 0.58),
                )
            } else {
                (
                    Color::from_rgb(0.56, 0.58, 0.63),
                    Color::from_rgb(0.16, 0.18, 0.22),
                    p.secondary.strong.color,
                )
            }
        }
    }
}

/// 获取 Worktree 状态的提示文本。
pub(super) fn worktree_state_hint(state: crate::app::task::WorktreeState) -> &'static str {
    match state {
        crate::app::task::WorktreeState::Idle => "新任务到达后可立即执行；脏现场会在执行前统一整理",
        crate::app::task::WorktreeState::Busy => "当前正在处理任务",
        crate::app::task::WorktreeState::Tainted => "需要人工或自动修复",
        crate::app::task::WorktreeState::Recycling => "正在回收与整理",
        crate::app::task::WorktreeState::Dead => "当前暂不可用",
    }
}

/// 获取 Worktree 状态对应的角色图标和状态文本。
pub(super) fn worktree_state_actor(
    state: crate::app::task::WorktreeState,
) -> (&'static str, &'static str) {
    match state {
        crate::app::task::WorktreeState::Idle => ("🧑‍💻", "待复用"),
        crate::app::task::WorktreeState::Busy => ("👨‍💻", "任务处理中"),
        crate::app::task::WorktreeState::Tainted => ("🕵️", "异常排查中"),
        crate::app::task::WorktreeState::Recycling => ("🧹", "回收整理中"),
        crate::app::task::WorktreeState::Dead => ("💤", "槽位停用中"),
    }
}

/// 获取 Worktree 房间道具列表（根据状态变化）。
pub(super) fn worktree_room_props(state: crate::app::task::WorktreeState) -> [&'static str; 5] {
    match state {
        crate::app::task::WorktreeState::Idle => ["🪑", "🖥️", "☕", "🪴", "📋"],
        crate::app::task::WorktreeState::Busy => ["🪑", "🖥️", "⌨️", "⚙️", "📞"],
        crate::app::task::WorktreeState::Tainted => ["🚧", "🧪", "🧰", "📁", "⚠️"],
        crate::app::task::WorktreeState::Recycling => ["🧹", "🧴", "🗂️", "🪣", "🔧"],
        crate::app::task::WorktreeState::Dead => ["🔌", "🗄️", "🧯", "📦", "⏸️"],
    }
}

/// 截断日志行到指定字符数。
pub(super) fn truncate_log_line(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    let mut result = String::new();
    let mut count = 0usize;

    for ch in trimmed.chars() {
        if count >= max_chars {
            result.push_str("...");
            return result;
        }
        result.push(ch);
        count += 1;
    }

    if result.is_empty() { "...".to_string() } else { result }
}

/// 获取 Worktree 槽位的日志预览行。
pub(super) fn worktree_log_preview_lines(
    app: &App,
    slot: &crate::app::task::WorktreeSlotSnapshot,
    now_ms: u64,
) -> Vec<String> {
    let Some(task_id) = slot.leased_task_id.as_deref() else {
        return vec!["等待任务日志...".to_string()];
    };

    let Some(task) = slot_task(app, slot) else {
        return vec![format!("任务 {}", truncate_log_line(task_id, 52))];
    };

    let mut lines = task
        .logs
        .iter()
        .rev()
        .filter_map(|entry| {
            let line = entry.message.replace('\n', " ");
            let trimmed = line.trim();
            if trimmed.is_empty() { None } else { Some(truncate_log_line(trimmed, 52)) }
        })
        .take(6)
        .collect::<Vec<_>>();

    if lines.is_empty() {
        lines.push("任务已启动...".to_string());
    }

    lines.reverse();

    let visible = 3usize.min(lines.len());
    let offset = lines.len().saturating_sub(visible);
    let _ = now_ms;
    lines.into_iter().skip(offset).take(visible).collect()
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
