//! 验证文件树路径建模、展开键、选择优先级与变更列表视图。
//! 重复分隔符、根文件、嵌套目录和不同菜单来源用于约束树模型的规范化与隔离。

use super::tree_list::{
    DirNode, build_changes_list, build_file_tree_list, change_list_expand_keys,
    changes_directory_button_style, changes_file_button_style, insert_path,
    tree_directory_button_style, tree_file_button_style,
};
use crate::app::App;
use crate::app::components::file_tree::model::build_file_tree_model;
use iced::widget::button;
use iced::{Background, Color, Theme};

fn background_color(style: button::Style) -> Option<Color> {
    match style.background {
        Some(Background::Color(color)) => Some(color),
        _ => None,
    }
}

fn primary_with_alpha(theme: &Theme, alpha: f32) -> Color {
    let primary = theme.palette().primary;
    Color::from_rgba(primary.r, primary.g, primary.b, alpha)
}

#[test]
fn insert_path_ignores_empty_paths_and_groups_root_and_nested_files() {
    let mut tree = DirNode::default();

    for path in ["", "/", "README.md", "src/main.rs", "src/bin/tool.rs", "src//lib.rs"] {
        insert_path(&mut tree, path);
    }

    assert_eq!(tree.files, vec!["README.md"]);
    let src = tree.children.get("src").expect("src directory should be created");
    assert_eq!(src.files, vec!["src/main.rs", "src//lib.rs"]);
    assert_eq!(src.children["bin"].files, vec!["src/bin/tool.rs"]);
}

#[test]
fn change_list_expand_keys_use_chg_prefix_and_all_ancestor_dirs() {
    let keys = change_list_expand_keys(["README.md", "src/main.rs", "src/nested/lib.rs", ""]);
    assert_eq!(
        keys.into_iter().collect::<Vec<_>>(),
        vec!["chg:src".to_string(), "chg:src/nested".to_string()]
    );
}

#[test]
fn insert_path_normalizes_leading_and_repeated_separators_for_directory_segments() {
    let mut tree = DirNode::default();

    insert_path(&mut tree, "/src/generated/schema.rs");
    insert_path(&mut tree, "src///generated/client.rs");

    let generated = &tree.children["src"].children["generated"];
    assert_eq!(generated.files, vec!["/src/generated/schema.rs", "src///generated/client.rs"]);
}

#[test]
fn tree_directory_style_distinguishes_active_hovered_and_pressed_states() {
    let theme = Theme::Dark;

    let active = tree_directory_button_style(&theme, button::Status::Active);
    let hovered = tree_directory_button_style(&theme, button::Status::Hovered);
    let pressed = tree_directory_button_style(&theme, button::Status::Pressed);

    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert_eq!(background_color(active), None);
    assert_eq!(background_color(hovered), Some(primary_with_alpha(&theme, 0.10)));
    assert_eq!(background_color(pressed), Some(primary_with_alpha(&theme, 0.18)));
}

#[test]
fn tree_file_style_preserves_selection_and_interaction_precedence() {
    let theme = Theme::Light;

    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Active, false)),
        None
    );
    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Active, true)),
        Some(primary_with_alpha(&theme, 0.18))
    );
    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Hovered, true)),
        Some(primary_with_alpha(&theme, 0.10))
    );
    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Pressed, false)),
        Some(primary_with_alpha(&theme, 0.18))
    );
    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Disabled, true)),
        Some(primary_with_alpha(&theme, 0.18))
    );
}

#[test]
fn changes_styles_use_hover_color_for_interaction_and_selection() {
    let theme = Theme::Dark;
    let expected = Some(primary_with_alpha(&theme, 0.10));

    assert_eq!(
        background_color(changes_directory_button_style(&theme, button::Status::Active)),
        None
    );
    assert_eq!(
        background_color(changes_directory_button_style(&theme, button::Status::Hovered)),
        expected
    );
    assert_eq!(
        background_color(changes_directory_button_style(&theme, button::Status::Pressed)),
        expected
    );
    assert_eq!(
        background_color(changes_file_button_style(&theme, button::Status::Active, false)),
        None
    );
    assert_eq!(
        background_color(changes_file_button_style(&theme, button::Status::Active, true)),
        expected
    );
    assert_eq!(
        background_color(changes_file_button_style(&theme, button::Status::Hovered, false)),
        expected
    );
    assert_eq!(
        background_color(changes_file_button_style(&theme, button::Status::Disabled, true)),
        expected
    );
}

