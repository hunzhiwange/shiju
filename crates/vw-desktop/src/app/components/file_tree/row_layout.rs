//! 文件树、变更树与原位编辑共享的层级尺寸。
use iced::widget::{Row, Space, container, row};
use iced::{Alignment, Element, Length};

use crate::app::Message;
use crate::app::assets::Icon;

pub(super) const ROW_PADDING: [u16; 2] = [2, 5];
pub(super) const INDENT: f32 = 20.0;
const ICON_SLOT: f32 = 16.0;
const NAME_GAP: f32 = 4.0;

pub(super) fn folder_icon(expanded: bool) -> Icon {
    if expanded { Icon::FolderOpen } else { Icon::FolderClosed }
}

pub(super) fn content<'a>(
    depth: usize,
    icon: impl Into<Element<'a, Message>>,
    name: impl Into<Element<'a, Message>>,
) -> Row<'a, Message> {
    row![
        Space::new().width(Length::Fixed(depth as f32 * INDENT)),
        row![container(icon).width(Length::Fixed(ICON_SLOT)), name.into()]
            .spacing(NAME_GAP)
            .align_y(Alignment::Center)
            .width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill)
}
