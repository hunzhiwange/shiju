//! Pointer geometry shared by desktop layout adapters.
pub const MIN_WORK_SIDEBAR_WIDTH: f32 = 220.0;
pub const MAX_WORK_SIDEBAR_WIDTH: f32 = 520.0;
pub const MIN_WORK_SPLIT_RATIO: f32 = 0.25;
pub const MAX_WORK_SPLIT_RATIO: f32 = 0.75;

/// Capture the first pointer position; subsequent positions resize within the given bounds.
pub fn drag_value(anchor: &mut Option<f32>, position: f32, start: f32, scale: f32, min: f32, max: f32) -> Option<f32> {
    let Some(origin) = *anchor else {
        *anchor = Some(position);
        return None;
    };
    Some((start + (position - origin) / scale).clamp(min, max))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalCommand { Search, Copy }

pub fn global_command(key: &iced::keyboard::Key, modifiers: iced::keyboard::Modifiers) -> Option<GlobalCommand> {
    if !modifiers.command() { return None; }
    match key {
        iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("p") => Some(GlobalCommand::Search),
        iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("c") => Some(GlobalCommand::Copy),
        _ => None,
    }
}

#[cfg(test)]
#[path = "view_layout_tests.rs"]
mod tests;
