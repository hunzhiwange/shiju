//! 聊天块展开状态的局部更新，不触碰窗口或测量缓存。
use std::collections::HashSet;

pub fn toggle_think(expanded: &mut HashSet<u64>, collapsed: &mut HashSet<u64>, message_index: usize, think_index: usize, default_expanded: bool) -> bool {
    let key = ((message_index as u64) << 32) | (think_index as u64);
    let had_expanded_override = expanded.remove(&key);
    let had_collapsed_override = collapsed.remove(&key);
    if had_expanded_override || had_collapsed_override {
        default_expanded
    } else if default_expanded {
        collapsed.insert(key);
        false
    } else {
        expanded.insert(key);
        true
    }
}

pub fn toggle_tool_file(expanded: &mut HashSet<String>, message_index: usize, tool_index: usize, path: &str) {
    let key = format!("{message_index}:{tool_index}:{path}");
    if !expanded.remove(&key) {
        expanded.insert(key);
    }
}

pub fn toggle_tool_block(expanded: &mut HashSet<u64>, message_index: usize, tool_index: usize) {
    let key = ((message_index as u64) << 32) | (tool_index as u64);
    if !expanded.remove(&key) {
        expanded.insert(key);
    }
}

/// Returns whether another animation frame is needed.
pub fn toggle_todo_panel(expanded: &mut bool, animation: &mut f32) -> bool {
    *expanded = !*expanded;
    let target = if *expanded { 1.0 } else { 0.0 };
    if (*animation - target).abs() < 0.001 {
        *animation = target;
        false
    } else {
        true
    }
}

pub fn tick_todo_panel(expanded: bool, animation: &mut f32) -> bool {
    let target = if expanded { 1.0 } else { 0.0 };
    let next = *animation + (target - *animation) * 0.22;
    let done = (next - target).abs() < 0.005;
    *animation = if done { target } else { next.clamp(0.0, 1.0) };
    !done
}

pub fn toggle_user_message(expanded: &mut HashSet<usize>, index: usize) {
    if !expanded.remove(&index) {
        expanded.insert(index);
    }
}

pub fn block_key(message_index: usize, block_index: usize) -> u64 {
    ((message_index as u64) << 32) | (block_index as u64)
}

pub fn show_todo_panel(expanded: &mut bool, animation: &mut f32) {
    *expanded = true;
    *animation = 1.0;
}

pub fn git_quick_action_path(repo_path: Option<String>, project_path: Option<&str>) -> Option<String> {
    repo_path.or_else(|| project_path.map(str::to_owned)).filter(|path| !path.trim().is_empty())
}
