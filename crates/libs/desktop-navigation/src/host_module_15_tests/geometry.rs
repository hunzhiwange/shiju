#[derive(Default)]
struct GeometryFixture {
    main_window_id: Option<iced::window::Id>,
    window_size: (f32, f32),
    startup_resize_checked: bool,
    fullscreen_layout_settling: bool,
}
struct Resized(iced::window::Id, f32, f32);
fn test_app() -> GeometryFixture {
    GeometryFixture::default()
}
fn update(
    app: &mut GeometryFixture,
    Resized(window, width, height): Resized,
) -> Option<iced::Size> {
    crate::window_lifecycle::resize_main_window(
        app.main_window_id,
        window,
        &mut app.window_size,
        &mut app.startup_resize_checked,
        app.fullscreen_layout_settling,
        width,
        height,
    )
}
#[test]
fn window_resize_ignores_other_windows_and_clamps_main_dimensions() {
    let mut app = test_app();
    let main_window_id = iced::window::Id::unique();
    app.main_window_id = Some(main_window_id);
    app.window_size = (900.0, 700.0);

    let _task = update(&mut app, Resized(iced::window::Id::unique(), 0.0, -5.0));
    assert_eq!(app.window_size, (900.0, 700.0));
    assert!(!app.startup_resize_checked);

    let _task = update(&mut app, Resized(main_window_id, 0.0, -5.0));
    assert_eq!(app.window_size, (1.0, 1.0));
    assert!(app.startup_resize_checked);
}

#[test]
fn window_resize_during_fullscreen_settling_does_not_consume_startup_check() {
    let mut app = test_app();
    let main_window_id = iced::window::Id::unique();
    app.main_window_id = Some(main_window_id);
    app.fullscreen_layout_settling = true;

    let _task = update(&mut app, Resized(main_window_id, 640.0, 480.0));

    assert_eq!(app.window_size, (640.0, 480.0));
    assert!(!app.startup_resize_checked);
}

#[test]
fn fullscreen_layout_settled_clears_settling_flag() {
 let mut settling = true;
 crate::window_lifecycle::settle_fullscreen_layout(&mut settling);
 assert!(!settling);
}
