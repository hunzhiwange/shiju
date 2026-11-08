//! 定位设置行，并在当前内容区滚动到对应控件。

use super::SETTINGS_CONTENT_SCROLL;
use crate::app::Message;
use iced::advanced::widget::{Id, Operation, operate, operation};
use iced::{Rectangle, Task, Vector};

pub(crate) fn reveal_field(id: &'static str) -> Task<Message> {
    operate(RevealField {
        target: id.into(),
        scroll_id: SETTINGS_CONTENT_SCROLL.into(),
        field_y: None,
        content_y: None,
    })
    .discard()
}

struct RevealField {
    target: Id,
    scroll_id: Id,
    field_y: Option<f32>,
    content_y: Option<f32>,
}

impl Operation for RevealField {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.target) {
            self.field_y = Some(bounds.y);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        _bounds: Rectangle,
        content: Rectangle,
        _translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&self.scroll_id) {
            self.content_y = Some(content.y);
        }
    }

    fn finish(&self) -> operation::Outcome<()> {
        let (Some(field_y), Some(content_y)) = (self.field_y, self.content_y) else {
            return operation::Outcome::None;
        };
        let y = (field_y - content_y - 12.0).max(0.0);
        operation::Outcome::Chain(Box::new(operation::scrollable::scroll_to(
            self.scroll_id.clone(),
            operation::scrollable::AbsoluteOffset { x: None, y: Some(y) },
        )))
    }
}
