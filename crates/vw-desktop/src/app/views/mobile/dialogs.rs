//! 仅挂载项目会话局部浮层；认证、审批等全局浮层保留在根视图。
//! 审批与询问在移动导航之上统一挂载，底层卡片按实际布局空间收缩，
//! 此处不重复挂载审批与询问，以保持根视图的请求和取消消息链。
use crate::app::{App, Message};
use iced::Element;
pub fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = iced::widget::stack![base, super::super::project::new_session_picker_layer(app)].into();
    crate::app::components::input_panel::acp_options::with_modal(app, base)
}
