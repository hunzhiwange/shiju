//! 组织桌面应用初始化阶段的 load.rs 逻辑。
//! 本模块把启动输入、配置加载和初始状态装配拆开，便于定位启动失败路径。

use super::state::NewAppInit;
use super::*;

/// 模块内可见函数，执行 load_new_app_init 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn load_new_app_init() -> NewAppInit {
    #[cfg(not(target_arch = "wasm32"))]
    let cfg = config::load_app_config();
    #[cfg(target_arch = "wasm32")]
    let cfg = serde_json::json!({});

    #[cfg(not(target_arch = "wasm32"))]
    let system_settings_cfg = config::load_system_settings_config();
    #[cfg(target_arch = "wasm32")]
    let system_settings_cfg = vw_config_types::ui::AppSystemSettingsConfig::default();

    let now = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
        }
        #[cfg(target_arch = "wasm32")]
        {
            web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).unwrap_or_default()
        }
    };

    let init_secs = now.as_secs() as i64;
    let init_ms = now.as_millis();
    let init_utc = crate::app::message::timestamp_tool::format_utc(init_secs);

    let gateway_client_cfg = config::load_gateway_client_config();

    #[cfg(not(target_arch = "wasm32"))]
    let full_agent_cfg = config::load_full_agent_config();
    #[cfg(target_arch = "wasm32")]
    let full_agent_cfg = vw_config_types::config::Config::default();

    #[cfg(not(target_arch = "wasm32"))]
    let global_acp_cfg = config::load_enabled_acp_config_result().unwrap_or_else(|err| {
        if vw_gateway_client::is_gateway_unavailable_error(&err) {
            tracing::debug!(target: "vw_desktop", error = %err, "failed to load ACP config via gateway");
        } else {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to load ACP config via gateway");
        }
        full_agent_cfg.acp.clone()
    });
    #[cfg(target_arch = "wasm32")]
    let global_acp_cfg = full_agent_cfg.acp.clone();

    #[cfg(not(target_arch = "wasm32"))]
    let gateway_cfg_result = config::load_gateway_config_result();
    #[cfg(target_arch = "wasm32")]
    let gateway_cfg_result = Ok(full_agent_cfg.gateway.clone());

    NewAppInit {
        cfg,
        system_settings_cfg,
        gateway_client_cfg,
        full_agent_cfg,
        global_acp_cfg,
        gateway_cfg_result,
        init_secs,
        init_ms,
        init_utc,
    }
}
#[cfg(test)]
#[path = "load_tests.rs"]
mod load_tests;
