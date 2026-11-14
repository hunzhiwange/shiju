//! 保留原生窗口的恢复入口；macOS 隐藏应用，让 Dock 激活可自然恢复窗口。

use iced::advanced::graphics::futures::MaybeSend;
use iced::{Task, window};

impl crate::app::App {
    /// 在主窗口创建完成后恢复本机窗口位置；其它窗口不共享保存名称。
    pub fn restore_main_window_state(&mut self) -> Task<crate::app::Message> {
        #[cfg(target_os = "macos")]
        {
            let Some(id) = self.main_window_id else {
                return Task::none();
            };
            // 原生恢复负责适配屏幕，避免首次宽屏缩放覆盖用户保存的尺寸。
            self.startup_resize_checked = true;
            window::run(id, |handle| {
                if let Err(error) = restore_native_frame(handle) {
                    tracing::warn!(%error, "恢复主窗口位置失败");
                }
            })
            .discard()
        }
        #[cfg(not(target_os = "macos"))]
        Task::none()
    }
}

#[cfg(target_os = "macos")]
fn restore_native_frame(handle: &dyn window::Window) -> Result<(), &'static str> {
    use iced::window::raw_window_handle::RawWindowHandle;
    use objc2_app_kit::{NSScreen, NSView};
    use objc2_foundation::NSString;

    let mtm = objc2::MainThreadMarker::new().ok_or("窗口恢复不在主线程")?;
    let raw = handle.window_handle().map_err(|_| "无法读取原生窗口句柄")?;
    let RawWindowHandle::AppKit(raw) = raw.as_raw() else {
        return Err("主窗口不是 AppKit 窗口");
    };
    // SAFETY: Iced 在 UI 线程回调期间持有窗口，AppKit 句柄指向有效 NSView。
    let view = unsafe { raw.ns_view.cast::<NSView>().as_ref() };
    let native = view.window().ok_or("原生视图尚未关联窗口")?;
    let name = NSString::from_str(if cfg!(debug_assertions) {
        "ShiJu-dev.main-window"
    } else {
        "ShiJu.main-window"
    });
    // 先读取旧值，再启用自动保存，避免初始窗口覆盖已保存的 frame。
    native.setFrameUsingName_force(&name, true);
    let frame = native.frame();
    let screens = NSScreen::screens(mtm);
    let mut screen = NSScreen::mainScreen(mtm).ok_or("没有可用屏幕")?;
    let mut largest_area = 0.0_f64;
    for index in 0..screens.count() {
        let candidate = screens.objectAtIndex(index);
        let visible = candidate.visibleFrame();
        let width = (frame.origin.x + frame.size.width)
            .min(visible.origin.x + visible.size.width)
            - frame.origin.x.max(visible.origin.x);
        let height = (frame.origin.y + frame.size.height)
            .min(visible.origin.y + visible.size.height)
            - frame.origin.y.max(visible.origin.y);
        let area = width.max(0.0) * height.max(0.0);
        if area > largest_area {
            largest_area = area;
            screen = candidate;
        }
    }
    let corrected = constrain_saved_frame(frame, screen.visibleFrame());
    if !native.setFrameAutosaveName(&name) {
        return Err("主窗口保存名称已被占用");
    }
    native.setFrame_display(corrected, true);
    native.saveFrameUsingName(&name);
    Ok(())
}

#[cfg(target_os = "macos")]
fn constrain_saved_frame(
    mut frame: objc2_foundation::NSRect,
    visible: objc2_foundation::NSRect,
) -> objc2_foundation::NSRect {
    if !frame.origin.x.is_finite()
        || !frame.origin.y.is_finite()
        || !frame.size.width.is_finite()
        || !frame.size.height.is_finite()
        || frame.size.width <= 0.0
        || frame.size.height <= 0.0
    {
        return visible;
    }
    frame.size.width = frame.size.width.min(visible.size.width);
    frame.size.height = frame.size.height.min(visible.size.height);
    frame.origin.x = frame.origin.x.clamp(
        visible.origin.x,
        visible.origin.x + visible.size.width - frame.size.width,
    );
    frame.origin.y = frame.origin.y.clamp(
        visible.origin.y,
        visible.origin.y + visible.size.height - frame.size.height,
    );
    frame
}

#[cfg(all(test, target_os = "macos"))]
#[path = "../desktop_integration_25_tests.rs"]
mod tests;

pub(crate) fn hide_main_window<T: MaybeSend + 'static>(id: window::Id) -> Task<T> {
    #[cfg(target_os = "macos")]
    {
        window::run(id, |_| {
            let mtm = objc2::MainThreadMarker::new().expect("window callback runs on UI thread");
            objc2_app_kit::NSApplication::sharedApplication(mtm).hide(None);
        })
        .discard()
    }
    #[cfg(not(target_os = "macos"))]
    window::set_mode(id, window::Mode::Hidden)
}

pub(crate) fn show_main_window<T: MaybeSend + 'static>(id: window::Id) -> Task<T> {
    #[cfg(target_os = "macos")]
    let unhide = window::run(id, |_| {
        let mtm = objc2::MainThreadMarker::new().expect("window callback runs on UI thread");
        objc2_app_kit::NSApplication::sharedApplication(mtm).unhide(None);
    })
    .discard();
    #[cfg(not(target_os = "macos"))]
    let unhide = Task::none();

    unhide
        .chain(window::minimize(id, false))
        .chain(window::set_mode(id, window::Mode::Windowed))
        .chain(window::gain_focus(id))
}
