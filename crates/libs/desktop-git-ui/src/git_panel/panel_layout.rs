//! Git 文件列表布局、主题和虚拟窗口计算。
use iced::Color;
use std::collections::{HashMap, HashSet};
use crate::DiffTheme;
use super::{utils, ops::DiffFileMeta};
pub struct FileListInput<'a> {
    pub git_focused_file: &'a Option<String>,
    pub expanded_files_set: &'a HashSet<String>,
    pub git_diff_contents_loading: &'a HashSet<String>,
    pub git_diff_contents: &'a HashMap<String, (String, String)>,
    pub git_diff_scroll_viewport_h: f32,
    pub git_diff_scroll_offset_y: f32,
}
pub fn is_dark_theme(theme: &iced::Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

pub fn get_diff_colors(theme: DiffTheme) -> (Color, Color, Color, Color, Color, Color, Color, Color) {
    utils::get_diff_colors(theme)
}

pub const GIT_FILE_CARD_GAP: f32 = 10.0;
pub const GIT_FILE_HEADER_ESTIMATED_HEIGHT: f32 = 32.0;
pub const GIT_FILE_FOCUSED_EXTRA_HEIGHT: f32 = 12.0;
pub const GIT_FILE_LOADING_PLACEHOLDER_HEIGHT: f32 = 44.0;
pub const GIT_FILE_LINE_ESTIMATED_HEIGHT: f32 = 22.0;
pub const GIT_FILE_VIRTUALIZATION_BUFFER: usize = 12;
pub const GIT_FILE_VIRTUALIZATION_MIN_ITEMS: usize = 48;

pub fn sum_git_file_block_height(heights: &[f32]) -> f32 {
    if heights.is_empty() {
        0.0
    } else {
        heights.iter().sum::<f32>() + GIT_FILE_CARD_GAP * heights.len().saturating_sub(1) as f32
    }
}

pub fn estimate_git_file_card_height(app: &FileListInput<'_>, meta: &DiffFileMeta) -> f32 {
    let mut height = GIT_FILE_HEADER_ESTIMATED_HEIGHT;

    if app.git_focused_file.as_deref() == Some(meta.path.as_str()) {
        height += GIT_FILE_FOCUSED_EXTRA_HEIGHT;
    }

    if !app.expanded_files_set.contains(&meta.path) {
        return height;
    }

    if app.git_diff_contents_loading.contains(&meta.path) {
        return height + GIT_FILE_LOADING_PLACEHOLDER_HEIGHT;
    }

    let estimated_lines = if let Some((old_content, new_content)) =
        app.git_diff_contents.get(&meta.path)
    {
        match meta.status {
            utils::FileStatus::Added | utils::FileStatus::Untracked => {
                new_content.lines().count().max(meta.insertions).max(1)
            }
            utils::FileStatus::Deleted => old_content.lines().count().max(meta.deletions).max(1),
            utils::FileStatus::Modified | utils::FileStatus::Renamed => {
                let changed_lines = meta.insertions.saturating_add(meta.deletions);
                let expanded_context =
                    changed_lines.saturating_add(desktop_git::DIFF_CONTEXT.saturating_mul(8));
                expanded_context
                    .max(12)
                    .min(old_content.lines().count().saturating_add(new_content.lines().count()))
            }
            utils::FileStatus::Unknown => 12,
        }
    } else {
        return height + GIT_FILE_LOADING_PLACEHOLDER_HEIGHT;
    };

    height + estimated_lines.max(1) as f32 * GIT_FILE_LINE_ESTIMATED_HEIGHT + 12.0
}

pub fn compute_git_file_virtual_window(app: &FileListInput<'_>, file_heights: &[f32]) -> (usize, usize, f32, f32) {
    if file_heights.is_empty() {
        return (0, 0, 0.0, 0.0);
    }

    let viewport_h = app.git_diff_scroll_viewport_h.max(0.0);
    if viewport_h <= 0.0 {
        return (0, file_heights.len(), 0.0, 0.0);
    }

    let total_height = sum_git_file_block_height(file_heights);
    let max_scroll = (total_height - viewport_h).max(0.0);
    let scroll_top = app.git_diff_scroll_offset_y.clamp(0.0, 1.0) * max_scroll;
    let visible_top = scroll_top;
    let visible_bottom = (scroll_top + viewport_h).min(total_height);

    let mut start = 0usize;
    let mut cursor = 0.0f32;
    while start < file_heights.len() {
        let item_bottom = cursor + file_heights[start];
        if item_bottom >= visible_top {
            break;
        }
        cursor = item_bottom + GIT_FILE_CARD_GAP;
        start += 1;
    }

    let mut end = start;
    let mut render_cursor = cursor;
    while end < file_heights.len() {
        let item_bottom = render_cursor + file_heights[end];
        end += 1;
        if item_bottom >= visible_bottom {
            break;
        }
        render_cursor = item_bottom + GIT_FILE_CARD_GAP;
    }

    let start = start.saturating_sub(GIT_FILE_VIRTUALIZATION_BUFFER);
    let end = (end + GIT_FILE_VIRTUALIZATION_BUFFER).min(file_heights.len());
    let end = end.max(start.saturating_add(1)).min(file_heights.len());
    let top_spacer = sum_git_file_block_height(&file_heights[..start]);
    let bottom_spacer = sum_git_file_block_height(&file_heights[end..]);

    (start, end, top_spacer, bottom_spacer)
}



#[cfg(test)]
#[path = "panel_tests.rs"]
mod tests;
