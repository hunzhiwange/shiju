//! 验证图标按钮构建器在各尺寸及缺失图标时的几何约束。
//! 占位图标用例用于锁定无资源时仍需保留的按钮宽度。

use iced::{Element, Length, Size};

use crate::app::Message;
use crate::app::assets::Icon;

use super::icon_buttons::{
    icon_button, medium_icon_button, small_icon_button, small_plain_icon_button,
    square_icon_button, square_icon_button_micro, square_icon_button_small,
    square_icon_button_tiny,
};

#[test]
fn icon_buttons_build_elements() {
    let _: Element<'static, Message> = icon_button(Icon::Plus, "plus".into(), Message::None);
    let _: Element<'static, Message> =
        square_icon_button(Icon::Trash, "trash".into(), Message::None);
    let _: Element<'static, Message> =
        square_icon_button_small(Icon::Image, "image".into(), Message::None);
    let _: Element<'static, Message> =
        square_icon_button_tiny(Icon::FileText, "file".into(), Message::None);
    let _: Element<'static, Message> =
        square_icon_button_micro(Icon::Gear, "gear".into(), Message::None);
}

#[test]
fn compact_icon_buttons_build_elements() {
    let _: Element<'static, Message> =
        medium_icon_button(Icon::CloudUpload, "upload".into(), Message::None);
    let _: Element<'static, Message> =
        small_icon_button(Icon::CloudDownload, "download".into(), Message::None);
    let _: Element<'static, Message> =
        small_plain_icon_button(Some(Icon::CheckSquare), "checked".into(), Message::None);
    let _: Element<'static, Message> =
        small_plain_icon_button(None, "empty slot".into(), Message::None);
}

#[test]
fn square_icon_buttons_preserve_size_specific_geometry() {
    let regular = square_icon_button(Icon::Trash, "regular".into(), Message::None);
    let small = square_icon_button_small(Icon::Image, "small".into(), Message::None);
    let tiny = square_icon_button_tiny(Icon::FileText, "tiny".into(), Message::None);
    let micro = square_icon_button_micro(Icon::Gear, "micro".into(), Message::None);

    for element in [&regular, &small] {
        assert_eq!(element.as_widget().size(), Size::new(Length::Fixed(32.0), Length::Fixed(32.0)));
        assert_eq!(element.as_widget().children().len(), 2);
    }
    assert_eq!(tiny.as_widget().size(), Size::new(Length::Fixed(28.0), Length::Fixed(28.0)));
    assert_eq!(micro.as_widget().size(), Size::new(Length::Fixed(24.0), Length::Fixed(24.0)));
}

#[test]
fn small_plain_icon_button_keeps_placeholder_width_without_icon() {
    let with_icon = small_plain_icon_button(Some(Icon::CheckSquare), "icon".into(), Message::None);
    let placeholder = small_plain_icon_button(None, String::new(), Message::None);

    for element in [&with_icon, &placeholder] {
        assert_eq!(element.as_widget().size().width, Length::Fixed(20.0));
        assert_eq!(element.as_widget().children().len(), 2);
    }
}
