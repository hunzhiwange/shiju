//! 工作空间配置接口由 agent-config 持有。
pub(crate) use agent_config::schema::workspace::{
    ConfigResolutionSource, default_config_and_workspace_dirs, resolve_runtime_config_dirs,
};
