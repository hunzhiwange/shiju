use super::{catalog::CatalogRow, details};
use crate::app::{Message, message::ViewMessage, state::AppsCatalogState};
use iced::advanced::renderer::Headless;
use iced::{Event, Font, Pixels, Point, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::agent_apps::*};

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
            let mut ui = UserInterface::build(
                details::view(&state, &app),
                Size::new(620.0, 400.0),
                Cache::new(),
                &mut renderer,
            );
            let mut messages = vec![];
            for y in (10..390).step_by(20) {
                for x in (10..610).step_by(20) {
                    let position = Point::new(x as f32, y as f32);
                    let cursor = mouse::Cursor::Available(position);
                    ui.update(
                        &[
                            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                        ],
                        cursor,
                        &mut renderer,
                        &mut iced::advanced::clipboard::Null,
                        &mut messages,
                    );
                }
            }
            assert!(
                !messages.is_empty(),
                "Desktop details must offer manual installation even when discovery is unavailable"
            );
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
