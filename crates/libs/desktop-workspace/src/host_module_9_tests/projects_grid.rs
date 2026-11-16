use crate::views::project::components::projects_list::{self, Message};
use iced::widget::{Space, container};
use iced::{Element, Length, Theme};
#[derive(Default)]
struct ProjectsFixture {
    recent_projects: Vec<String>,
    recent_projects_edits: Vec<String>,
    code_projects_menu_open: bool,
}
fn projects_list(app: &ProjectsFixture, collapsed: bool) -> Element<'static, Message> {
    projects_list::projects_list(
        &projects_list::Input {
            recent_projects: &app.recent_projects,
            recent_projects_edits: &app.recent_projects_edits,
            recent_projects_meta: &[],
            project_path: &None,
            hovered_recent_project: &None,
            code_projects_menu_open: app.code_projects_menu_open,
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            project_attention: vec![],
        },
        collapsed,
    )
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn code_projects_rail_renders_titles_and_opens_the_four_column_grid() {
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::widget::row;
    use iced::{Event, Font, Point, Renderer, Size, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let mut app = ProjectsFixture::default();
    app.recent_projects = (0..7).map(|index| format!("/tmp/project-{index}")).collect();
    app.recent_projects_edits =
        (0..7).map(|index| format!("项目 {index} 的完整中文名称 Mixed Project Name")).collect();
    for font in desktop_assets::fonts::load_all() {
        iced::advanced::graphics::text::font_system().write().unwrap().load_font(font);
    }
    let mut renderer = iced::futures::executor::block_on(Renderer::new(
        Font::with_name("Noto Sans CJK SC"),
        14.0.into(),
        Some("tiny-skia"),
    ))
    .unwrap();
    let size = Size::new(640.0, 600.0);
    let rail = |app: &ProjectsFixture| {
        row![container(projects_list(app, false)).width(56), Space::new().width(Length::Fill)]
    };
    let update = |ui: &mut UserInterface<'_, Message, Theme, Renderer>,
                  renderer: &mut Renderer,
                  point: Point,
                  events: &[Event]| {
        let mut messages = Vec::new();
        ui.update(
            events,
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
    };
    let click = [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ];
    let move_to = |point| [Event::Mouse(mouse::Event::CursorMoved { position: point })];
    let menu_changed = |message: &Message, open| {
        matches!(
            message,
            Message::CodeProjectsMenuChanged(value)
                if *value == open
        )
    };
    let trigger = Point::new(28.0, 327.0);
    let gap = Point::new(54.0, 327.0);
    let last_project = Point::new(331.0, 450.0);
    let outside = Point::new(560.0, 327.0);

    let seventh_name = app.recent_projects_edits.pop().unwrap();
    let mut six_projects = UserInterface::build(rail(&app), size, Cache::new(), &mut renderer);
    assert!(
        update(&mut six_projects, &mut renderer, trigger, &click)
            .iter()
            .any(|message| { matches!(message, Message::OpenFolderPressed) })
    );
    drop(six_projects);
    app.recent_projects_edits.push(seventh_name);

    for theme in [Theme::Light, Theme::Dark] {
        app.code_projects_menu_open = false;
        let mut ui = UserInterface::build(rail(&app), size, Cache::new(), &mut renderer);
        let style = Style { text_color: theme.palette().text };
        let first_project = Point::new(28.0, 26.0);
        update(&mut ui, &mut renderer, first_project, &move_to(first_project));
        ui.draw(&mut renderer, &theme, &style, mouse::Cursor::Available(first_project));
        let pixels = renderer.screenshot(Size::new(640, 600), 1.0, theme.palette().background);
        let background = &pixels[..4];
        let tooltip_pixel = (26 * 640 + 100) * 4;
        assert_ne!(&pixels[tooltip_pixel..tooltip_pixel + 4], background);
        assert!(
            update(&mut ui, &mut renderer, trigger, &move_to(trigger))
                .iter()
                .any(|message| menu_changed(message, true))
        );
        drop(ui);

        app.code_projects_menu_open = true;
        let mut ui = UserInterface::build(rail(&app), size, Cache::new(), &mut renderer);
        for point in [gap, last_project] {
            assert!(
                !update(&mut ui, &mut renderer, point, &move_to(point))
                    .iter()
                    .any(|message| menu_changed(message, false))
            );
        }
        ui.draw(&mut renderer, &theme, &style, mouse::Cursor::Available(last_project));
        let pixels = renderer.screenshot(Size::new(640, 600), 1.0, theme.palette().background);
        let grid_pixel = (365 * 640 + 440) * 4;
        assert_ne!(&pixels[grid_pixel..grid_pixel + 4], &pixels[..4]);
        assert!(update(&mut ui, &mut renderer, last_project, &click).iter().any(|message| {
            matches!(
                message,
                Message::OpenRecentPressed(path)
                    if path == "/tmp/project-6"
            )
        }));
        assert!(
            update(&mut ui, &mut renderer, outside, &move_to(outside))
                .iter()
                .any(|message| menu_changed(message, false))
        );
    }
}
