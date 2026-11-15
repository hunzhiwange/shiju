//! 原生窗口恢复；调用由 Iced UI 线程调度。

use iced::advanced::graphics::futures::MaybeSend;
use iced::{Task, window};

pub fn show_main_window<T: MaybeSend + 'static>(id: window::Id) -> Task<T> {
    let unhide = window::run(id, |_| {
        let mtm = objc2::MainThreadMarker::new().expect("window callback runs on UI thread");
        objc2_app_kit::NSApplication::sharedApplication(mtm).unhide(None);
    })
    .discard();
    unhide
        .chain(window::minimize(id, false))
        .chain(window::set_mode(id, window::Mode::Windowed))
        .chain(window::gain_focus(id))
}
