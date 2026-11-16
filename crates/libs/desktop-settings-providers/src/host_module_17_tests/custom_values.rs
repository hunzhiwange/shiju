use crate::desktop_split_11_tests::custom_model::*;
use crate::provider_state::{CustomProviderTab, ModelCapabilityDefaults};
use crate::provider_view_test_support::Fixture as App;
use crate::state::CustomProviderModelTab;
use crate::view_message::{Message, SettingsMessage};
use iced::{Renderer, Size, Theme, mouse};
fn provider_update(app: &mut App, message: crate::provider_message::SettingsMessage) {
    let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1);
    let _ = crate::provider_update::update(
        &mut app.provider_settings,
        &mut crate::state::ModelSettingsState::default(),
        endpoint.clone(),
        desktop_config::ConfigAccess::new(endpoint, None),
        false,
        message,
    );
}
fn apply(app: &mut App, messages: Vec<Message>) {
    assert_eq!(messages.len(), 1, "expected one settings message");
    let Message::Settings(message) = messages.into_iter().next().unwrap() else {
        panic!("expected settings message")
    };
    use crate::provider_message::SettingsMessage as Local;
    let message = match message {
        SettingsMessage::CustomProviderModelClose => Local::CustomProviderModelClose,
        SettingsMessage::CustomProviderModelFilterSelected(v0) => {
            Local::CustomProviderModelFilterSelected(v0)
        }
        SettingsMessage::CustomProviderModelModalContextLimitChanged(v0) => {
            Local::CustomProviderModelModalContextLimitChanged(v0)
        }
        SettingsMessage::CustomProviderModelModalIdChanged(v0) => {
            Local::CustomProviderModelModalIdChanged(v0)
        }
        SettingsMessage::CustomProviderModelModalInputModalityToggled(v0, v1) => {
            Local::CustomProviderModelModalInputModalityToggled(v0, v1)
        }
        SettingsMessage::CustomProviderModelModalReasoningChanged(v0) => {
            Local::CustomProviderModelModalReasoningChanged(v0)
        }
        SettingsMessage::CustomProviderModelModalSave => Local::CustomProviderModelModalSave,
        SettingsMessage::CustomProviderModelOpen(v0) => Local::CustomProviderModelOpen(v0),
        SettingsMessage::CustomProviderModelsDiscover => Local::CustomProviderModelsDiscover,
        SettingsMessage::CustomProviderSave => Local::CustomProviderSave,
        SettingsMessage::CustomProviderModelModalInputLimitChanged(v0) => {
            Local::CustomProviderModelModalInputLimitChanged(v0)
        }
        SettingsMessage::CustomProviderModelModalOutputLimitChanged(v0) => {
            Local::CustomProviderModelModalOutputLimitChanged(v0)
        }
        SettingsMessage::CustomProviderModelTabSelected(v0) => {
            Local::CustomProviderModelTabSelected(v0)
        }
        _ => panic!("unexpected model editor event"),
    };
    provider_update(app, message);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_09_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.provider_settings.custom_provider_tab = CustomProviderTab::Models;
    app.provider_settings.custom.models[0].inherited_capabilities = Some(ModelCapabilityDefaults {
        reasoning: true,
        tool_call: true,
        input_modalities: vec!["text".into(), "image".into()],
        ..Default::default()
    });

    app.provider_settings.custom_models_discovering = true;
    let mut scene = narrow(&app, &mut renderer);
    let discover = scene.bounds(&scalar_control(0, &[1]));
    let save = scene.bounds(&modal(&[2, 2]));
    for button in [discover, save] {
        assert!(scene.click(&mut renderer, button.center()).is_empty());
    }
    scene.capture(&mut renderer, &Theme::Dark, cursor(discover), "provider-models-discovering");
    drop(scene);
    app.provider_settings.custom_models_discovering = false;
    let mut scene = narrow(&app, &mut renderer);
    let discover = scene.bounds(&scalar_control(0, &[1]));
    assert!(matches!(
        scene.click(&mut renderer, discover.center()).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelsDiscover)]
    ));
    let add = scene.bounds(&scalar_control(0, &[2]));
    let edit = scene.bounds(&group(&[4, 0, 2]));
    let filter = scene.bounds(&scalar_control(2, &[2]));
    let filtered = scene.click(&mut renderer, filter.center());
    assert!(matches!(
        filtered.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelFilterSelected(
            crate::provider_state::CustomProviderModelFilter::Disabled
        ))]
    ));
    let add_messages = scene.click(&mut renderer, add.center());
    assert!(matches!(
        add_messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelOpen(None))]
    ));
    let body = scene.bounds(&modal(&[1]));
    let offset = scene.scroll(&mut renderer, body, 400.0);
    let edit_point = edit.center() - Vector::new(0.0, offset);
    assert!(body.contains(edit_point), "the model edit button must scroll into view");
    let hover = mouse::Cursor::Available(edit_point);
    let normal = scene.capture(
        &mut renderer,
        &Theme::Dark,
        mouse::Cursor::Unavailable,
        "provider-models-scrolled",
    );
    assert!(
        scene
            .send(
                &mut renderer,
                &[Event::Mouse(mouse::Event::CursorMoved { position: edit_point })],
                hover
            )
            .is_empty()
    );
    let hovered = scene.capture(&mut renderer, &Theme::Dark, hover, "provider-model-edit-tooltip");
    assert!(normal != hovered, "model actions must respond to hover after scrolling");
    let edit_messages = scene.click(&mut renderer, edit_point);
    assert!(matches!(
        edit_messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelOpen(Some(0)))]
    ));
    drop(scene);

    apply(&mut app, add_messages);
    let mut scene = narrow(&app, &mut renderer);
    let save = scene.bounds(&modal(&[2, 2]));
    let id = scene.bounds(&scalar_control(0, &[]));
    assert!(scene.click(&mut renderer, id.center()).is_empty());
    assert!(
        scene.send(&mut renderer, &[key(Named::Enter), key(Named::Escape)], cursor(id)).is_empty()
    );
    let invalid_save = scene.click(&mut renderer, save.center());
    drop(scene);
    apply(&mut app, invalid_save);
    assert!(app.provider_settings.custom_model_modal.is_some());
    assert!(app.provider_settings.save_error.is_some());
    let mut scene = narrow(&app, &mut renderer);
    scene.capture(&mut renderer, &Theme::Light, mouse::Cursor::Unavailable, "model-required-id");
    let id = scene.bounds(&scalar_control(0, &[]));
    let id_messages = scene.edit(&mut renderer, id, "fixture-added");
    assert!(matches!(
        id_messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalIdChanged(id))]
            if id == "fixture-added"
    ));
    drop(scene);
    apply(&mut app, id_messages);
    save_model(&mut app, &mut renderer);
    assert_eq!(app.provider_settings.custom.models[1].model_id, "fixture-added");
    assert!(app.provider_settings.custom.models[1].tool_call.is_none());

    apply(&mut app, edit_messages);
    let mut scene = narrow(&app, &mut renderer);
    let capability_tab = scene.bounds(&modal(&[0, 1, 2]));
    let messages = scene.click(&mut renderer, capability_tab.center());
    drop(scene);
    apply(&mut app, messages);
    assert_eq!(
        app.provider_settings.custom_model_modal.as_ref().unwrap().active_tab,
        CustomProviderModelTab::Capabilities
    );
    let mut scene = narrow(&app, &mut renderer);
    let reasoning = scene.bounds(&group(&[0, 0, 1, 0]));
    let last = scene.bounds(&group(&[0, 0, 1, 4]));
    let options = scene.bounds(&group(&[0, 0, 1]));
    assert!(last.y > reasoning.y && last.x + last.width <= options.x + options.width);
    let hover = Event::Mouse(mouse::Event::CursorMoved { position: last.center() });
    assert!(scene.send(&mut renderer, &[hover], cursor(last)).is_empty());
    scene.capture(&mut renderer, &Theme::Dark, cursor(last), "model-capability-hover");
    let messages = scene.click(&mut renderer, reasoning.center());
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalReasoningChanged(false))]
    ));
    let image = scene.bounds(&group(&[2, 0, 0, 1, 1]));
    let modalities = scene.click(&mut renderer, image.center());
    assert!(matches!(
        modalities.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalInputModalityToggled(modality, false))]
            if modality == "image"
    ));
    let limits = scene.bounds(&modal(&[0, 1, 1]));
    let tab_messages = scene.click(&mut renderer, limits.center());
    drop(scene);
    apply(&mut app, messages);
    apply(&mut app, modalities);
    apply(&mut app, tab_messages);

    let mut scene = narrow(&app, &mut renderer);
    let limit = scene.bounds(&scalar_control(0, &[]));
    let messages = scene.edit(&mut renderer, limit, "32768");
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalContextLimitChanged(value))]
            if value == "32768"
    ));
    drop(scene);
    apply(&mut app, messages);
    save_model(&mut app, &mut renderer);
    let saved = &app.provider_settings.custom.models[0];
    assert!(saved.model_id.starts_with("fixture-model-with-"));
    assert_eq!(saved.context_limit, "32768");
    assert_eq!(saved.reasoning, Some(false));
    assert!(saved.tool_call.is_none() && saved.input_limit.is_empty());
    assert_eq!(saved.input_modalities.as_deref().unwrap(), ["text"]);
    assert!(saved.output_modalities.is_none());
    assert!(saved.inherited_capabilities.as_ref().unwrap().tool_call);

    let _ = provider_update(
        &mut app,
        crate::provider_message::SettingsMessage::CustomProviderModelOpen(Some(0)),
    );
    let mut scene = narrow(&app, &mut renderer);
    let cancel = scene.bounds(&modal(&[2, 1]));
    let messages = scene.click(&mut renderer, cancel.center());
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelClose)]
    ));
    drop(scene);
    apply(&mut app, messages);
    assert!(app.provider_settings.custom_provider_modal_open);
    assert!(app.provider_settings.custom_model_modal.is_none());

    apply(&mut app, filtered);
    app.provider_settings.custom_editing_provider_id = Some("fixture".into());
    screenshot(&app, &mut renderer, &Theme::Light, Size::new(420, 620), "provider-models-empty");
    let _ = provider_update(
        &mut app,
        crate::provider_message::SettingsMessage::CustomProviderSaveDone(Err(
            "Fixture save failed".into(),
        )),
    );
    let mut scene = narrow(&app, &mut renderer);
    let save = scene.bounds(&modal(&[2, 2]));
    scene.capture(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "provider-save-failed");
    assert!(matches!(
        scene.click(&mut renderer, save.center()).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderSave)]
    ));
}

