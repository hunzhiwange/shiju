use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size, Theme};

#[tokio::test(flavor = "multi_thread")]
async fn mobile_layout_switch_draws_compactly_in_both_themes() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    for preference in [LayoutPreference::Auto, LayoutPreference::Desktop, LayoutPreference::Mobile]
    {
        app.mobile_layout.select(preference);
        for theme in [Theme::Light, Theme::Dark] {
            for width in [360.0, 1200.0] {
                let size = Size::new(width, 800.0);
                let mut element = layout_switch(&app);
                let mut tree = Tree::new(element.as_widget());
                let node = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, size),
                );
                assert_eq!(node.size(), Size::new(24.0, 24.0));
                element.as_widget().draw(
                    &tree,
                    &mut renderer,
                    &theme,
                    &renderer::Style::default(),
                    Layout::new(&node),
                    mouse::Cursor::Available(iced::Point::new(12.0, 12.0)),
                    &Rectangle::with_size(size),
                );
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn mobile_layout_narrow_view_fits_small_and_large_windows() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let app = App::new().0;
    for width in [360.0, 1200.0] {
        let mut element = narrow_view(&app);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, 800.0)),
        );
        assert_eq!(node.bounds().width, width);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn mobile_layout_navigation_icons_draw_and_route_actions() {
    use iced::advanced::{Shell, clipboard};
    use iced::{Event, Point};
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    for (icon, label, action) in [
        (Icon::ArrowLeft, "返回", ViewMessage::MobileBack),
        (Icon::FolderClosed, "项目", ViewMessage::MobileDrawerToggled),
        (Icon::ChatTextFill, "会话", ViewMessage::MobileChat),
        (Icon::Gear, "设置", ViewMessage::MobileSettings),
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let mut element = navigation_icon(icon, label, action.clone());
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, Size::new(360.0, 800.0)),
            );
            assert_eq!(node.size(), Size::new(24.0, 24.0));
            let cursor = mouse::Cursor::Available(Point::new(16.0, 16.0));
            let viewport = Rectangle::with_size(Size::new(360.0, 800.0));
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                cursor,
                &viewport,
            );
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ] {
                element.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    Layout::new(&node),
                    cursor,
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &viewport,
                );
            }
            assert_eq!(messages.len(), 1);
            assert_eq!(
                format!("{:?}", messages[0]),
                format!("{:?}", Message::View(action.clone()))
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn mobile_bottom_navigation_draws_equal_tabs_and_opens_projects() {
    use iced::advanced::{Shell, clipboard};
    use iced::{Event, Point};
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    for theme in [Theme::Light, Theme::Dark] {
        for width in [320.0, 430.0] {
            for (drawer, settings) in [(false, false), (true, false), (false, true)] {
                app.mobile_layout.drawer_open = drawer;
                app.navigation_panels.show_system_settings = settings;
                let mut element = bottom_navigation(&app);
                let mut tree = Tree::new(element.as_widget());
                let viewport = Rectangle::with_size(Size::new(width, 100.0));
                let node = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, viewport.size()),
                );
                let tabs = node.children()[0].children();
                assert_eq!(tabs.len(), 4);
                for tab in tabs {
                    assert!((tab.size().width - width / 4.0).abs() < 1.0);
                }
                let cursor =
                    mouse::Cursor::Available(Point::new(width * 0.375, node.size().height / 2.0));
                element.as_widget().draw(
                    &tree,
                    &mut renderer,
                    &theme,
                    &renderer::Style::default(),
                    Layout::new(&node),
                    cursor,
                    &viewport,
                );
                let mut messages = Vec::new();
                for event in [
                    mouse::Event::ButtonPressed(mouse::Button::Left),
                    mouse::Event::ButtonReleased(mouse::Button::Left),
                ] {
                    element.as_widget_mut().update(
                        &mut tree,
                        &Event::Mouse(event),
                        Layout::new(&node),
                        cursor,
                        &renderer,
                        &mut clipboard::Null,
                        &mut Shell::new(&mut messages),
                        &viewport,
                    );
                }
                if drawer {
                    assert!(messages.is_empty(), "selected project tab stays open");
                } else {
                    assert!(matches!(
                        messages.as_slice(),
                        [Message::View(ViewMessage::MobileDrawerToggled)]
                    ));
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn mobile_bottom_bar_reaches_page_edges_on_all_tabs() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    app.mobile_layout.select(LayoutPreference::Mobile);
    // 检查完整手机页面，避免仅测底栏而漏掉父级 padding 引入的空隙。
    fn has_bottom_bar(layout: Layout<'_>, size: Size) -> bool {
        let bounds = layout.bounds();
        let children: Vec<_> = layout.children().collect();
        if bounds.x.abs() < 0.1
            && (bounds.width - size.width).abs() < 0.1
            && (bounds.y + bounds.height - size.height).abs() < 0.1
            && (60.0..90.0).contains(&bounds.height)
            && children.len() == 1
            && children[0].children().count() == 4
        {
            return true;
        }
        children.into_iter().any(|child| has_bottom_bar(child, size))
    }
    for theme in [Theme::Light, Theme::Dark] {
        for (width, height) in
            [(320.0, 568.0), (390.0, 844.0), (430.0, 932.0), (480.0, 800.0), (1200.0, 800.0)]
        {
            for (drawer, settings) in [(false, false), (true, false), (false, true)] {
                app.mobile_layout.drawer_open = drawer;
                app.navigation_panels.show_system_settings = settings;
                let size = Size::new(width, height);
                let mut element = narrow_view(&app);
                let mut tree = Tree::new(element.as_widget());
                let node = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, size),
                );
                assert!(
                    has_bottom_bar(Layout::new(&node), size),
                    "bottom bar must span the page and touch its bottom edge"
                );
                element.as_widget().draw(
                    &tree,
                    &mut renderer,
                    &theme,
                    &renderer::Style::default(),
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &Rectangle::with_size(size),
                );
            }
        }
    }
}

#[test]
fn mobile_mine_navigation_is_exclusive_and_returns_to_chat() {
    use crate::app::message::view::layout;
    let mut app = App::new().0;
    app.mobile_layout.drawer_open = true;
    app.navigation_panels.show_system_settings = true;
    let _ = layout::update(&mut app, ViewMessage::MobileMine);
    assert!(app.mobile_layout.mine_open);
    assert!(!app.mobile_layout.drawer_open);
    assert!(!app.navigation_panels.show_system_settings);
    let _ = layout::update(&mut app, ViewMessage::MobileChat);
    assert!(!app.mobile_layout.mine_open);
    let _ = layout::update(&mut app, ViewMessage::MobileMine);
    let _ = layout::update(&mut app, ViewMessage::MobileSettings);
    assert!(!app.mobile_layout.mine_open);
    assert!(app.navigation_panels.show_system_settings);
}

#[tokio::test(flavor = "multi_thread")]
async fn mobile_mine_logged_out_page_draws_in_both_themes() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    app.mobile_layout.mine_open = true;
    for theme in [Theme::Light, Theme::Dark] {
        let size = Size::new(320.0, 640.0);
        let mut element = narrow_view(&app);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        assert_eq!(node.size(), size);
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(size),
        );
    }
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread")]
async fn mobile_header_keeps_navigation_below_native_window_controls() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let app = App::new().0;
    for theme in [Theme::Light, Theme::Dark] {
        for width in [320.0, 430.0] {
            let mut element = header(&app, "会话");
            let mut tree = Tree::new(element.as_widget());
            let size = Size::new(width, 100.0);
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, size),
            );
            let layout = Layout::new(&node);
            let children: Vec<_> = layout.children().collect();
            assert_eq!(
                children[0].bounds().height,
                crate::app::components::top_bar::TOP_BAR_HEIGHT
            );
            assert!(children[1].bounds().y >= children[0].bounds().y + children[0].bounds().height);
            assert_eq!(node.size().width, width);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                layout,
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(size),
            );
        }
    }
}

