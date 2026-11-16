//! 启动配置回填的宿主装配：保留编辑保护、身份检查与跨领域应用顺序。

use super::*;

pub(super) use desktop_settings_providers::app_init::settings::{build_embedding_routes_settings, build_model_routes_settings};



pub(super) use desktop_settings_runtime::app_init::settings::build_query_classification_settings;

pub(super) use desktop_settings_cron::app_init::settings::build_heartbeat_settings;

pub(super) use desktop_settings_cron::app_init::settings::build_cron_settings;

pub(super) use desktop_settings_cron::app_init::settings::build_scheduler_settings;

pub(super) use desktop_settings_runtime::app_init::settings::build_hooks_settings;

pub(super) use desktop_settings_runtime::app_init::settings::build_runtime_settings;



pub(super) use desktop_settings_network::app_init::settings::build_web_search_settings;

pub(super) use desktop_settings_network::app_init::settings::build_browser_settings;

pub(super) use desktop_settings_network::app_init::settings::build_http_request_settings;

pub(super) use desktop_settings_network::app_init::settings::build_gateway_settings;

pub(super) use desktop_settings_memory::app_init::settings::build_memory_settings;

pub(super) use desktop_settings_runtime::app_init::settings::build_reliability_settings;

pub(super) use desktop_settings_runtime::app_init::settings::build_multimodal_settings;

pub(super) use desktop_settings_security::app_init::settings::build_security_settings;



pub(super) use desktop_settings_runtime::app_init::settings::build_observability_settings;

pub(super) use desktop_settings_memory::app_init::settings::build_storage_settings;

pub(super) use desktop_settings_network::app_init::settings::build_proxy_settings;

pub(super) use desktop_settings_network::app_init::settings::build_tunnel_settings;

pub(super) fn apply_bootstrap_agent_config(
    app: &mut App,
    full_agent_cfg: &config_aggregate::Config,
    gateway: &config_gateway::GatewayConfig,
) {
    // Control state is read through its authenticated API, never inferred from config.
    #[cfg(not(target_arch = "wasm32"))]
    app.computer_control_settings.reset_for_endpoint(&crate::app::gateway_client_endpoint());
    let gateway_cfg_result = Ok(gateway.clone());

    #[cfg(not(target_arch = "wasm32"))]
    let scope = super::config::preferences_scope(&super::config::gateway_client_endpoint());
    macro_rules! apply_section {
        ($field:ident, $value:expr) => {
            #[cfg(not(target_arch = "wasm32"))]
            if !app.startup_configuration.setting_edited(&scope, stringify!($field)) {
                app.$field = $value;
            }
            #[cfg(target_arch = "wasm32")]
            {
                app.$field = $value;
            }
        };
    }

    apply_section!(
        embedding_routes_settings,
        build_embedding_routes_settings(full_agent_cfg, &gateway_cfg_result)
    );
    apply_section!(model_routes_settings, build_model_routes_settings(full_agent_cfg));
    apply_section!(
        query_classification_settings,
        build_query_classification_settings(full_agent_cfg)
    );
    apply_section!(heartbeat_settings, build_heartbeat_settings(full_agent_cfg));
    apply_section!(cron_settings, build_cron_settings(full_agent_cfg));
    apply_section!(scheduler_settings, build_scheduler_settings(full_agent_cfg));
    apply_section!(hooks_settings, build_hooks_settings(full_agent_cfg));
    apply_section!(runtime_settings, build_runtime_settings(full_agent_cfg));
    apply_section!(skills_settings, build_skills_settings(&full_agent_cfg.skills));
    apply_section!(web_search_settings, build_web_search_settings(full_agent_cfg));
    apply_section!(browser_settings, build_browser_settings(full_agent_cfg));
    apply_section!(http_request_settings, build_http_request_settings(full_agent_cfg));
    apply_section!(gateway_settings, build_gateway_settings(&gateway_cfg_result));
    apply_section!(memory_settings, build_memory_settings(full_agent_cfg));
    apply_section!(
        channels_settings,
        super::settings_builders::build_channels_settings(&full_agent_cfg.channels_config)
    );
    apply_section!(reliability_settings, build_reliability_settings(full_agent_cfg));
    apply_section!(multimodal_settings, build_multimodal_settings(full_agent_cfg));
    apply_section!(security_settings, build_security_settings(full_agent_cfg));
    apply_section!(autonomy_settings, crate::app::state::AutonomySettingsState::from_config(&full_agent_cfg.autonomy));
    apply_section!(observability_settings, build_observability_settings(full_agent_cfg));
    apply_section!(storage_settings, build_storage_settings(full_agent_cfg));
    apply_section!(proxy_settings, build_proxy_settings(full_agent_cfg));
    apply_section!(tunnel_settings, build_tunnel_settings(full_agent_cfg));
}
