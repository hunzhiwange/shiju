use super::page_fixture::details;
use super::page_fixture::{AppsCatalogState, Message, ViewMessage};
use crate::system_settings_apps::catalog::CatalogRow;
use gateway_http::AgentAppsReadError;
use iced::advanced::renderer::Headless;
use iced::{Event, Font, Pixels, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_api_app_identity::{
    AppCatalogEntry, AppDesktopObservation, AppId, AppReadResponse, AppRuntimeKind,
};

#[tokio::test]
async fn cc_apps_desktop_manual_guide_click_opens_only_official_client_url() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(14.0), Some("tiny-skia"))
            .await
            .unwrap();
    let state = AppsCatalogState::default();
    for theme in [Theme::Dark, Theme::Light] {
        for paths in [None, Some(vec![]), Some(vec!["/隔离/Claude.app".into()])] {
            let app = CatalogRow {
                entry: AppCatalogEntry {
                    id: AppId::ClaudeDesktop,
                    name: "Claude Desktop".into(),
                    cli_command: None,
                    runtime_kinds: vec![AppRuntimeKind::DesktopApp],
                },
                observation: match paths {
                    Some(paths) => Ok(AppReadResponse {
                        installations: vec![],
                        desktop: Some(AppDesktopObservation {
                            supported: true,
                            app_bundles: Some(paths),
                            config_directories: vec![],
                        }),
                    }),
                    None => Err(AgentAppsReadError::Transport("offline".into())),
                },
            };
            let size = Size::new(620.0, 600.0);
            let mut element = details::view(&state, &app);
            let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &iced::advanced::layout::Limits::new(Size::ZERO, size),
            );
            // The manual download button is the final child of the details column.
            // Target it directly: the page also offers a separate discovery refresh.
            let button = iced::advanced::Layout::new(&node)
                .children()
                .next()
                .expect("details container must contain a column")
                .children()
                .last()
                .expect("details must offer a manual download button")
                .bounds();
            assert!(button.y + button.height <= size.height);
            let mut ui = UserInterface::build(element, size, Cache::new(), &mut renderer);
            let mut messages = vec![];
            ui.update(
                &[
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(button.center()),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            assert!(
                !messages.is_empty(),
                "Desktop details must offer manual installation even when discovery is unavailable"
            );
            assert_eq!(messages.len(), 1, "one guide click must emit exactly one action");
            for message in messages {
                assert!(
                    matches!(message, Message::View(ViewMessage::OpenUrlExternal(ref url)) if url == "https://claude.com/download"),
                    "manual guide must open a fixed official URL on the client, never execute lifecycle actions"
                );
            }
            ui.draw(
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style::default(),
                mouse::Cursor::Unavailable,
            );
        }
    }
}
