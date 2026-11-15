use super::core::Agent;
use crate::app::agent::config::Config;
use anyhow::Result;

/// 运行代理会话的便捷函数
///
/// 这是最顶层的代理运行入口，支持单次消息处理或交互式会话。
/// 根据 `message` 参数是否提供，自动选择运行模式：
/// - 提供消息：运行单次对话
/// - 未提供消息：进入交互式 REPL 模式（非 WASM 平台）
///
/// # 参数
///
/// * `config` - 代理配置对象
/// * `message` - 可选的用户消息。`None` 时进入交互模式
/// * `provider_override` - 可选的提供商覆盖，用于覆盖配置中的默认提供商
/// * `model_override` - 可选的模型覆盖，用于覆盖配置中的默认模型
/// * `temperature` - 温度参数，用于覆盖配置中的默认温度
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误
///
/// # 平台差异
///
/// - 在非 WASM 平台：支持交互式 REPL 模式
/// - 在 WASM 平台：不支持交互模式，调用时将返回错误
///
/// # 可观测性
///
/// 该函数会自动记录代理会话的开始和结束事件，包括：
/// - 提供商和模型信息
/// - 会话持续时间
/// - 使用的令牌数（如果可用）
/// - 成本（如果可用）
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_window::app::agent::agent::run;
/// use vibe_window::app::agent::config::Config;
///
/// let config = Config::load("config.toml")?;
///
/// run(
///     config,
///     Some("请帮我写一个排序函数".to_string()),
///     None,
///     None,
///     0.7,
/// ).await?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub async fn run(
    config: Config,
    message: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    temperature: f64,
) -> Result<String> {
    run_with_agent_factory(
        config,
        message,
        provider_override,
        model_override,
        temperature,
        Agent::from_config,
    )
    .await
}

pub(super) async fn run_with_agent_factory(
    config: Config,
    message: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    temperature: f64,
    agent_factory: impl FnOnce(&Config) -> Result<Agent>,
) -> Result<String> {
    agent_loop::run::run_with_agent_factory(
        config, message, provider_override, model_override, temperature, agent_factory,
    ).await
}
