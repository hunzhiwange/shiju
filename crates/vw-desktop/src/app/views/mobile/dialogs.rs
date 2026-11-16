//! 仅挂载项目会话局部浮层；认证、审批等全局浮层保留在根视图。
use crate::app::{App, Message};
use iced::Element;
pub fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = iced::widget::stack![base, super::super::project::new_session_picker_layer(app)].into();
    crate::app::components::input_panel::acp_options::with_modal(app, base)
}
