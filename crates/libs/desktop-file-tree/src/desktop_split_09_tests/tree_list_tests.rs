//! 验证文件树路径建模、展开键、选择优先级与变更列表视图。
//! 重复分隔符、根文件、嵌套目录和不同菜单来源用于约束树模型的规范化与隔离。

use super::ViewState;
use super::model::build_file_tree_model;
use super::tree_list::{build_changes_list, build_file_tree_list};
use iced::{Background, Theme};

#[test]
fn tree_list_builds_empty_states_with_and_without_project() {
    let app = ViewState::default();
    let _ = build_file_tree_list(app.inputs());

    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.file_tree_model = Some(build_file_tree_model("/tmp/demo", &[]));
    let _ = build_file_tree_list(app.inputs());
}

#[test]
fn tree_list_builds_fallback_model_with_expanded_selection_and_file_menu() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.file_tree_model = Some(build_file_tree_model("/tmp/demo", &[]));
    app.file_index = vec![
        "/tmp/demo/README.md".to_string(),
        "/tmp/demo/src/main.rs".to_string(),
        "/tmp/demo/src/nested/lib.rs".to_string(),
    ];
    app.file_tree_expanded_set.insert("src".to_string());
    app.file_tree_expanded_set.insert("src/nested".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/main.rs".to_string());
    app.file_tree_menu_source = Some("tree-file".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(12.0, 24.0));

    let _ = build_file_tree_list(app.inputs());
}

#[test]
fn tree_list_builds_cached_model_with_directory_menu() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    let files = vec!["/tmp/demo/src/main.rs".to_string()];
    app.file_tree_model = Some(build_file_tree_model("/tmp/demo", &files));
    app.file_tree_expanded_set.insert("src".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src".to_string());
    app.file_tree_menu_source = Some("tree-dir".to_string());

    let _ = build_file_tree_list(app.inputs());
}

#[test]
fn changes_list_builds_loading_no_changes_and_empty_path_states() {
    let mut app = ViewState::default();
    let _ = build_changes_list(app.inputs());

    app.git_changed_files_loading = true;
    let _ = build_changes_list(app.inputs());

    app.git_changed_files_loading = false;
    app.git_changed_files = vec![String::new(), "/".to_string()];
    let _ = build_changes_list(app.inputs());
}

#[test]
fn changes_list_builds_root_and_nested_files_with_directory_menu() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.git_changed_files =
        vec!["README.md".to_string(), "src/main.rs".to_string(), "src/nested/lib.rs".to_string()];
    app.file_tree_expanded_set.insert("chg:src".to_string());
    app.git_focused_file = Some("src/main.rs".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/nested".to_string());
    app.file_tree_menu_source = Some("changes-dir".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(8.0, 16.0));

    let _ = build_changes_list(app.inputs());
}

#[test]
fn changes_list_builds_expanded_file_menu_for_selected_file() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    app.git_changed_files = vec!["src/main.rs".to_string()];
    app.file_tree_expanded_set.insert("chg:src".to_string());
    app.git_focused_file = Some("src/main.rs".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/main.rs".to_string());
    app.file_tree_menu_source = Some("changes-file".to_string());

    let _ = build_changes_list(app.inputs());
}

#[test]
fn changes_list_builds_root_file_menu_without_project_path() {
    let mut app = ViewState::default();
    app.git_changed_files = vec!["README.md".to_string()];
    app.git_focused_file = Some("README.md".to_string());
    app.file_tree_menu_path = Some("README.md".to_string());
    app.file_tree_menu_source = Some("changes-file".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(4.0, 6.0));

    let element = build_changes_list(app.inputs());

    assert_eq!(element.as_widget().size().width, iced::Length::Fill);
}

#[test]
fn tree_lists_build_collapsed_directories_and_ignore_mismatched_menu_sources() {
    let mut app = ViewState::default();
    app.project_path = Some("/tmp/demo".to_string());
    let files = vec!["/tmp/demo/src/nested/lib.rs".to_string()];
    app.file_tree_model = Some(build_file_tree_model("/tmp/demo", &files));
    app.file_tree_menu_path = Some("/tmp/demo/src".to_string());
    app.file_tree_menu_source = Some("changes-dir".to_string());

    let _collapsed_tree = build_file_tree_list(app.inputs());
    drop(_collapsed_tree);

    app.git_changed_files = vec!["src/nested/lib.rs".to_string()];
    app.file_tree_menu_source = Some("tree-dir".to_string());

    let _collapsed_changes = build_changes_list(app.inputs());
}

pub(super) struct RowScene<'a> {
    pub(super) quads: Vec<(iced::advanced::renderer::Quad, Background)>,
    pub(super) ui:
        iced_runtime::user_interface::UserInterface<'a, crate::Message, Theme, iced::Renderer>,
    pub(super) labels: Vec<(String, iced::Rectangle)>,
    pub(super) inputs: Vec<iced::Rectangle>,
    pub(super) icons: Vec<(iced::advanced::svg::Handle, iced::Rectangle)>,
}

