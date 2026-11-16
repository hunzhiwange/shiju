use super::super::ui_driver::*;
use super::Event as Message;
use super::*;
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use iced::widget::{Space, button as action, column, row, text};
use iced::{Point, Size, Theme};
fn fixture() -> Fixture {
    let mut app = Fixture::default();
    app.settings.providers = vec![provider(
        "fixture",
        "測試供應商",
        (0..15)
            .map(|index| {
                model(
                    &format!("model-{index:02}"),
                    &if index == 13 {
                        "測試長名稱".repeat(10)
                    } else {
                        format!("測試模型 {index:02}")
                    },
                    true,
                )
            })
            .collect(),
    )];
    app.options.model_source_capabilities =
        vec![vw_api_acp_model::acp_model_source::AcpModelSourceCapability::Supported {
            source: AcpModelSource::SessionModel,
        }];
    app.options.connected = true;
    app.model = "fixture/model-00".into();
    app
}
fn overlay(app: &Fixture) -> Element<'_, Message> {
    let overlay = desktop_ui_controls::overlays::AboveOverlay::new(
        model_views::model_toggle_button(
            desktop_i18n::I18n::new(app.locale),
            &app.settings.providers,
            false,
            false,
            "fixture/model-00",
            true,
            Message::Model,
        ),
        app.popover(),
    )
    .show(true)
    .block_background_events(true)
    .gap(6.0)
    .on_close(Message::Agent);
    column![
        row![
            Space::new().width(Length::Fill),
            action(text("fixture send")).on_press(Message::Agent)
        ],
        Space::new().height(Length::Fill),
        overlay
    ]
    .padding(10)
    .into()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_model_results_keep_groups_and_actions() {
    if !isolated(&format!(
        "{}::ui_refresh_42_model_results_keep_groups_and_actions",
        module_path!()
    )) {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(640, 480);
    app.locale = DesktopLocale::English;
    app.settings.query = "model-13".into();
    for pending in [false, true] {
        app.pending = pending;
        let mut ui = scene(overlay(&app), &mut renderer, size);
        let group = label(&bounds(&mut ui, &renderer), "測試供應商");
        let row_y = group.y + group.height + 4.0 + 8.0 + 18.0;
        let point = Point::new(group.x + 25.0, row_y);
        let rest = capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("model-results-{pending}"),
            Point::ORIGIN,
        );
        hover(&mut ui, &mut renderer, point);
        let hovered = capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("model-hover-{pending}"),
            point,
        );
        hover(&mut ui, &mut renderer, Point::ORIGIN);
        let left = capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("model-left-{pending}"),
            Point::ORIGIN,
        );
        assert_eq!(rest, left);
        let messages = click(&mut ui, &mut renderer, point);
        if pending {
            assert_eq!(rest, hovered);
            assert!(messages.is_empty(), "{messages:?}");
        } else {
            assert_ne!(rest, hovered);
            assert!(matches!(
                messages.as_slice(),
                [Message::Model(model_views::Action::ModelSelected(model))] if model == "fixture/model-13"
            ));
            let info = Point::new(group.x + group.width - 11.0, row_y);
            assert!(matches!(
                click(&mut ui, &mut renderer, info).as_slice(),
                [Message::Model(model_views::Action::OpenSystemSettingsModelDetail(provider, model))]
                    if provider == "fixture" && model == "model-13"
            ));
        }
    }
    let i18n = I18n::new(app.locale);
    app.settings.providers.clear();
    for loading in [true, false] {
        app.settings.loading = loading;
        let mut ui = scene(overlay(&app), &mut renderer, size);
        let name = if loading {
            i18n.__("加载中…")
        } else {
            i18n.__("暂无可选模型（请先在系统设置里启用模型）")
        };
        inside(label(&bounds(&mut ui, &renderer), name), viewport(size));
        capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("model-loading-{loading}"),
            Point::ORIGIN,
        );
    }
}
