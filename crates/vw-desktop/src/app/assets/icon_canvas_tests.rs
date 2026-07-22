#[test]
fn task_637_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("icon_canvas_tests.rs"));
}

#[test]
fn register_icons_populates_canvas_icon_group_without_clearing_existing_entries() {
    use std::collections::HashMap;

    use iced::widget::svg;

    use super::Icon;

    let mut icons = HashMap::new();
    icons.insert(
        Icon::Logo,
        svg::Handle::from_memory(br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#),
    );

    super::register_icons(&mut icons);

    for icon in [
        Icon::Cursor,
        Icon::Arrows,
        Icon::ArrowsMove,
        Icon::ArrowsFullscreen,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::ListUl,
        Icon::Bezier,
        Icon::Link,
        Icon::Link45Deg,
        Icon::ChevronRight,
        Icon::ChevronLeft,
        Icon::ChevronDown,
        Icon::Trash,
        Icon::Square,
        Icon::SquareHalf,
        Icon::Plus,
        Icon::Pencil,
        Icon::Circle,
        Icon::Star,
        Icon::Triangle,
        Icon::Diamond,
        Icon::Pentagon,
        Icon::Hexagon,
        Icon::Capsule,
        Icon::Parallelogram,
        Icon::Trapezoid,
        Icon::Speedometer2,
        Icon::Image,
        Icon::CloudDownload,
        Icon::Pen,
        Icon::PaintBucket,
        Icon::Type,
        Icon::LayoutTextWindow,
        Icon::Fullscreen,
        Icon::FullscreenExit,
        Icon::FileText,
        Icon::HandIndex,
        Icon::Sliders,
        Icon::Palette,
        Icon::Keyboard,
        Icon::Gear,
        Icon::GearWideConnected,
        Icon::HddNetwork,
        Icon::Braces,
        Icon::Box,
        Icon::File,
        Icon::FolderOpen,
        Icon::FileEarmarkPlus,
        Icon::ChatTextFill,
        Icon::Journals,
        Icon::CloudUpload,
    ] {
        assert!(icons.contains_key(&icon), "missing registered canvas icon: {icon:?}");
    }
    assert!(icons.contains_key(&Icon::Logo));
}
