//! agent 配置对象的扩展入口。
//!
//! 本模块重新导出共享配置结构，并在 agent 侧为 `Config` 补齐加载、保存、校验和环境变量覆盖能力。
//! 运行时归一化委托 agent-config，持久化入口暂由宿主组装。

use crate::runtime_schema::config_helpers::read_codex_openai_api_key;
use crate::runtime_schema::config_validate::normalize_wire_api;
use crate::schema::workspace::resolve_config_dir_for_workspace;

pub use config_aggregate::*;

use anyhow::Result;

/// `Config` 在 agent 运行时需要的持久化与校验能力。
///
/// trait 将共享配置类型与 agent 侧的 I/O、环境变量覆盖和校验实现连接起来。所有方法都返回
/// `anyhow::Result`，调用方应把错误展示为配置加载或保存失败，而不是继续使用部分初始化的配置。
pub trait ConfigExt {
    /// 从磁盘加载配置，不存在时初始化默认配置。
    ///
    /// 返回完整的 `Config`。加载过程中会处理旧格式迁移、密钥解密、环境变量覆盖和配置校验；任一阶段失败
    /// 都会返回错误。
    fn load_or_init() -> impl std::future::Future<Output = Result<Config>> + Send;

    /// 将当前配置保存到本地租户的共享数据库。
    ///
    /// 保存前由底层实现处理校验、密钥加密和 JSON payload 写入；数据库或序列化失败会作为错误返回。
    fn save(&self) -> impl std::future::Future<Output = Result<()>> + Send;

    /// 校验当前配置是否满足运行时约束。
    ///
    /// 返回值为 `Ok(())` 表示配置可用；字段为空、取值越界或安全策略无效时返回错误。
    fn validate(&self) -> Result<()>;

    /// 应用当前进程环境变量中的配置覆盖。
    ///
    /// 该方法只修改内存中的配置对象，不负责保存到磁盘。
    fn apply_env_overrides(&mut self);
}

impl ConfigExt for Config {
    fn load_or_init() -> impl std::future::Future<Output = Result<Config>> + Send {
        crate::runtime_schema::config_load::load_or_init_config()
    }

    fn save(&self) -> impl std::future::Future<Output = Result<()>> + Send {
        crate::runtime_schema::config_save::save_config(self)
    }

    fn validate(&self) -> Result<()> {
        crate::runtime_schema::config_validate::validate_config(self)
    }

    fn apply_env_overrides(&mut self) {
        crate::runtime_schema::config_env::apply_env_overrides(self);
    }
}

pub fn apply_named_model_provider_profile(config: &mut Config) {
    crate::schema::config::apply_named_model_provider_profile(config, read_codex_openai_api_key, normalize_wire_api);
}
pub fn apply_workspace_override(config: &mut Config, workspace: &str) {
    crate::schema::config::apply_workspace_override(config, workspace, resolve_config_dir_for_workspace);
}
