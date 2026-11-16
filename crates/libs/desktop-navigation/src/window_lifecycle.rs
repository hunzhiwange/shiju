#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowClosedOutcome {
    Continue,
    Exit,
}

pub fn mark_window_closed(
    main_window_id: &mut Option<iced::window::Id>,
    task_pet_window_id: &mut Option<iced::window::Id>,
    closed_window_id: iced::window::Id,
) -> WindowClosedOutcome {
    if *main_window_id == Some(closed_window_id) {
        *main_window_id = None;
        *task_pet_window_id = None;
        return WindowClosedOutcome::Exit;
    }

    if *task_pet_window_id == Some(closed_window_id) {
        *task_pet_window_id = None;
    }

    if main_window_id.is_none() && task_pet_window_id.is_none() {
        WindowClosedOutcome::Exit
    } else {
        WindowClosedOutcome::Continue
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 关闭请求中需要分别处理的主窗口与任务宠物窗口。
pub struct CloseRequestedWindows {
    /// 发起关闭的主窗口；若请求来自宠物窗口则为空。
    pub main_window_id: Option<iced::window::Id>,
    /// 发起关闭的任务宠物窗口；关闭它不会退出主应用。
    pub task_pet_window_id: Option<iced::window::Id>,
}

/// 将窗口关闭请求分类为退出应用或仅关闭任务宠物。
pub fn close_requested_windows(
    main_window_id: Option<iced::window::Id>,
    task_pet_window_id: Option<iced::window::Id>,
    requested_window_id: iced::window::Id,
) -> CloseRequestedWindows {
    if main_window_id == Some(requested_window_id) {
        return CloseRequestedWindows {
            main_window_id,
            task_pet_window_id: task_pet_window_id.filter(|id| *id != requested_window_id),
        };
    }

    if task_pet_window_id == Some(requested_window_id) {
        return CloseRequestedWindows { main_window_id: None, task_pet_window_id };
    }

    CloseRequestedWindows { main_window_id: None, task_pet_window_id: None }
}

/// 原子式标记关闭流程；已开始关闭时返回 `false` 防止重复清理。
pub fn begin_shutdown(shutdown_requested: &mut bool) -> bool {
    if *shutdown_requested {
        return false;
    }
    *shutdown_requested = true;
    true
}

/// Normalize the main-window geometry and return the one-shot startup correction.
pub fn resize_main_window(
    main: Option<iced::window::Id>,
    window: iced::window::Id,
    size: &mut (f32, f32),
    startup_checked: &mut bool,
    blocked: bool,
    width: f32,
    height: f32,
) -> Option<iced::Size> {
    if main != Some(window) {
        return None;
    }
    let width = width.max(1.0);
    let height = height.max(1.0);
    *size = (width, height);
    if blocked || *startup_checked {
        return None;
    }
    *startup_checked = true;
    #[cfg(not(target_arch = "wasm32"))]
    if width > height * 2.0 {
        return Some(iced::Size::new(width * 0.5, height));
    }
    None
}

/// Finish the native layout-settling interval.
pub fn settle_fullscreen_layout(settling: &mut bool) { *settling = false; }
