use super::navigation_fixture::Fixture;
use crate::components::message::Message;
use iced::{Color, Theme};
#[tokio::test(flavor = "multi_thread")]
async fn app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::Tree;
    use iced::{Font, Pixels};
    let mut app = Fixture::default();
    app.desktop_update.release = Some(desktop_release::DesktopRelease {
        source: desktop_release::UpdateSource::Shiju,
        version: "99.0.0".into(),
        assets: vec![],
    });
    app.update_panel.cli_install_modal_show_install_action = true;
    app.update_panel.cli_install_modal_title = "安装包已就绪".into();
    app.update_panel.cli_install_modal_message =
        "安装包路径：/tmp/下载目录/安装包.dmg；请完成系统安装。".into();
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    use crate::components::cli_command_help::CliCommandsTab;
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
        app.cli_commands_tab = tab;
        for mut element in [app.view()] {
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

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_peripheral_appearance_cli_commands() {
    use crate::components::cli_command_help::CliCommandsTab;
    fn view_app_update(input: &Fixture) -> iced::Element<'_, Message> {
        input.view()
    }
    use super::appearance as tab_appearance_tests;

    use crate::components::message::ViewMessage;
    use desktop_i18n::I18n;

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_peripheral_appearance_cli_commands"),
        420,
        Fixture::default,
        |scene, app| {
            app.update_panel.show_cli_install_modal = true;
            app.update_panel.cli_install_modal_show_update_action = true;
            app.update_panel.cli_install_modal_show_install_action = true;
            app.update_panel.cli_install_modal_use_app_update_action = false;
            app.update_panel.cli_install_modal_is_checking_update = true;
            app.update_panel.cli_install_modal_title = "CLI 更新检测".into();
            app.update_panel.cli_install_modal_message = "下方命令可直接复制。".into();
            app.cli_commands_tab = CliCommandsTab::Common;
            let i18n = I18n::new(app.desktop_locale);
            let tabs = ["常用命令", "服务管理", "全部命令"].map(|label| i18n.__(label));
            scene.save_preview(app, view_app_update(app), "cli-commands");
            let message = scene.check_message(app, view_app_update(app), &tabs, 1, 14);
            assert!(matches!(
                message,
                Message::View(ViewMessage::CliCommandsTabSelected(CliCommandsTab::Service))
            ));
            let _ = app.update(message);
            assert_eq!(app.cli_commands_tab, CliCommandsTab::Service);
            let copy = scene.click_message(view_app_update(app), i18n.__("复制"));
            assert!(
                matches!(copy, Message::CopyCode(command) if command == "shiju service status")
            );
            scene.check_disabled(view_app_update(app), i18n.__("安装 CLI"));
            scene.check_modal_scroll(
                view_app_update(app),
                i18n.__("CLI 更新检测"),
                i18n.__("关闭"),
            );
            let close = scene.click_message(view_app_update(app), i18n.__("关闭"));
            assert!(matches!(close, Message::View(ViewMessage::CloseInstallCliModal)));
            let _ = app.update(close);
            assert!(!app.update_panel.show_cli_install_modal);
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn update_modals_hide_explanations_and_draw_hover_help_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::{Id, Operation};
    use iced::{Event, Font, Point, Rectangle, Size, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    #[derive(Default)]
    struct Labels(Vec<(String, Rectangle)>);
    impl Operation for Labels {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
            self.0.push((value.into(), bounds));
        }
    }

    let mut app = Fixture::default();
    app.update_panel.cli_install_modal_message = "当前已是最新版本。".into();
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 16.0.into(), Some("tiny-skia")).await.unwrap();
    for theme in [Theme::Light, Theme::Dark] {
        for app_update in [false, true] {
            app.update_panel.cli_install_modal_use_app_update_action = app_update;
            app.update_panel.cli_install_modal_title =
                if app_update { "检测更新" } else { "CLI 更新检测" }.into();
            let mut ui = UserInterface::build(
                app.view(),
                Size::new(960.0, 900.0),
                Cache::new(),
                &mut renderer,
            );
            let mut labels = Labels::default();
            ui.operate(&renderer, &mut labels);
            assert!(labels.0.iter().any(|(label, _)| label == "当前已是最新版本。"));
            assert!(labels.0.iter().all(|(label, _)| {
                !label.contains("通常无需手动安装")
                    && !label.contains("更新前会先检测")
                    && !label.contains("自动选择 DEB/RPM")
                    && !label.contains("在终端运行；点击标签页切换")
            }));
            let bounds = labels
                .0
                .iter()
                .find(|(label, _)| label == &app.update_panel.cli_install_modal_title)
                .unwrap()
                .1;
            let point = Point::new(bounds.x + bounds.width + 15.0, bounds.center_y());
            let style = iced::advanced::renderer::Style { text_color: theme.palette().text };
            ui.draw(&mut renderer, &theme, &style, mouse::Cursor::Unavailable);
            let idle = renderer.screenshot(Size::new(960, 900), 1.0, theme.palette().background);
            let cursor = mouse::Cursor::Available(point);
            let mut messages = Vec::new();
            ui.update(
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                cursor,
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            ui.draw(&mut renderer, &theme, &style, cursor);
            let opened = renderer.screenshot(Size::new(960, 900), 1.0, theme.palette().background);
            assert!(messages.is_empty());
            assert_ne!(idle, opened, "hover must draw the update explanation");
        }
    }
}
