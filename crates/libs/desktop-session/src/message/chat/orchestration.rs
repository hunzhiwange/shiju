//! 聊天增量后的局部状态计算，不持有桌面应用。
pub const STREAM_AUTOSCROLL_THROTTLE_MS: u64 = 120;
pub const AUTOSCROLL_HOLD_MS: u64 = 280;

pub fn arm_autoscroll_hold(generation: &mut u64, auto_scroll: &mut bool, hold_until: &mut u64, now: u64) {
    *generation = generation.wrapping_add(1);
    *auto_scroll = true;
    *hold_until = now.saturating_add(AUTOSCROLL_HOLD_MS);
}

pub fn stream_autoscroll_due(auto_scroll: bool, last_ms: &mut u64, now: u64) -> bool {
    if !auto_scroll || now.saturating_sub(*last_ms) < STREAM_AUTOSCROLL_THROTTLE_MS {
        return false;
    }
    *last_ms = now;
    true
}

pub fn think_block_key(msg_idx: usize, think_idx: usize) -> u64 {
    ((msg_idx as u64) << 32) | (think_idx as u64)
}

pub struct ThinkSyncOutcome {
    pub removed_indexes: std::ops::Range<usize>,
    pub scroll_to_open_block: bool,
}

pub fn sync_stream_think_tracking(
    tracked_message: &mut Option<usize>,
    tracked_count: &mut usize,
    tracked_open: &mut Option<usize>,
    message_index: usize,
    count: usize,
    thinking_open: bool,
) -> ThinkSyncOutcome {
    let open_index = if thinking_open && count > 0 { Some(count.saturating_sub(1)) } else { None };
    let same_message = *tracked_message == Some(message_index);
    let previous_open = if same_message { *tracked_open } else { None };
    let previous_count = if same_message { *tracked_count } else { 0 };
    *tracked_message = Some(message_index);
    *tracked_count = count;
    *tracked_open = open_index;
    ThinkSyncOutcome {
        removed_indexes: count..previous_count,
        scroll_to_open_block: (previous_open != open_index || previous_count != count) && open_index.is_some(),
    }
}

pub fn fork_session_anchor(
    chat: &[session_ui_types::ChatMessage],
    message_ids: &[Option<String>],
    index: usize,
    session_id: Option<&str>,
    directory: Option<String>,
    open_generation: u64,
) -> Option<super::fork::ForkSessionAnchor> {
    if chat.get(index)?.role != session_ui_types::ChatRole::User || message_ids.len() != chat.len() {
        return None;
    }
    let session_id = session_id?;
    let directory = directory?;
    let user_message_id = message_ids.get(index)?.as_ref()?;
    if directory.trim().is_empty() || user_message_id.trim().is_empty() {
        return None;
    }
    Some(super::fork::ForkSessionAnchor {
        session_id: session_id.to_owned(),
        directory,
        user_message_id: user_message_id.clone(),
        open_generation,
    })
}

pub fn fork_session_anchor_is_current(
    anchor: &super::fork::ForkSessionAnchor,
    active_session_id: Option<&str>,
    open_generation: u64,
    directory: Option<&str>,
) -> bool {
    active_session_id == Some(anchor.session_id.as_str())
        && open_generation == anchor.open_generation
        && directory == Some(anchor.directory.as_str())
}