#[test]
fn tree_list_builds_empty_states_with_and_without_project() {
    let app = App::new().0;
    let _ = build_file_tree_list(&app);

    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_tree_model_cache
        .insert("/tmp/demo".to_string(), build_file_tree_model("/tmp/demo", &[]));
    let _ = build_file_tree_list(&app);
}

#[test]
fn tree_list_builds_fallback_model_with_expanded_selection_and_file_menu() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_tree_model_cache
        .insert("/tmp/demo".to_string(), build_file_tree_model("/tmp/demo", &[]));
    app.file_index_cache.insert(
        "/tmp/demo".to_string(),
        vec![
            "/tmp/demo/README.md".to_string(),
            "/tmp/demo/src/main.rs".to_string(),
            "/tmp/demo/src/nested/lib.rs".to_string(),
        ],
    );
    app.file_tree_expanded_set.insert("src".to_string());
    app.file_tree_expanded_set.insert("src/nested".to_string());
    app.active_preview_path = Some("/tmp/demo/src/main.rs".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/main.rs".to_string());
    app.file_tree_menu_source = Some("tree-file".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(12.0, 24.0));

    let _ = build_file_tree_list(&app);
}

#[test]
fn tree_list_builds_cached_model_with_directory_menu() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    let files = vec!["/tmp/demo/src/main.rs".to_string()];
    app.file_tree_model_cache
        .insert("/tmp/demo".to_string(), build_file_tree_model("/tmp/demo", &files));
    app.file_tree_expanded_set.insert("src".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src".to_string());
    app.file_tree_menu_source = Some("tree-dir".to_string());

    let _ = build_file_tree_list(&app);
}

#[test]
fn changes_list_builds_loading_no_changes_and_empty_path_states() {
    let mut app = App::new().0;
    let _ = build_changes_list(&app);

    app.git_changed_files_loading = true;
    let _ = build_changes_list(&app);

    app.git_changed_files_loading = false;
    app.git_changed_files = vec![String::new(), "/".to_string()];
    let _ = build_changes_list(&app);
}

#[test]
fn changes_list_builds_root_and_nested_files_with_directory_menu() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.git_changed_files =
        vec!["README.md".to_string(), "src/main.rs".to_string(), "src/nested/lib.rs".to_string()];
    app.file_tree_expanded_set.insert("chg:src".to_string());
    app.git_focused_file = Some("src/main.rs".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/nested".to_string());
    app.file_tree_menu_source = Some("changes-dir".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(8.0, 16.0));

    let _ = build_changes_list(&app);
}

#[test]
fn changes_list_builds_expanded_file_menu_for_selected_file() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.git_changed_files = vec!["src/main.rs".to_string()];
    app.file_tree_expanded_set.insert("chg:src".to_string());
    app.git_focused_file = Some("src/main.rs".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/main.rs".to_string());
    app.file_tree_menu_source = Some("changes-file".to_string());

    let _ = build_changes_list(&app);
}

#[test]
fn changes_list_builds_root_file_menu_without_project_path() {
    let mut app = App::new().0;
    app.git_changed_files = vec!["README.md".to_string()];
    app.git_focused_file = Some("README.md".to_string());
    app.file_tree_menu_path = Some("README.md".to_string());
    app.file_tree_menu_source = Some("changes-file".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(4.0, 6.0));

    let element = build_changes_list(&app);

    assert_eq!(element.as_widget().size().width, iced::Length::Fill);
}

