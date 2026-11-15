//! Native companion window presentation settings.
#[cfg(not(target_arch = "wasm32"))]
pub fn task_pet_window_settings(size: iced::Size, position: iced::Point) -> iced::window::Settings {
    iced::window::Settings {
        size,
        // 禁止用户缩放，但允许展开动画和伙伴大小设置调整原生窗口。
        min_size: None,
        max_size: None,
        position: iced::window::Position::Specific(position),
        resizable: false,
        decorations: false,
        transparent: true,
        level: iced::window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        ..Default::default()
    }
}


#[cfg(test)]
mod tests;

/// Begin a size draft without changing the persisted scale.
pub fn open_size(draft: &mut Option<f32>, dragging: &mut bool, scale: f32) -> bool {
    if draft.is_some() { return false; }
    *draft = Some(scale);
    *dragging = false;
    true
}

pub fn preview_size(
    draft: &mut Option<f32>,
    pending: bool,
    avatar: crate::pet_state::TaskPetAvatarKind,
    scale: f32,
) -> Result<bool, &'static str> {
    if draft.is_none() || pending || avatar == crate::pet_state::TaskPetAvatarKind::None {
        return Ok(false);
    }
    if !scale.is_finite() { return Err("伙伴大小必须是有限数值"); }
    *draft = Some(scale.clamp(0.5, 2.0));
    Ok(true)
}

pub fn cancel_size(draft: &mut Option<f32>, pending: bool) -> bool {
    if pending { return false; }
    *draft = None;
    true
}

pub enum SizeSave { Idle, Cancel, Save(f32) }

pub fn size_save(draft: Option<f32>, pending: bool, scale: f32) -> SizeSave {
    if pending { return SizeSave::Idle; }
    match draft {
        None => SizeSave::Idle,
        Some(value) if value == scale => SizeSave::Cancel,
        Some(value) => SizeSave::Save(value),
    }
}
