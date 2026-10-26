//! 运行时会话命令处理模块。
//!
//! 本模块负责处理通道消息中的会话级命令，例如查看或切换 provider/model、
//! 重置会话，以及进入任务模式。命令处理会更新发送者维度的路由选择，
//! 并在影响上下文连续性时清理会话历史，避免新配置沿用旧上下文造成混淆。

use super::super::*;
use super::session_lifecycle::{invalidate_archive, persist_current_route};
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
        Err(error) => format!("模型目录不可用：{error}"),
        Ok(Some(provider_name)) => {
            match get_or_create_session_provider(ctx, &provider_name).await {
                Ok(_) => {
                    if provider_name != current.provider {
                        let mut next = current.clone();
                        next.provider = provider_name.clone();
                        next.reasoning_effort = None;
                        if persist_current_route(ctx, &next).await.is_err() {
                            return "模型选项保存失败，路由与历史保持不变。".into();
                        }
                        *current = next;
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
                    let safe_err =
                        crate::app::agent::providers::sanitize_api_error(&err.to_string());
                    format!(
                        "Failed to initialize provider `{provider_name}`. Route unchanged.\nDetails: {safe_err}"
                    )
                }
            }
        }
        Ok(None) => {
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
pub(super) async fn handle_set_model(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
    current: &mut ChannelRouteSelection,
    raw_model: String,
) -> String {
    let model = raw_model.trim().trim_matches('`').trim().to_string();
    if model.is_empty() {
        "Model ID cannot be empty. Use `/model <model-id>`.".to_string()
    } else {
        let declared = match provider::get_model(&current.provider, &model).await {
            Ok(model) => model,
            Err(provider::CatalogError::Configuration(error)) => return format!("模型配置读取失败：{error}"),
            Err(provider::CatalogError::ModelNotFound(_)) => return "模型未在当前 provider 目录中声明。使用 /models 查看可用模型；路由与历史保持不变。".into(),
        };
        if declared.status == "deprecated" {
            return "该模型已停用。使用 /models 查看可用模型；路由与历史保持不变。".into();
        }
        if current.model == model {
            return format!("Current model: `{model}`");
        }
        let mut next = current.clone();
        next.reasoning_effort = None;
        next.model = model.clone();
        if persist_current_route(ctx, &next).await.is_err() {
            return "模型选项保存失败，路由与历史保持不变。".into();
        }
        *current = next;
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
pub(super) async fn handle_clear_session(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
) -> String {
    if let Err(err) = clear_sender_session_id(ctx, msg).await {
        return format!("项目不可用：{err}");
    }
    clear_sender_history(ctx, sender_key);
    set_sender_task_mode(ctx, sender_key, false);
    "会话已重置. 开始新会话.".to_string()
}

/// 立即创建并持久化新会话，写入失败不替换旧绑定。
pub(super) async fn handle_new_session(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
) -> String {
    let result = async {
        let snapshot = resolve_channel_message_snapshot(ctx, msg).await?;
        let mut route = get_route_selection(ctx, sender_key);
        route.task_mode_enabled = false;
        let context = crate::project::instance::Context {
            directory: snapshot.directory.to_string_lossy().into(),
            worktree: snapshot.project.worktree.clone(),
            project: snapshot.project.clone(),
        };
        crate::session::ui_store::with_tenant_scope(
            snapshot.scope.chat.tenant.clone(),
            crate::project::instance::provide_context(context, async {
                let model = Box::pin(provider::get_model(&route.provider, &route.model))
                    .await
                    .context("当前模型已不可用，请先选择有效模型")?;
                anyhow::ensure!(model.status != "deprecated", "当前模型已停用");
                if let Some(effort) = &route.reasoning_effort {
                    anyhow::ensure!(
                        effort_levels(&model).contains(effort),
                        "当前推理强度已不可用，请先使用 /effort default"
                    );
                }
                persist_current_route(ctx, &get_route_selection(ctx, sender_key)).await?;
                let id = create_channel_session(&snapshot, msg, Some(route.clone())).await?;
                clear_sender_history(ctx, sender_key);
                set_route_selection(ctx, sender_key, route);
                invalidate_archive(&snapshot);
                Ok::<_, anyhow::Error>(format!("已创建新会话 {id}；旧会话已保留。"))
            }),
        )
        .await
    }
    .await;
    result.unwrap_or_else(|err| {
        format!(
            "创建会话失败，原绑定保持不变：{}",
            crate::providers::sanitize_api_error(&err.to_string())
        )
    })
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
    if let Err(err) = clear_sender_session_id(ctx, msg).await {
        return format!("项目不可用：{err}");
    }
    clear_sender_history(ctx, sender_key);
    set_sender_task_mode(ctx, sender_key, true);
    "我进入了任务模式。".to_string()
}

/// 查询真实 provider 目录；不把本地预览缓存当作能力凭据。
pub(super) async fn handle_models(current: &ChannelRouteSelection) -> String {
    let info = match provider::get_provider(&current.provider).await {
        Ok(Some(info)) => info,
        Ok(None) => {
            return "当前 provider 无可用模型目录。请联系维护者配置；使用 /help 查看命令。".into();
        }
        Err(error) => return format!("模型目录不可用：{error}"),
    };
    let mut ids = info
        .models
        .values()
        .filter(|m| m.status != "deprecated")
        .map(|m| m.id.as_str())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    format!(
        "Current provider: `{}`\nCurrent model: `{}`\n可用模型：\n{}\n使用 /model <ID>；/models <provider> 切换 provider。",
        current.provider,
        current.model,
        ids.join("\n")
    )
}

pub(super) fn effort_levels(model: &vw_shared::provider::types::Model) -> Vec<String> {
    if !model.capabilities.reasoning {
        return Vec::new();
    }
    vw_shared::provider::reasoning::reasoning_levels(
        &model.api.id,
        &model.api.adapter,
        &model.provider_id,
        &model.api.url,
        model.capabilities.reasoning_options.as_deref(),
    )
}

/// 校验后只更新当前路由，不清历史；default 清除当前覆盖。
pub(super) async fn handle_effort(
    ctx: &ChannelRuntimeContext,
    key: &str,
    current: &mut ChannelRouteSelection,
    level: &str,
) -> String {
    let level = level.to_ascii_lowercase();
    if level == "default" {
        let mut next = current.clone();
        next.reasoning_effort = None;
        if persist_current_route(ctx, &next).await.is_err() {
            return "模型选项保存失败，路由与历史保持不变。".into();
        }
        *current = next;
        set_route_selection(ctx, key, current.clone());
        return "推理强度已恢复默认。".into();
    }
    let model = match provider::get_model(&current.provider, &current.model).await {
        Ok(model) => model,
        Err(provider::CatalogError::Configuration(error)) => {
            return format!("模型配置读取失败：{error}");
        }
        Err(provider::CatalogError::ModelNotFound(_)) => {
            return "当前模型无能力目录，无法设置推理强度；路由与历史保持不变。".into();
        }
    };
    let levels = effort_levels(&model);
    if level.is_empty() {
        return format!(
            "当前推理强度：{}\n可用强度：{}；default 恢复默认。",
            current.reasoning_effort.as_deref().unwrap_or("default"),
            levels.join(", ")
        );
    }
    if !levels.contains(&level) {
        return format!(
            "不支持的推理强度；可用：{}。路由与历史保持不变。使用 /effort。",
            levels.join(", ")
        );
    }
    let mut next = current.clone();
    next.reasoning_effort = Some(level.clone());
    if persist_current_route(ctx, &next).await.is_err() {
        return "模型选项保存失败，路由与历史保持不变。".into();
    }
    *current = next;
    set_route_selection(ctx, key, current.clone());
    format!("推理强度已切换为 {level}。")
}

/// 状态查询只读取当前完整消息作用域，不创建会话或枚举其他聊天。
pub(super) async fn handle_status(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    current: &ChannelRouteSelection,
) -> String {
    let snapshot = match resolve_channel_message_snapshot(ctx, msg).await {
        Ok(snapshot) => snapshot,
        Err(err) => return format!("项目不可用：{err}"),
    };
    let session = sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&snapshot.scope)
        .cloned();
    let status = session
        .as_deref()
        .map(crate::session::status::get)
        .unwrap_or(crate::session::status::Info::Idle);
    let status = match status {
        crate::session::status::Info::Idle => "空闲",
        crate::session::status::Info::Busy => "运行中",
        crate::session::status::Info::Retry { .. } => "重试中",
    };
    format!(
        "当前项目：{}\n会话：{}\nProvider：{}\n模型：{}\n推理强度：{}\n运行状态：{}\n任务模式：{}\n设置仅对当前作用域生效；权限由现有策略控制。",
        snapshot.project.id,
        session.as_deref().unwrap_or("未创建"),
        current.provider,
        current.model,
        current.reasoning_effort.as_deref().unwrap_or("default"),
        status,
        if current.task_mode_enabled { "开启" } else { "关闭" }
    )
}