#[test]
fn mobile_projects_default_to_first_project_and_preserve_current_selection() {
    use crate::app::message::view::layout;
    let mut app = App::new().0;
    app.navigation_tabs.project_path = None;
    app.workspace_state.recent_projects = vec!["/projects/first".into(), "/projects/second".into()];
    let _ = layout::update(&mut app, ViewMessage::MobileDrawerToggled);
    assert!(app.mobile_layout.drawer_open);
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/projects/first"));
    let _ = layout::update(&mut app, ViewMessage::MobileProjectSelected("/projects/second".into()));
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/projects/second"));
    let _ = layout::update(&mut app, ViewMessage::MobileDrawerToggled);
    let _ = layout::update(&mut app, ViewMessage::MobileDrawerToggled);
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/projects/second"));
}

#[test]
fn mobile_projects_without_recent_projects_keep_empty_selection() {
    use crate::app::message::view::layout;
    let mut app = App::new().0;
    app.navigation_tabs.project_path = None;
    app.workspace_state.recent_projects.clear();
    let _ = layout::update(&mut app, ViewMessage::MobileDrawerToggled);
    assert!(app.navigation_tabs.project_path.is_none());
}

#[test]
fn mobile_new_session_selects_its_project_before_showing_chat() {
    use crate::app::message::view::layout;
    let mut app = App::new().0;
    app.navigation_tabs.project_path = Some("/projects/old".into());
    app.workspace_state.new_session_creating = false;
    app.mobile_layout.drawer_open = true;
    let _ = layout::update(&mut app, ViewMessage::MobileNewSession("/projects/current".into()));
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/projects/current"));
    assert!(!app.mobile_layout.drawer_open);
}

#[test]
fn mobile_project_load_more_reuses_work_mode_page_size_per_project() {
    use crate::app::message::view;
    use desktop_workspace::views::project::layout::work_sidebar::presentation::{
        WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE, WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP,
    };
    let mut app = App::new().0;
    let path = "/projects/current".to_string();
    let _ = view::update(&mut app, ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(path.clone()));
    assert_eq!(
        app.navigation_sidebar.work_sidebar_space_task_visible.get(&path),
        Some(&(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE + WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP))
    );
    assert!(
        !app.navigation_sidebar.work_sidebar_space_task_visible.contains_key("/projects/other")
    );
}
