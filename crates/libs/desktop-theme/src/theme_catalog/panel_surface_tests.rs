use super::{PanelImageFit, with_panel_background};
use iced::widget::text;
use iced::{ContentFit, Element, Length};
use std::path::Path;

#[test]
fn panel_image_fit_parses_known_values_and_defaults() {
    assert_eq!(PanelImageFit::parse(None).unwrap(), PanelImageFit::Cover);
    assert_eq!(PanelImageFit::parse(Some("")).unwrap(), PanelImageFit::Cover);
    assert_eq!(PanelImageFit::parse(Some("cover")).unwrap(), PanelImageFit::Cover);
    assert_eq!(PanelImageFit::parse(Some("Contain")).unwrap(), PanelImageFit::Contain);
    assert_eq!(PanelImageFit::parse(Some("fill")).unwrap(), PanelImageFit::Fill);
    assert_eq!(PanelImageFit::parse(Some("scale_down")).unwrap(), PanelImageFit::ScaleDown);
    assert_eq!(PanelImageFit::parse(Some("none")).unwrap(), PanelImageFit::None);
    assert!(PanelImageFit::parse(Some("stretch")).is_err());
}

#[test]
fn panel_image_fit_maps_to_iced_content_fit() {
    assert_eq!(PanelImageFit::Cover.to_content_fit(), ContentFit::Cover);
    assert_eq!(PanelImageFit::Contain.to_content_fit(), ContentFit::Contain);
    assert_eq!(PanelImageFit::Fill.to_content_fit(), ContentFit::Fill);
    assert_eq!(PanelImageFit::ScaleDown.to_content_fit(), ContentFit::ScaleDown);
    assert_eq!(PanelImageFit::None.to_content_fit(), ContentFit::None);
}

#[test]
fn with_panel_background_without_file_returns_content() {
    let content: Element<'_, ()> = text("hello").into();
    let wrapped = with_panel_background(
        content,
        Some(Path::new("/no/such/panel-bg.png")),
        PanelImageFit::Cover,
    );
    // 缺文件时不应包一层；烟雾构建即可。
    let _ = wrapped;
}

#[test]
fn with_panel_background_stack_keeps_content_driven_height_hint() {
    // 有图时用 Stack::push + push_under：高度由内容决定，不强制 Fill。
    let content: Element<'_, ()> = text("card").height(Length::Shrink).into();
    let path = std::env::temp_dir().join("vibe-window-panel-bg-fit-test.png");
    // 最小 1x1 PNG
    let png: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8,
        0xff, 0xff, 0x3f, 0x00, 0x05, 0xfe, 0x02, 0xfe, 0xdc, 0xcc, 0x59, 0xe7, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    std::fs::write(&path, png).expect("write png");
    let wrapped = with_panel_background(content, Some(path.as_path()), PanelImageFit::Cover);
    let size = iced::advanced::Widget::size(wrapped.as_widget());
    assert_ne!(
        size.height,
        Length::Fill,
        "panel background must not force Fill height (that expands input/header)"
    );
    let _ = std::fs::remove_file(&path);
}