#[derive(Default)]
struct RowBounds {
    labels: Vec<(String, iced::Rectangle)>,
    inputs: Vec<iced::Rectangle>,
}

impl iced::advanced::widget::operation::Operation for RowBounds {
    fn traverse(
        &mut self,
        children: &mut dyn FnMut(&mut dyn iced::advanced::widget::operation::Operation),
    ) {
        children(self);
    }

    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: &mut dyn iced::advanced::widget::operation::text_input::TextInput,
    ) {
        self.inputs.push(bounds);
    }
}

impl<'a> RowScene<'a> {
    pub(super) fn new(
        element: iced::Element<'a, crate::Message>,
        renderer: &mut iced::Renderer,
        theme: &Theme,
    ) -> Self {
        use iced::advanced::Renderer as _;
        renderer.reset(iced::Rectangle::with_size(iced::Size::new(640.0, 700.0)));
        let mut ui = iced_runtime::user_interface::UserInterface::build(
            element,
            iced::Size::new(640.0, 700.0),
            iced_runtime::user_interface::Cache::new(),
            renderer,
        );
        let mut bounds = RowBounds::default();
        ui.operate(renderer, &mut bounds);
        ui.draw(
            renderer,
            theme,
            &iced::advanced::renderer::Style { text_color: theme.palette().text },
            iced::mouse::Cursor::Unavailable,
        );
        let iced::Renderer::Secondary(software) = renderer else {
            panic!("expected tiny-skia renderer")
        };
        let quads = software.layers().iter().flat_map(|layer| layer.quads.clone()).collect();
        let icons = software
            .layers()
            .iter()
            .flat_map(|layer| &layer.images)
            .filter_map(|image| {
                if let iced::advanced::graphics::Image::Vector { svg, bounds, .. } = image {
                    Some((svg.handle.clone(), *bounds))
                } else {
                    None
                }
            })
            .collect();
        // Rasterize the actual SVG/text layers too: Chinese shaping must survive draw.
        use iced::advanced::renderer::Headless as _;
        let pixels =
            renderer.screenshot(iced::Size::new(640, 700), 1.0, theme.palette().background);
        assert_eq!(pixels.len(), 640 * 700 * 4);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
        Self { ui, labels: bounds.labels, inputs: bounds.inputs, icons, quads }
    }

