use super::file_preview;
use crate::app::App;
use crate::app::git_workbench::{Event, Project, files, update};
use desktop_i18n::I18n;
use iced::advanced::{Layout, layout, renderer, renderer::Headless, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size, Theme, mouse};

#[tokio::test]
async fn file_preview_reconciles_loading_content_errors_and_tab_switches() {
    let mut app = App::new().0;
    app.git_workbench.projects =
        vec![Project { id: "demo".into(), name: "Demo".into(), directory: "/tmp/demo".into() }];
    app.git_workbench.selected = Some("demo".into());
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(13.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut tree = Tree::empty();
    let size = Size::new(550.0, 420.0);
    let theme = Theme::Light;
    let mut render = |app: &App| {
        let mut view = file_preview(i18n, &app.git_workbench);
        tree.diff(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        view.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(size),
        );
    };
    let dispatch = |app: &mut App, event| {
        let epoch = app.git_workbench.epoch;
        let _ = update(app, Event::Files(epoch, event));
    };

    let _ = files::open_file(&mut app.git_workbench, "a.rs".into(), None);
    let first = app.git_workbench.files.active_preview().unwrap().id;
    render(&app);
    dispatch(&mut app, files::Event::Opened(first, Ok(("fn main() {}\n".into(), false))));
    assert!(matches!(app.git_workbench.files.active_preview().unwrap().content, Some(Ok(_))));
    render(&app);

    let _ = files::open_file(&mut app.git_workbench, "docs/说明.md".into(), None);
    let second = app.git_workbench.files.active_preview().unwrap().id;
    render(&app);
    dispatch(&mut app, files::Event::Opened(second, Err("Unable to read file".into())));
    render(&app);
    dispatch(&mut app, files::Event::SelectPreview(first));
    render(&app);

    let _ = files::open_file(&mut app.git_workbench, "docs/说明.md".into(), Some((1, 1)));
    let tab = app.git_workbench.files.active_preview().unwrap();
    assert_eq!(tab.id, second);
    assert_eq!(app.git_workbench.files.previews.len(), 2);
    render(&app);
    dispatch(&mut app, files::Event::Opened(second, Err("Stale request".into())));
    assert!(app.git_workbench.files.active_preview().unwrap().content.is_none());
    dispatch(&mut app, files::Event::Opened(second + 1, Ok(("说明\n".into(), false))));
    assert!(matches!(app.git_workbench.files.active_preview().unwrap().content, Some(Ok(_))));
    render(&app);
    dispatch(&mut app, files::Event::ClosePreview(second));
    assert_eq!(app.git_workbench.files.active_preview().unwrap().id, first);
    render(&app);
    dispatch(&mut app, files::Event::ClosePreview(first));
    assert!(app.git_workbench.files.active_preview().is_none());
    render(&app);
}
