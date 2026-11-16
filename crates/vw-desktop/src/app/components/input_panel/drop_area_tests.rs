use super::drop_area::{DropArea, DropAreaElement};
use crate::app::Message;
// 宿主保留：宿主 Message 与 DropAreaElement 类型别名装配契约。
#[test]
fn drop_area_can_be_wrapped_as_app_alias() {
    let content: iced::Element<'static, Message> = iced::widget::text("app element").into();
    let _: DropAreaElement<'static> = DropArea::new(content, Message::None, None, false);
}
