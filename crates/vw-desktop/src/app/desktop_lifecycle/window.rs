//! 原生窗口的位置恢复、轻量释放与托盘重建入口。

use iced::advanced::graphics::futures::MaybeSend;
use iced::{Task, window};

impl crate::app::App {
    /// macOS 轻量模式释放原生窗口，页面模型与订阅仍由 App 持有。
    #[cfg(target_os = "macos")]
    pub(crate) fn release_main_window(&mut self) -> Task<crate::app::Message> {
        let Some(id) = self.main_window_id.take() else {
            return Task::none();
        };
        self.released_main_windows.push(id);
        window::close(id)
    }

    pub(crate) fn reopen_main_window(&mut self) -> Task<crate::app::Message> {
        if let Some(id) = self.main_window_id {
            return show_main_window(id);
        }
        #[cfg(target_os = "macos")]
        {
            if self.shutdown_requested {
                return Task::none();
            }
            let (id, open) = window::open(window::Settings {
                size: iced::Size::new(1720.0, 1000.0),
                position: window::Position::Centered,
                exit_on_close_request: false,
                platform_specific: window::settings::PlatformSpecific {
                    title_hidden: true,
                    titlebar_transparent: true,
                    fullsize_content_view: true,
                },
                ..Default::default()
            });
            self.main_window_id = Some(id);
            let restore = self.restore_main_window_state();
            return open.discard().chain(restore).chain(show_main_window(id));
        }
        #[cfg(not(target_os = "macos"))]
        Task::none()
    }

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
                if let Err(error) = desktop_platform::window::restore_native_frame(handle) {
                    tracing::warn!(%error, "恢复主窗口位置失败");
                }
            })
            .discard()
        }
        #[cfg(not(target_os = "macos"))]
        Task::none()
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn hide_main_window<T: MaybeSend + 'static>(id: window::Id) -> Task<T> {
    window::set_mode(id, window::Mode::Hidden)
}

pub(crate) fn show_main_window<T: MaybeSend + 'static>(id: window::Id) -> Task<T> {
    #[cfg(target_os = "macos")]
    return desktop_platform::window::show_main_window(id);
    #[cfg(not(target_os = "macos"))]
    {
        Task::none()
            .chain(window::minimize(id, false))
            .chain(window::set_mode(id, window::Mode::Windowed))
            .chain(window::gain_focus(id))
    }
}
