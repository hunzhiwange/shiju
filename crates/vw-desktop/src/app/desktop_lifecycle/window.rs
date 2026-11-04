//! 保留原生窗口的恢复入口；macOS 隐藏应用，让 Dock 激活可自然恢复窗口。

use iced::advanced::graphics::futures::MaybeSend;
use iced::{Task, window};

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
