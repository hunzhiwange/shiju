//! 运行时会话命令处理模块。
//!
//! 本模块负责处理通道消息中的会话级命令，例如查看或切换 provider/model、
//! 重置会话，以及进入任务模式。命令处理会更新发送者维度的路由选择，
//! 并在影响上下文连续性时清理会话历史，避免新配置沿用旧上下文造成混淆。

use super::super::*;
use super::task_mode::set_sender_task_mode;

#[cfg(test)]
tokio::task_local! {
    static TEST_PROVIDER_INIT_ERROR: Option<String>;
}

#[cfg(test)]
pub(super) async fn with_test_provider_init_error<T>(
    err: impl Into<String>,
    future: impl std::future::Future<Output = T>,
) -> T {
    TEST_PROVIDER_INIT_ERROR.scope(Some(err.into()), future).await
}

async fn get_or_create_session_provider(
    ctx: &ChannelRuntimeContext,
    provider_name: &str,
) -> anyhow::Result<std::sync::Arc<dyn crate::app::agent::providers::Provider>> {
    #[cfg(test)]
    if let Ok(Some(err)) = TEST_PROVIDER_INIT_ERROR.try_with(Clone::clone) {
        anyhow::bail!(err);
    }

    get_or_create_provider(ctx, provider_name).await
}

/// 构建当前 provider 列表与选中状态的帮助文本。
///
/// # 参数
/// - `current`: 当前发送者会话的路由选择。
///
/// # 返回值
/// 返回可直接发送给用户的 provider 帮助文本。
pub(super) fn handle_show_providers(current: &ChannelRouteSelection) -> String {
    build_providers_help_response(current)
}

/// 切换当前发送者会话使用的 provider。
///
/// # 参数
/// - `ctx`: 通道运行时上下文，用于创建 provider、保存路由状态和清理历史。
/// - `sender_key`: 当前发送者的稳定会话键。
/// - `current`: 当前路由选择，会在 provider 切换成功时被更新。
/// - `raw_provider`: 用户输入的 provider 名称或别名。
///
/// # 返回值
/// 返回用户可读的切换结果。provider 初始化失败或名称未知时返回错误说明文本。
///
/// # 错误处理
/// 本函数不向外传播错误；provider 初始化错误会先脱敏，再写入返回文本。
pub(super) async fn handle_set_provider(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
    current: &mut ChannelRouteSelection,
    raw_provider: String,
) -> String {
    match resolve_provider_alias(&raw_provider) {
        Some(provider_name) => match get_or_create_session_provider(ctx, &provider_name).await {
            Ok(_) => {
                if provider_name != current.provider {
                    current.provider = provider_name.clone();
                    set_route_selection(ctx, sender_key, current.clone());
                    // provider 变化后旧对话可能包含不兼容的模型假设，清理历史能让新路由从干净上下文开始。
                    clear_sender_history(ctx, sender_key);
                }

                format!(
                    "Provider switched to `{provider_name}` for this sender session. Current model is `{}`.\nUse `/model <model-id>` to set a provider-compatible model.",
                    current.model
                )
            }
            Err(err) => {
                // provider 错误可能包含 API 返回内容，展示前必须脱敏，避免令牌或敏感载荷进入聊天。
                let safe_err = crate::app::agent::providers::sanitize_api_error(&err.to_string());
                format!(
                    "Failed to initialize provider `{provider_name}`. Route unchanged.\nDetails: {safe_err}"
                )
            }
        },
        None => {
            format!("Unknown provider `{raw_provider}`. Use `/providers` to list valid providers.")
        }
    }
}

/// 构建当前 provider 下的模型帮助文本。
///
/// # 参数
/// - `current`: 当前发送者会话的路由选择。
/// - `workspace_dir`: 用于读取本地模型配置或提示的工作区目录。
///
/// # 返回值
/// 返回可直接发送给用户的模型帮助文本。
pub(super) fn handle_show_model(current: &ChannelRouteSelection, workspace_dir: &Path) -> String {
    build_models_help_response(current, workspace_dir)
}

/// 切换当前发送者会话使用的模型。
///
/// # 参数
/// - `ctx`: 通道运行时上下文，用于保存路由选择和清理历史。
/// - `sender_key`: 当前发送者的稳定会话键。
/// - `current`: 当前路由选择，会在模型非空时被更新。
/// - `raw_model`: 用户输入的模型 ID，可带反引号。
///
/// # 返回值
/// 返回用户可读的切换结果；模型 ID 为空时返回提示文本。
pub(super) fn handle_set_model(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
    current: &mut ChannelRouteSelection,
    raw_model: String,
) -> String {
    let model = raw_model.trim().trim_matches('`').trim().to_string();
    if model.is_empty() {
        "Model ID cannot be empty. Use `/model <model-id>`.".to_string()
    } else {
        current.model = model.clone();
        set_route_selection(ctx, sender_key, current.clone());
        // 模型切换会改变后续推理能力和上下文解释方式，因此重置历史保持行为确定。
        clear_sender_history(ctx, sender_key);

        format!(
            "Model switched to `{model}` for provider `{}` in this sender session.",
            current.provider
        )
    }
}

/// 重置当前发送者的普通会话。
///
/// # 参数
/// - `ctx`: 通道运行时上下文，用于清除会话 ID、历史和任务模式状态。
/// - `msg`: 触发命令的通道消息，用于定位发送者会话。
/// - `sender_key`: 当前发送者的稳定会话键。
///
/// # 返回值
/// 返回会话已重置的用户提示文本。
pub(super) async fn handle_new_session(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
) -> String {
    clear_sender_session_id(ctx, msg).await;
    clear_sender_history(ctx, sender_key);
    set_sender_task_mode(ctx, sender_key, false);
    "会话已重置. 开始新会话.".to_string()
}

/// 重置当前发送者并进入任务模式。
///
/// # 参数
/// - `ctx`: 通道运行时上下文，用于清除原会话并开启任务模式。
/// - `msg`: 触发命令的通道消息，用于定位发送者会话。
/// - `sender_key`: 当前发送者的稳定会话键。
///
/// # 返回值
/// 返回任务模式已开启的用户提示文本。
pub(super) async fn handle_task_mode(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
) -> String {
    clear_sender_session_id(ctx, msg).await;
    clear_sender_history(ctx, sender_key);
    set_sender_task_mode(ctx, sender_key, true);
    "我进入了任务模式。".to_string()
}
