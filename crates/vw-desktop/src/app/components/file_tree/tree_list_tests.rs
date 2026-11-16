//! 宿主拖放清理、目录选择更新与消息映射契约。
use super::model::build_file_tree_model;
use super::tree_list::{build_changes_list, build_file_tree_list};
use crate::app::App;
use iced::{Background, Theme};
#[test]
fn file_tree_drop_release_and_cleanup_orders_leave_no_stale_state() {
    use crate::app::message::project::{self, ProjectMessage};
    for release_first in [true, false] {
        let mut app = App::new().0;
        app.navigation_tabs.project_path = Some("/project".into());
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
        assert_eq!(
            app.file_tree_interaction.file_tree_drop_target.as_deref(),
            Some("/project/target")
        );
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
        assert!(app.file_tree_interaction.file_tree_drag_source.is_none());
        assert!(app.file_tree_interaction.file_tree_drop_target.is_none());
        assert!(app.file_tree_interaction.dragging_file_paths.is_empty());
        assert!(app.file_tree_interaction.pending_drop_file_paths.is_empty());
        assert!(app.file_tree_interaction.file_tree_operation_in_progress);
    }
}

#[test]
fn file_tree_drop_chat_sources_and_busy_operations_do_not_enable_move_state() {
    use crate::app::message::project::{self, ProjectMessage};
    let mut app = App::new().0;
    app.navigation_tabs.project_path = Some("/project".into());
    let _ =
        project::update(&mut app, ProjectMessage::FileTreeDragStart("/project/file".into(), None));
    let _ = project::update(
        &mut app,
        ProjectMessage::FileTreeDropTargetChanged(Some("/project".into())),
    );
    assert!(app.file_tree_interaction.file_tree_drag_source.is_none());
    assert!(app.file_tree_interaction.file_tree_drop_target.is_none());
    app.file_tree_interaction.file_tree_operation_in_progress = true;
    let _ = project::update(
        &mut app,
        ProjectMessage::FileTreeInternalDragStart {
            project_path: "/project".into(),
            source_path: "/project/file".into(),
        },
    );
    assert!(app.file_tree_interaction.file_tree_drag_source.is_none());
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
    app.navigation_tabs.project_path = Some("/tree-fixture".into());
    app.git.git_changed_files =
        ["root.md", "sp6/原始视频.mov", "sp7/.shiju/配置.json", "closed/child.txt"]
            .map(str::to_owned)
            .to_vec();
    let files = app
        .git.git_changed_files
        .iter()
        .map(|path| format!("/tree-fixture/{path}"))
        .collect::<Vec<_>>();
    app.workspace_state.file_tree_model_cache
        .insert("/tree-fixture".into(), build_file_tree_model("/tree-fixture", &files));
    for key in ["sp6", "sp7", "sp7/.shiju", "chg:sp6", "chg:sp7", "chg:sp7/.shiju"] {
        app.workspace_state.file_tree_expanded_set.insert(key.into());
    }
    app
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