fn save_model(app: &mut App, renderer: &mut Renderer) {
    let mut scene = narrow(app, renderer);
    let save = scene.bounds(&modal(&[2, 2]));
    let messages = scene.click(renderer, save.center());
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalSave)]
    ));
    drop(scene);
    apply(app, messages);
    assert!(
        app.provider_settings.custom_model_modal.is_none(),
        "{:?}",
        app.provider_settings.save_error
    );
}

use iced::{Event, keyboard::key::Named};
impl Scene<'_> {
    fn assert_body_text(&mut self, renderer: &Renderer, value: &str, visible: bool) {
        #[derive(Default)]
        struct Labels(Vec<String>);
        impl iced::advanced::widget::Operation for Labels {
            fn traverse(
                &mut self,
                children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
            ) {
                children(self);
            }
            fn text(
                &mut self,
                _: Option<&iced::advanced::widget::Id>,
                _: iced::Rectangle,
                value: &str,
            ) {
                self.0.push(value.into());
            }
        }
        let mut labels = Labels::default();
        self.ui.operate(renderer, &mut labels);
        assert_eq!(
            labels.0.iter().any(|item| item.contains(value)),
            visible,
            "body visibility for {value:?}"
        );
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_help_gap_03_preserves_masks_and_validation() {
    use desktop_i18n::settings_models::connection_text::text as tr;

    let mut renderer = renderer().await;
    let mut app = fixture();
    let size = Size::new(420, 620);
    app.provider_settings.custom_provider_tab = CustomProviderTab::Authentication;
    let frame = |app: &App, renderer: &mut Renderer| {
        Scene::new(view(app), renderer, size).capture(
            renderer,
            &app.app_theme,
            mouse::Cursor::Unavailable,
            "",
        )
    };
    let masked = frame(&app, &mut renderer);
    app.provider_settings.custom.api_key = "zz-fixture".into();
    assert!(frame(&app, &mut renderer) == masked, "same length keys draw the same mask");
    app.provider_settings.custom.api_key.push('x');
    assert!(frame(&app, &mut renderer) != masked, "the input draws the longer mask");

    app.provider_settings.custom_provider_tab = CustomProviderTab::Basic;
    app.provider_settings.custom.provider_id = "INVALID".into();
    let mut provider = Scene::new(view(&app), &mut renderer, size);
    let save = provider.bounds(&modal(&[2, 2])).center();
    let messages = provider.click(&mut renderer, save);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderSave)]
    ));
    drop(provider);
    apply(&mut app, messages);
    let mut provider = Scene::new(view(&app), &mut renderer, size);
    provider.assert_body_text(
        &renderer,
        tr(app.desktop_locale, "提供商 ID 仅支持小写字母、数字、连字符或下划线"),
        true,
    );
    provider.capture(&mut renderer, &app.app_theme, mouse::Cursor::Unavailable, "");
    drop(provider);

    app.provider_settings.custom.provider_id = "fixture".into();
    apply(&mut app, vec![Message::Settings(SettingsMessage::CustomProviderModelOpen(Some(0)))]);
    apply(
        &mut app,
        vec![Message::Settings(SettingsMessage::CustomProviderModelTabSelected(
            CustomProviderModelTab::Limits,
        ))],
    );
    let mut model = Scene::new(view(&app), &mut renderer, size);
    let input = model.bounds(&scalar_control(2, &[]));
    let input_changed = model.edit(&mut renderer, input, "0");
    let output = model.bounds(&scalar_control(4, &[]));
    let output_changed = model.edit(&mut renderer, output, "0");
    assert!(matches!(
        input_changed.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalInputLimitChanged(value))]
            if value == "0"
    ));
    assert!(matches!(
        output_changed.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalOutputLimitChanged(value))]
            if value == "0"
    ));
    let save = model.bounds(&modal(&[2, 2])).center();
    let messages = model.click(&mut renderer, save);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalSave)]
    ));
    drop(model);
    apply(&mut app, input_changed);
    apply(&mut app, output_changed);
    apply(&mut app, messages);
    assert!(app.provider_settings.custom_model_modal.is_some());
    let mut model = Scene::new(view(&app), &mut renderer, size);
    model.assert_body_text(&renderer, "input limit must be a positive integer", true);
    let input = model.bounds(&scalar_control(2, &[]));
    assert!(model.click(&mut renderer, input.center()).is_empty());
    let clear = model.send(&mut renderer, &[key(Named::End), key(Named::Backspace)], cursor(input));
    assert!(matches!(
        clear.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalInputLimitChanged(value))]
            if value.is_empty()
    ));
    let save = model.bounds(&modal(&[2, 2])).center();
    let messages = model.click(&mut renderer, save);
    drop(model);
    apply(&mut app, clear);
    apply(&mut app, messages);
    assert!(app.provider_settings.custom_model_modal.is_none());
    assert_eq!(app.provider_settings.custom.models[0].output_limit, "0");
}
