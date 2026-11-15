pub mod task_panel;
pub mod selection;
pub mod s3_modal;
pub mod webdav_modal;
pub mod preview;

#[derive(Clone)]
pub enum EntryPoint { WebDav, S3 }
pub fn entry_points() -> iced::Element<'static, EntryPoint> {
    use iced::widget::{button, row, text};
    use desktop_ui_controls::system_settings_common::rounded_action_btn_style;
    row![
        button(text("WebDAV 同步")).on_press(EntryPoint::WebDav).padding([8, 14]).style(rounded_action_btn_style),
        button(text("S3 同步")).on_press(EntryPoint::S3).padding([8, 14]).style(rounded_action_btn_style),
    ].spacing(8).into()
}
