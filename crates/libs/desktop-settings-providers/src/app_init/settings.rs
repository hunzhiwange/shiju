

pub fn build_embedding_routes_settings(
    full_agent_cfg: &config_aggregate::Config,
    gateway_cfg_result: &Result<config_gateway::GatewayConfig, String>,
) -> crate::app_init::state::EmbeddingRoutesSettingsState {
    crate::app_init::state::EmbeddingRoutesSettingsState {
        routes: full_agent_cfg
            .embedding_routes
            .iter()
            .map(|route| crate::app_init::state::EmbeddingRouteDraft {
                pattern: route.hint.clone(),
                provider: route.provider.clone(),
                model: route.model.clone(),
                dimensions: route.dimensions.map(|value| value.to_string()).unwrap_or_default(),
                api_key_input: route.api_key.clone().unwrap_or_default(),
            })
            .collect(),
        save_error: gateway_cfg_result
            .as_ref()
            .err()
            .map(desktop_i18n::settings_models::messages::Feedback::gateway),
        save_success: false,
    }
}