#[test]
fn tree_lists_build_collapsed_directories_and_ignore_mismatched_menu_sources() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    let files = vec!["/tmp/demo/src/nested/lib.rs".to_string()];
    app.file_tree_model_cache
        .insert("/tmp/demo".to_string(), build_file_tree_model("/tmp/demo", &files));
    app.file_tree_menu_path = Some("/tmp/demo/src".to_string());
    app.file_tree_menu_source = Some("changes-dir".to_string());

    let _collapsed_tree = build_file_tree_list(&app);
    drop(_collapsed_tree);

    app.git_changed_files = vec!["src/nested/lib.rs".to_string()];
    app.file_tree_menu_source = Some("tree-dir".to_string());

    let _collapsed_changes = build_changes_list(&app);
}

#[test]
fn file_tree_drop_directory_highlight_supports_both_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let style =
            super::tree_list::file_tree_drop_directory_style(&theme, button::Status::Active, true);
        assert_eq!(style.text_color, theme.palette().text);
        assert_eq!(background_color(style), Some(primary_with_alpha(&theme, 0.24)));
        assert_eq!(
            background_color(super::tree_list::file_tree_drop_directory_style(
                &theme,
                button::Status::Active,
                false
            )),
            None
        );
    }
}

#[test]
fn file_tree_drop_release_and_cleanup_orders_leave_no_stale_state() {
    use crate::app::message::project::{self, ProjectMessage};
    for release_first in [true, false] {
        let mut app = App::new().0;
        app.project_path = Some("/project".into());
        let _ = project::update(
            &mut app,
            ProjectMessage::FileTreeInternalDragStart {
                project_path: "/project".into(),
                source_path: "/project/source".into(),
            },
        );
        let _ = project::update(
            &mut app,
            ProjectMessage::FileTreeDropTargetChanged(Some("/project/target".into())),
        );
        assert_eq!(app.file_tree_drop_target.as_deref(), Some("/project/target"));
        let request = ProjectMessage::FileTreeDropRequested {
            project_path: "/project".into(),
            source_path: "/project/source".into(),
            target_directory: "/project/target".into(),
        };
        if release_first {
            let _ = project::update(&mut app, ProjectMessage::FileTreeDragEnd);
            let _ = project::update(&mut app, request);
        } else {
            let _ = project::update(&mut app, request);
            let _ = project::update(&mut app, ProjectMessage::FileTreeDragEnd);
        }
        assert!(app.file_tree_drag_source.is_none());
        assert!(app.file_tree_drop_target.is_none());
        assert!(app.dragging_file_paths.is_empty());
        assert!(app.pending_drop_file_paths.is_empty());
        assert!(app.file_tree_operation_in_progress);
    }
}

#[test]
fn file_tree_drop_chat_sources_and_busy_operations_do_not_enable_move_state() {
    use crate::app::message::project::{self, ProjectMessage};
    let mut app = App::new().0;
    app.project_path = Some("/project".into());
    let _ =
        project::update(&mut app, ProjectMessage::FileTreeDragStart("/project/file".into(), None));
    let _ = project::update(
        &mut app,
        ProjectMessage::FileTreeDropTargetChanged(Some("/project".into())),
    );
    assert!(app.file_tree_drag_source.is_none());
    assert!(app.file_tree_drop_target.is_none());
    app.file_tree_operation_in_progress = true;
    let _ = project::update(
        &mut app,
        ProjectMessage::FileTreeInternalDragStart {
            project_path: "/project".into(),
            source_path: "/project/file".into(),
        },
    );
    assert!(app.file_tree_drag_source.is_none());
}

pub(super) struct RowScene<'a> {
    pub(super) quads: Vec<(iced::advanced::renderer::Quad, Background)>,
    pub(super) ui:
        iced_runtime::user_interface::UserInterface<'a, crate::app::Message, Theme, iced::Renderer>,
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
        element: iced::Element<'a, crate::app::Message>,
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
    ) -> Vec<crate::app::Message> {
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
        messages.retain(|message| !matches!(message, crate::app::Message::None));
        messages
    }
}

pub(super) fn alignment_app() -> App {
    let mut app = App::new().0;
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
    app.file_tree_model_cache
        .insert("/tree-fixture".into(), build_file_tree_model("/tree-fixture", &files));
    for key in ["sp6", "sp7", "sp7/.shiju", "chg:sp6", "chg:sp7", "chg:sp7/.shiju"] {
        app.file_tree_expanded_set.insert(key.into());
    }
    app
}

