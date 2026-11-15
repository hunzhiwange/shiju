//! 环境配置运行时组合。
use super::{Config, config::{apply_named_model_provider_profile, apply_workspace_override}, proxy};
pub fn apply_env_overrides(config: &mut Config) {
    crate::schema::config_env::apply_env_overrides(config, &crate::schema::config_env::EnvironmentHost {
        apply_named_profile: apply_named_model_provider_profile,
        apply_workspace: apply_workspace_override,
        validate_proxy: proxy::validate_proxy_config,
        apply_proxy: proxy::apply_proxy_to_process_env,
        set_proxy: proxy::set_runtime_proxy_config,
        parse_proxy_enabled: proxy::parse_proxy_enabled,
        parse_proxy_scope: proxy::parse_proxy_scope,
    });
}
