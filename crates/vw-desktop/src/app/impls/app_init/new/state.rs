//! 组织桌面应用初始化阶段的 state.rs 逻辑。
//! 本模块把启动输入、配置加载和初始状态装配拆开，便于定位启动失败路径。

use std::collections::HashMap;

/// 模块内可见结构体，承载 NewAppInit 对应的状态数据。
/// 字段保持与相邻业务流程和序列化格式一致。
pub(super) struct NewAppInit {
    pub(super) cfg: serde_json::Value,
    pub(super) system_settings_cfg: vw_config_types::ui::AppSystemSettingsConfig,
    pub(super) gateway_client_cfg: vw_config_types::ui::GatewayClientSystemSettingsConfig,
    pub(super) full_agent_cfg: vw_config_types::config::Config,
    pub(super) global_acp_cfg: HashMap<String, vw_config_types::config::AcpAgentConfig>,
    pub(super) gateway_cfg_result: Result<vw_config_types::gateway::GatewayConfig, String>,
    pub(super) init_secs: i64,
    pub(super) init_ms: u128,
    pub(super) init_utc: String,
}
#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
