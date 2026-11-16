use model_types::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelLimit, ProviderSource,
};
use std::collections::HashMap;
#[derive(Default)]
struct Fixture {
    providers: crate::provider_state::ProviderSettingsState,
    model_settings: crate::state::ModelSettingsState,
}
fn update(app: &mut Fixture, message: crate::provider_message::SettingsMessage) {
    let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1);
    let _ = crate::provider_update::update(
        &mut app.providers,
        &mut app.model_settings,
        endpoint.clone(),
        desktop_config::ConfigAccess::new(endpoint, None),
        false,
        message,
    );
}
fn model(provider_id: &str, id: &str, name: &str, status: &str) -> Model {
    Model {
        id: id.to_string(),
        provider_id: provider_id.to_string(),
        api: ApiInfo {
            protocols: None,
            id: id.to_string(),
            url: "https://example.test".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: name.to_string(),
        family: None,
        capabilities: Capabilities {
            native_tool_search: false,
            catalog_execution: None,
            temperature: true,
            reasoning: false,
            reasoning_options: None,
            attachment: false,
            toolcall: true,
            input: CapabilityIO { text: true, audio: false, image: true, video: false, pdf: true },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Field { field: "content".to_string() },
        },
        cost: ModelCost {
            input: 1.0,
            output: 2.0,
            cache: ModelCostCache { read: 0.5, write: 0.75 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 200_000, input: Some(100_000), output: 20_000 },
        status: status.to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "2026-01-01".to_string(),
        variants: HashMap::new(),
    }
}

fn provider(
    id: &str,
    name: &str,
    source: ProviderSource,
    key: Option<&str>,
    models: Vec<Model>,
) -> Info {
    Info {
        id: id.to_string(),
        name: name.to_string(),
        source,
        env: Vec::new(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: models.into_iter().map(|model| (model.id.clone(), model)).collect(),
    }
}

#[test]
fn provider_changes_update_available_models_without_opening_model_settings() {
    let mut app = Fixture::default();
    for message in [
        crate::provider_message::SettingsMessage::CustomProviderSaveDone
            as fn(
                crate::provider_message::ProvidersLoaded,
            ) -> crate::provider_message::SettingsMessage,
        crate::provider_message::SettingsMessage::ProviderConnectDone,
        crate::provider_message::SettingsMessage::ProvidersRefreshed,
        crate::provider_message::SettingsMessage::ProviderModelsSyncDone,
        crate::provider_message::SettingsMessage::ProviderDisconnectDone,
    ] {
        let providers = HashMap::from([(
            "custom".to_string(),
            provider(
                "custom",
                "Custom",
                ProviderSource::Api,
                None,
                vec![model("custom", "latest", "Latest", "active")],
            ),
        )]);
        let _ = update(&mut app, message(Ok((providers, Vec::new(), false, Vec::new()))));
        assert_eq!(app.model_settings.providers[0].models[0].id, "latest");
        assert!(app.model_settings.providers[0].models[0].enabled);

        let _ = update(&mut app, message(Ok((HashMap::new(), Vec::new(), false, Vec::new()))));
        assert!(app.model_settings.providers.is_empty());
    }
}