    pub(super) fn label(&self, name: &str) -> iced::Rectangle {
        self.labels
            .iter()
            .find(|(value, _)| value == name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .1
    }

    pub(super) fn icon(&self, name: &str) -> &(iced::advanced::svg::Handle, iced::Rectangle) {
        let label = self.label(name);
        self.icons
            .iter()
            .filter(|(_, bounds)| (bounds.center_y() - label.center_y()).abs() < 1.0)
            .min_by(|(_, a), (_, b)| (label.x - a.x).abs().total_cmp(&(label.x - b.x).abs()))
            .expect("row icon")
    }

    pub(super) fn click(
        &mut self,
        renderer: &mut iced::Renderer,
        name: &str,
        modifiers: iced::keyboard::Modifiers,
    ) -> Vec<crate::Message> {
        use iced::{Event, keyboard, mouse};
        let point = self.label(name).center();
        let mut messages = Vec::new();
        let _ = self.ui.update(
            &[
                Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        messages.retain(|message| !matches!(message, crate::Message::None));
        messages
    }
}

pub(super) fn alignment_app() -> ViewState {
    let mut app = ViewState::default();
    app.project_path = Some("/tree-fixture".into());
    app.git_changed_files =
        ["root.md", "sp6/原始视频.mov", "sp7/.shiju/配置.json", "closed/child.txt"]
            .map(str::to_owned)
            .to_vec();
    let files = app
        .git_changed_files
        .iter()
        .map(|path| format!("/tree-fixture/{path}"))
        .collect::<Vec<_>>();
    app.file_tree_model = Some(build_file_tree_model("/tree-fixture", &files));
    for key in ["sp6", "sp7", "sp7/.shiju", "chg:sp6", "chg:sp7", "chg:sp7/.shiju"] {
        app.file_tree_expanded_set.insert(key.into());
    }
    app
}

#[tokio::test]
async fn tree_rows_align_siblings_and_indent_children() {
    let mut renderer = super::renderer().await;
    let app = alignment_app();
    for theme in [Theme::Light, Theme::Dark] {
        for element in [build_file_tree_list(app.inputs()), build_changes_list(app.inputs())] {
            let scene = RowScene::new(element, &mut renderer, &theme);
            let root = scene.label("root.md");
            for name in ["sp6", "sp7", "closed"] {
                assert_eq!(scene.label(name).x, root.x, "same-depth name: {name}");
                assert_eq!(
                    scene.icon(name).1.x,
                    scene.icon("root.md").1.x,
                    "same-depth icon: {name}"
                );
            }
            assert_eq!(scene.label("原始视频.mov").x - root.x, 20.0);
            assert_eq!(scene.label(".shiju").x - root.x, 20.0);
            assert_eq!(scene.label("配置.json").x - root.x, 40.0);
            for name in ["root.md", "sp6", "原始视频.mov", "配置.json"] {
                let icon = scene.icon(name).1;
                assert_eq!(scene.label(name).x - icon.x, 20.0);
                assert!((scene.label(name).center_y() - icon.center_y()).abs() < 1.0);
            }
        }
    }
}

#[tokio::test]
async fn tree_rows_use_distinct_folder_states_without_chevrons() {
    let mut renderer = super::renderer().await;
    let app = alignment_app();
    for element in [build_file_tree_list(app.inputs()), build_changes_list(app.inputs())] {
        let scene = RowScene::new(element, &mut renderer, &Theme::Dark);
        assert_eq!(scene.icons.len(), 7, "exactly one icon per visible row");
        assert_ne!(
            scene.icon("closed").0,
            scene.icon("sp6").0,
            "closed and open folders must differ"
        );
        assert_eq!(
            scene.icon("sp6").0,
            desktop_assets::assets::get_icon(desktop_assets::assets::Icon::FolderOpen)
        );
    }
}

#[tokio::test]
async fn tree_rows_emit_selection_open_and_toggle_messages() {
    use crate::Message;
    use crate::ProjectMessage;
    use iced::keyboard::Modifiers;
    let mut renderer = super::renderer().await;
    let app = alignment_app();
    for theme in [Theme::Light, Theme::Dark] {
        for modifiers in [Modifiers::NONE, Modifiers::CTRL, Modifiers::SHIFT, Modifiers::LOGO] {
            let mut scene =
                RowScene::new(build_file_tree_list(app.inputs()), &mut renderer, &theme);
            let messages = scene.click(&mut renderer, "原始视频.mov", modifiers);
            assert!(messages.iter().any(|message| matches!(message,
                Message::Project(ProjectMessage::FileTreeSelected { path, directory: false, modifiers: actual, .. })
                    if path == "/tree-fixture/sp6/原始视频.mov" && *actual == modifiers
            )));
        }
        let mut scene = RowScene::new(build_changes_list(app.inputs()), &mut renderer, &theme);
        let messages = scene.click(&mut renderer, "原始视频.mov", Modifiers::NONE);
        assert!(messages.iter().any(|message| matches!(message,
            Message::Project(ProjectMessage::OpenChangedFile(path)) if path == "sp6/原始视频.mov"
        )));
        drop(scene);
        for changes in [false, true] {
            let element = if changes {
                build_changes_list(app.inputs())
            } else {
                build_file_tree_list(app.inputs())
            };
            let mut scene = RowScene::new(element, &mut renderer, &theme);
            let messages = scene.click(&mut renderer, "sp6", Modifiers::NONE);
            drop(scene);
            let toggle = messages.into_iter().find(|message| if changes {
                matches!(message, Message::Project(ProjectMessage::ToggleTreeDir(key)) if key == "chg:sp6")
            } else {
                matches!(message, Message::Project(ProjectMessage::FileTreeSelected { path, directory: true, .. }) if path == "/tree-fixture/sp6")
            }).expect("directory toggle message");
            assert!(matches!(toggle, Message::Project(_)));
        }
    }
}

#[tokio::test]
async fn tree_gutter_tracks_expansion_in_both_lists() {
    let mut renderer = super::renderer().await;
    for changes in [false, true] {
        let mut app = alignment_app();
        let build = if changes { build_changes_list } else { build_file_tree_list };
        let scene = RowScene::new(build(app.inputs()), &mut renderer, &Theme::Dark);
        let child = scene.label("原始视频.mov");
        let parent = scene.icon("sp6").1;
        assert!(
            scene.quads.iter().any(|(quad, _)| quad.bounds.width == 1.0
                && (quad.bounds.center_x() - parent.center_x()).abs() < 0.01
                && quad.bounds.y <= child.y
                && quad.bounds.y + quad.bounds.height >= child.y + child.height),
            "expanded sp6 must guide its visible child"
        );
        drop(scene);
        app.file_tree_expanded_set.remove(if changes { "chg:sp6" } else { "sp6" });
        let scene = RowScene::new(build(app.inputs()), &mut renderer, &Theme::Dark);
        assert!(!scene.labels.iter().any(|(name, _)| name == "原始视频.mov"));
        let parent = scene.label("sp6");
        let next = scene.label("sp7");
        assert!(!scene.quads.iter().any(|(quad, _)| quad.bounds.width == 1.0
            && quad.bounds.y >= parent.y
            && quad.bounds.y < next.y));
    }
}
