#[test]
fn task_645_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("svg_icons_tests.rs"));
}

#[test]
fn get_icon_returns_handles_from_every_svg_registry_group() {
    for icon in [
        super::Icon::Save,
        super::Icon::QuestionCircle,
        super::Icon::Cursor,
        super::Icon::Rust,
        super::Icon::AppCodex,
    ] {
        std::hint::black_box(super::get_icon(icon));
    }
}

#[test]
#[should_panic(expected = "Icon missing in assets map")]
fn get_icon_panics_for_png_only_icon() {
    let _ = super::get_icon(super::Icon::AppFinder);
}
