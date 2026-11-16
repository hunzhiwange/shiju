use iced::{Theme, Color};
#[tokio::test(flavor = "multi_thread")]
async fn app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::Tree;
    use iced::{Font, Pixels};
    let mut app = crate::app::App::new().0;
    app.desktop_update.release = Some(desktop_release::DesktopRelease {
        source: desktop_release::UpdateSource::Shiju,
        version: "99.0.0".into(),
        assets: vec![],
    });
    app.update_panel.cli_install_modal_show_install_action = true;
    app.update_panel.cli_install_modal_title = "安装包已就绪".into();
    app.update_panel.cli_install_modal_message = "安装包路径：/tmp/下载目录/安装包.dmg；请完成系统安装。".into();
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    use super::cli_command_help::CliCommandsTab;
    for (theme, app_update, tab) in [
        (Theme::Dark, true, CliCommandsTab::Common),
        (Theme::Light, true, CliCommandsTab::Common),
        (Theme::Dark, false, CliCommandsTab::Common),
        (Theme::Light, false, CliCommandsTab::Common),
        (Theme::Dark, false, CliCommandsTab::Service),
        (Theme::Light, false, CliCommandsTab::Service),
        (Theme::Dark, false, CliCommandsTab::All),
        (Theme::Light, false, CliCommandsTab::All),
    ] {
        app.update_panel.cli_install_modal_use_app_update_action = app_update;
        app.navigation_panels.cli_commands_tab = tab;
        for mut element in
            [super::top_bar::view(&app)]
        {
            let viewport = iced::Rectangle::with_size(iced::Size::new(960.0, 900.0));
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
            );
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style::default(),
                iced::advanced::Layout::new(&node),
                iced::mouse::Cursor::Unavailable,
                &viewport,
            );
            let pixels = renderer.screenshot(iced::Size::new(960, 900), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }
}