#[tokio::test]
async fn tree_rows_align_siblings_and_indent_children() {
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    let app = alignment_app();
    for theme in [Theme::Light, Theme::Dark] {
        for element in [build_file_tree_list(&app), build_changes_list(&app)] {
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
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    let app = alignment_app();
    for element in [build_file_tree_list(&app), build_changes_list(&app)] {
        let scene = RowScene::new(element, &mut renderer, &Theme::Dark);
        assert_eq!(scene.icons.len(), 7, "exactly one icon per visible row");
        assert_ne!(
            scene.icon("closed").0,
            scene.icon("sp6").0,
            "closed and open folders must differ"
        );
        assert_eq!(
            scene.icon("sp6").0,
            crate::app::assets::get_icon(crate::app::assets::Icon::FolderOpen)
        );
    }
}

#[tokio::test]
async fn tree_rows_preserve_selection_open_and_toggle_messages() {
    use crate::app::Message;
    use crate::app::message::ProjectMessage;
    use iced::keyboard::Modifiers;
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    let mut app = alignment_app();
    for theme in [Theme::Light, Theme::Dark] {
        for modifiers in [Modifiers::NONE, Modifiers::CTRL, Modifiers::SHIFT, Modifiers::LOGO] {
            let mut scene = RowScene::new(build_file_tree_list(&app), &mut renderer, &theme);
            let messages = scene.click(&mut renderer, "原始视频.mov", modifiers);
            assert!(messages.iter().any(|message| matches!(message,
                Message::Project(ProjectMessage::FileTreeSelected { path, directory: false, modifiers: actual, .. })
                    if path == "/tree-fixture/sp6/原始视频.mov" && *actual == modifiers
            )));
        }
        let mut scene = RowScene::new(build_changes_list(&app), &mut renderer, &theme);
        let messages = scene.click(&mut renderer, "原始视频.mov", Modifiers::NONE);
        assert!(messages.iter().any(|message| matches!(message,
            Message::Project(ProjectMessage::OpenChangedFile(path)) if path == "sp6/原始视频.mov"
        )));
        drop(scene);
        for changes in [false, true] {
            let element =
                if changes { build_changes_list(&app) } else { build_file_tree_list(&app) };
            let mut scene = RowScene::new(element, &mut renderer, &theme);
            let messages = scene.click(&mut renderer, "sp6", Modifiers::NONE);
            drop(scene);
            let toggle = messages.into_iter().find(|message| if changes {
                matches!(message, Message::Project(ProjectMessage::ToggleTreeDir(key)) if key == "chg:sp6")
            } else {
                matches!(message, Message::Project(ProjectMessage::FileTreeSelected { path, directory: true, .. }) if path == "/tree-fixture/sp6")
            }).expect("directory toggle message");
            let Message::Project(toggle) = toggle else { unreachable!() };
            let _ = crate::app::message::project::update(&mut app, toggle);
            let key = if changes { "chg:sp6" } else { "sp6" };
            assert!(!app.is_file_tree_dir_expanded(key));
            // Restore via the same event handler, keeping each theme independent.
            let _ = crate::app::message::project::update(
                &mut app,
                ProjectMessage::ToggleTreeDir(key.into()),
            );
            assert!(app.is_file_tree_dir_expanded(key));
        }
    }
}

#[tokio::test]
async fn tree_gutter_tracks_expansion_in_both_lists() {
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    for changes in [false, true] {
        let mut app = alignment_app();
        let build = if changes { build_changes_list } else { build_file_tree_list };
        let scene = RowScene::new(build(&app), &mut renderer, &Theme::Dark);
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
        let scene = RowScene::new(build(&app), &mut renderer, &Theme::Dark);
        assert!(!scene.labels.iter().any(|(name, _)| name == "原始视频.mov"));
        let parent = scene.label("sp6");
        let next = scene.label("sp7");
        assert!(!scene.quads.iter().any(|(quad, _)| quad.bounds.width == 1.0
            && quad.bounds.y >= parent.y
            && quad.bounds.y < next.y));
    }
}
