use super::tree_list::{build_changes_list, build_file_tree_list};
use super::tree_list_tests::{RowScene, alignment_app};
use iced::{Background, Theme};

#[tokio::test]
async fn tree_gutter_draws_ancestor_lines_with_subtree_bounds() {
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    for theme in [Theme::Light, Theme::Dark] {
        for changes in [false, true] {
            let app = alignment_app();
            let build = if changes { build_changes_list } else { build_file_tree_list };
            let scene = RowScene::new(build(&app), &mut renderer, &theme);
            let lines =
                scene.quads.iter().filter(|(quad, _)| quad.bounds.width == 1.0).collect::<Vec<_>>();
            assert_eq!(lines.len(), 4, "one line per ancestor of each visible row");
            let sp6 = scene.icon("sp6").1;
            let video = scene.label("原始视频.mov");
            // The guide is centered below its parent's icon, across the full child row.
            let line = lines
                .iter()
                .find(|(quad, _)| {
                    (quad.bounds.center_x() - sp6.center_x()).abs() < 0.01
                        && quad.bounds.y <= video.y
                        && (quad.bounds.y + quad.bounds.height) >= (video.y + video.height)
                })
                .expect("sp6 child guide");
            assert!(line.0.bounds.y > sp6.center_y());
            assert!((line.0.bounds.y + line.0.bounds.height) < scene.label("sp7").center_y());
            for (quad, color) in &lines {
                assert_eq!(
                    *color,
                    Background::Color(theme.extended_palette().background.strong.color)
                );
                assert_ne!(*color, Background::Color(theme.palette().background));
                assert!(quad.bounds.y > scene.label("closed").center_y());
                assert!((quad.bounds.y + quad.bounds.height) < scene.label("root.md").center_y());
            }
            let nested = scene.icon(".shiju").1;
            let config = scene.label("配置.json");
            let nested_lines = lines
                .iter()
                .filter(|(quad, _)| {
                    quad.bounds.y <= config.y
                        && (quad.bounds.y + quad.bounds.height) >= (config.y + config.height)
                })
                .collect::<Vec<_>>();
            assert_eq!(nested_lines.len(), 2);
            assert!(
                nested_lines
                    .iter()
                    .any(|(quad, _)| (quad.bounds.center_x() - nested.center_x()).abs() < 0.01)
            );
            let mut parent_lines = lines
                .iter()
                .filter(|(quad, _)| {
                    (quad.bounds.center_x() - sp6.center_x()).abs() < 0.01
                        && quad.bounds.y > scene.label("sp7").center_y()
                })
                .map(|(quad, _)| quad.bounds)
                .collect::<Vec<_>>();
            parent_lines.sort_by(|a, b| a.y.total_cmp(&b.y));
            assert_eq!(parent_lines.len(), 2);
            assert!(
                ((parent_lines[0].y + parent_lines[0].height) - parent_lines[1].y).abs() < 0.01
            );
            if let Some(directory) = std::env::var_os("SHIJU_TREE_GUTTER_EVIDENCE") {
                use iced::advanced::renderer::Headless as _;
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                let pixels =
                    renderer.screenshot(iced::Size::new(640, 700), 1.0, theme.palette().background);
                image::save_buffer(
                    directory.join(format!("tree-gutter-{theme:?}-{changes}.png")),
                    &pixels,
                    640,
                    700,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}

#[tokio::test]
async fn tree_gutter_preserves_alignment_and_ignores_pointer_events() {
    use crate::app::{Message, message::ProjectMessage};
    use iced::{Event, mouse};
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    for theme in [Theme::Light, Theme::Dark] {
        for changes in [false, true] {
            let app = alignment_app();
            let build = if changes { build_changes_list } else { build_file_tree_list };
            let mut scene = RowScene::new(build(&app), &mut renderer, &theme);
            assert_eq!(scene.label("原始视频.mov").x - scene.label("sp6").x, 20.0);
            let point = iced::Point::new(
                scene.icon("sp6").1.center_x(),
                scene.label("原始视频.mov").center_y(),
            );
            let mut messages = Vec::new();
            scene.ui.update(
                &[
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(point),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            assert!(messages.iter().any(|message| if changes {
                matches!(message, Message::Project(ProjectMessage::OpenChangedFile(path)) if path == "sp6/原始视频.mov")
            } else {
                matches!(message, Message::Project(ProjectMessage::FileTreeSelected { path, directory: false, .. }) if path == "/tree-fixture/sp6/原始视频.mov")
            }), "guide must preserve full-row click targets");
            messages.clear();
            scene.ui.update(
                &[
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
                ],
                mouse::Cursor::Available(point),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            assert!(
                messages.iter().any(|message| matches!(message,
                    Message::Project(ProjectMessage::FileTreeRightClicked(path, source, _, _))
                    if path == "/tree-fixture/sp6/原始视频.mov"
                        && source == if changes { "changes-file" } else { "tree-file" }
                )),
                "guide must preserve the context menu target"
            );
        }
    }
}

#[tokio::test]
async fn tree_gutter_clips_deep_rows_and_skips_empty_subtrees() {
    use super::model::{FileTreeNode, build_file_tree_model};
    use iced::widget::{container, scrollable};
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    let mut app = alignment_app();
    let paths = vec!["/tree-fixture/a/b/c/d/原始视频.mov".to_string()];
    let mut model = build_file_tree_model("/tree-fixture", &paths);
    model.children.insert("empty".into(), FileTreeNode::default());
    app.file_tree_model_cache.insert("/tree-fixture".into(), model);
    for key in ["a", "a/b", "a/b/c", "a/b/c/d", "empty"] {
        app.file_tree_expanded_set.insert(key.into());
    }
    for theme in [Theme::Light, Theme::Dark] {
        let scene = RowScene::new(build_file_tree_list(&app), &mut renderer, &theme);
        let child = scene.label("原始视频.mov");
        let lines =
            scene.quads.iter().filter(|(quad, _)| quad.bounds.width == 1.0).collect::<Vec<_>>();
        assert_eq!(lines.len(), 10); // depths 1 + 2 + 3 + 4; empty root directory has none.
        assert_eq!(
            lines
                .iter()
                .filter(|(quad, _)| quad.bounds.y <= child.y
                    && (quad.bounds.y + quad.bounds.height) >= (child.y + child.height))
                .count(),
            4
        );
        assert!(lines.iter().all(
            |(quad, _)| (quad.bounds.y + quad.bounds.height) < scene.label("empty").center_y()
        ));
        drop(scene);
        let clipped = RowScene::new(
            container(scrollable(build_file_tree_list(&app)).height(80)).height(80).into(),
            &mut renderer,
            &theme,
        );
        let lines =
            clipped.quads.iter().filter(|(quad, _)| quad.bounds.width == 1.0).collect::<Vec<_>>();
        assert!(!lines.is_empty());
        assert!(
            lines
                .iter()
                .all(|(quad, _)| quad.bounds.y >= 0.0
                    && (quad.bounds.y + quad.bounds.height) <= 80.0)
        );
    }
}

#[tokio::test]
async fn tree_gutter_remains_visible_on_selected_and_hovered_rows() {
    use iced::advanced::{Renderer as _, renderer::Headless as _};
    use iced::{Event, Point, Size, mouse};
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    for theme in [Theme::Light, Theme::Dark] {
        for changes in [false, true] {
            let mut app = alignment_app();
            app.git_focused_file = Some("sp6/原始视频.mov".into());
            let selected = "/tree-fixture/sp6/原始视频.mov".to_string();
            app.file_tree_selection.select(
                "/tree-fixture",
                selected.clone(),
                false,
                false,
                &[selected],
            );
            let build = if changes { build_changes_list } else { build_file_tree_list };
            let mut scene = RowScene::new(build(&app), &mut renderer, &theme);
            let child = scene.label("原始视频.mov");
            let guide = Point::new(scene.icon("sp6").1.center_x(), child.center_y());
            for hovered in [false, true] {
                let cursor = if hovered {
                    mouse::Cursor::Available(guide)
                } else {
                    mouse::Cursor::Unavailable
                };
                // Iced 在重绘事件中更新悬浮样式，与真实窗口事件顺序一致。
                scene.ui.update(
                    &[
                        Event::Mouse(mouse::Event::CursorMoved { position: guide }),
                        Event::Window(iced::window::Event::RedrawRequested(
                            iced::time::Instant::now(),
                        )),
                    ],
                    cursor,
                    &mut renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut Vec::new(),
                );
                renderer.reset(iced::Rectangle::with_size(Size::new(640.0, 700.0)));
                scene.ui.draw(
                    &mut renderer,
                    &theme,
                    &iced::advanced::renderer::Style { text_color: theme.palette().text },
                    cursor,
                );
                let iced::Renderer::Secondary(software) = &mut renderer else {
                    panic!("tiny-skia renderer")
                };
                let quads =
                    software.layers().iter().flat_map(|layer| &layer.quads).collect::<Vec<_>>();
                assert!(
                    quads
                        .iter()
                        .any(|(quad, _)| quad.bounds.width == 1.0 && quad.bounds.contains(guide))
                );
                let alpha = if changes || hovered { 0.10 } else { 0.18 };
                let primary = theme.palette().primary;
                let expected = Background::Color(iced::Color::from_rgba(
                    primary.r, primary.g, primary.b, alpha,
                ));
                assert!(quads.iter().any(|(quad, color)| quad.bounds.width > 1.0
                    && quad.bounds.contains(guide)
                    && *color == expected));
                let pixels =
                    renderer.screenshot(Size::new(640, 700), 1.0, theme.palette().background);
                // Compare the rasterized guide with nearby background, away from icons and text.
                let pixel = |x: usize| &pixels[(guide.y as usize * 640 + x) * 4..][..3];
                assert_ne!(pixel(guide.x.floor() as usize), pixel(guide.x.floor() as usize + 3));
                if let Some(directory) = std::env::var_os("SHIJU_TREE_GUTTER_EVIDENCE") {
                    std::fs::create_dir_all(&directory).unwrap();
                    image::save_buffer(
                        std::path::PathBuf::from(directory)
                            .join(format!("tree-gutter-{theme:?}-{changes}-hover-{hovered}.png")),
                        &pixels,
                        640,
                        700,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
            }
        }
    }
}
