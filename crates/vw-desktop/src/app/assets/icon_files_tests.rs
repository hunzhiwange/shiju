#[test]
fn task_638_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("icon_files_tests.rs"));
}

#[test]
fn register_icons_inserts_every_file_icon_without_clearing_existing_entries() {
    use super::Icon;
    use iced::widget::svg;
    use std::collections::HashMap;

    let sentinel = Icon::QuestionCircle;
    let sentinel_handle = svg::Handle::from_memory(b"<svg></svg>");
    let mut icons = HashMap::from([(sentinel, sentinel_handle)]);

    super::register_icons(&mut icons);

    for icon in [
        Icon::Rust,
        Icon::Typescript,
        Icon::Javascript,
        Icon::Json,
        Icon::Toml,
        Icon::Yaml,
        Icon::Markdown,
        Icon::Html,
        Icon::Css,
        Icon::Python,
        Icon::Go,
        Icon::Console,
        Icon::Document,
        Icon::QrCode,
        Icon::Scissors,
        Icon::Copy,
    ] {
        assert!(icons.contains_key(&icon), "{icon:?} should be registered");
    }

    assert!(icons.contains_key(&sentinel));
    assert_eq!(icons.len(), 17);
}
