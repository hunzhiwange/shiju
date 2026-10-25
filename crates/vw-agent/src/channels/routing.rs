//! 通道路由选择模块
//!
//! 本模块负责管理通道中消息路由选择的核心逻辑，包括：
//! - Provider（模型提供者）的发现与别名解析
//! - 路由选择的获取、设置与默认值计算
//! - 基于消息分类的智能路由决策
//! - 模型缓存加载与帮助信息生成
//!
//! ## 主要组件
//!
//! - [`available_provider_ids`] - 获取当前可用的 Provider ID 列表
//! - [`resolve_provider_alias`] - 解析 Provider 别名为规范 ID
//! - [`get_route_selection`] - 获取指定发送者的路由选择
//! - [`set_route_selection`] - 设置指定发送者的路由选择
//! - [`classify_message_route`] - 根据消息内容分类并返回路由选择
//! - [`build_models_help_response`] - 构建模型帮助响应文本

use super::*;

/// 获取当前可用的 Provider ID 列表（非 WASM 平台）
///
/// 在非 WASM 目标平台上，此函数会异步查询 provider 模块获取所有已注册的
/// 模型提供者，并返回按字母顺序排序的 ID 列表。
///
/// # 返回值
///
/// 返回已排序的 Provider ID 字符串向量
///
/// # 平台说明
///
/// 此版本仅在非 WASM 平台编译，WASM 平台使用 [`available_provider_ids`]
/// 的另一个无操作版本。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn available_provider_ids() -> Vec<String> {
    // 异步阻塞调用 provider 模块获取所有已注册的提供者
    let providers = block_on_future(provider::list());
    // 提取所有键（Provider ID）并收集为向量
    let mut ids = providers.into_keys().collect::<Vec<_>>();
    // 按字母顺序排序，确保返回结果的一致性和可预测性
    ids.sort();
    ids
}

/// 获取当前可用的 Provider ID 列表（WASM 平台）
///
/// 在 WASM 目标平台上，由于运行时限制，Provider 发现功能不可用，
/// 此函数返回空向量。
///
/// # 返回值
///
/// 始终返回空的 `Vec<String>`
///
/// # 平台说明
///
/// 此版本仅在 WASM 平台编译，非 WASM 平台使用功能完整的
/// [`available_provider_ids`] 版本。
#[cfg(target_arch = "wasm32")]
pub(crate) fn available_provider_ids() -> Vec<String> {
    Vec::new()
}

/// 解析 Provider 别名为规范的 Provider ID
///
/// 此函数将用户输入的 Provider 名称（可能是别名或不区分大小写的变体）
/// 解析为系统中注册的标准 Provider ID。解析过程会忽略大小写差异。
///
/// # 参数
///
/// * `name` - 用户输入的 Provider 名称或别名
///
/// # 返回值
///
/// - `Some(String)` - 找到匹配的规范 Provider ID
/// - `None` - 输入为空或未找到匹配的 Provider
///
/// # 示例
///
/// ```ignore
/// // 如果系统中注册了 "OpenAI" 提供者
/// let id = resolve_provider_alias("openai");  // 返回 Some("OpenAI")
/// let id = resolve_provider_alias("OPENAI");  // 返回 Some("OpenAI")
/// let id = resolve_provider_alias("");        // 返回 None
/// let id = resolve_provider_alias("unknown"); // 返回 None
/// ```
///
/// # 实现细节
///
/// 1. 去除输入字符串两端的空白字符
/// 2. 检查处理后的字符串是否为空
/// 3. 遍历所有可用的 Provider ID，执行不区分大小写的匹配
/// 4. 返回第一个匹配的规范 ID（保持原始大小写）
pub(crate) fn resolve_provider_alias(name: &str) -> Option<String> {
    // 去除输入名称的首尾空白字符
    let candidate = name.trim();
    // 空字符串直接返回 None，避免无意义的匹配
    if candidate.is_empty() {
        return None;
    }

    // 遍历所有可用的 Provider ID 进行不区分大小写的匹配
    for id in available_provider_ids() {
        if id.eq_ignore_ascii_case(candidate) {
            // 返回规范化的 Provider ID（保持原始注册时的大小写）
            return Some(id);
        }
    }

    // 未找到匹配项
    None
}

/// 构建默认的路由选择配置
///
/// 根据通道运行时上下文中的默认配置，创建一个标准的路由选择对象。
/// 此函数用于在没有用户自定义路由时的回退场景。
///
/// # 参数
///
/// * `ctx` - 通道运行时上下文，包含默认配置信息
///
/// # 返回值
///
/// 返回一个 `ChannelRouteSelection` 实例，包含：
/// - 从运行时默认配置中提取的 provider
/// - 从运行时默认配置中提取的 model
/// - `task_mode_enabled` 固定为 `false`
///
/// # 用途
///
/// - 初始化新发送者的路由状态
/// - 当用户切换回默认配置时作为比较基准
/// - 清除自定义路由时的回退值
pub(crate) fn default_route_selection(ctx: &ChannelRuntimeContext) -> ChannelRouteSelection {
    // 从运行时上下文获取默认配置快照
    let defaults = runtime_defaults_snapshot(ctx);
    // 构建默认路由选择，任务模式默认禁用
    ChannelRouteSelection {
        provider: defaults.default_provider,
        model: defaults.model,
        reasoning_effort: None,
        task_mode_enabled: false,
    }
}

/// 获取指定发送者的路由选择配置
///
/// 查询路由覆盖映射表，获取指定发送者的自定义路由选择。
/// 如果该发送者没有自定义配置，则返回默认路由选择。
///
/// # 参数
///
/// * `ctx` - 通道运行时上下文，包含路由覆盖映射表
/// * `sender_key` - 发送者的唯一标识键（通常由通道类型和用户 ID 组成）
///
/// # 返回值
///
/// 返回 `ChannelRouteSelection` 实例，优先返回用户自定义配置，
/// 无自定义配置时返回默认配置。
///
/// # 线程安全
///
/// 此函数通过 Mutex 访问共享的路由覆盖映射表。如果 Mutex 被污染
/// （持有线程 panic），会自动恢复并继续操作，确保系统稳定性。
///
/// # 示例
///
/// ```ignore
/// let selection = get_route_selection(&ctx, "telegram:user123");
/// println!("Using provider: {}, model: {}", selection.provider, selection.model);
/// ```
pub(crate) fn get_route_selection(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
) -> ChannelRouteSelection {
    ctx.route_overrides
        // 获取 Mutex 锁，如果被污染则恢复并继续
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        // 查找指定发送者的自定义路由
        .get(sender_key)
        .cloned()
        // 如果没有自定义配置，使用默认路由
        .unwrap_or_else(|| default_route_selection(ctx))
}

/// 根据消息内容分类并返回智能路由选择
///
/// 使用消息分类器分析用户消息的内容特征，根据匹配的规则
/// 自动选择最合适的 Provider 和模型。此功能支持基于消息
/// 复杂度、类型等特征的路由优化。
///
/// # 参数
///
/// * `ctx` - 通道运行时上下文，包含分类器配置和模型路由规则
/// * `message` - 待分类的用户消息内容
///
/// # 返回值
///
/// - `Some(ChannelRouteSelection)` - 分类成功且找到匹配的路由规则
/// - `None` - 分类功能禁用、无匹配规则或消息无法分类
///
/// # 日志输出
///
/// 成功分类时，会在 `query_classification` 目标下记录 INFO 级别日志，
/// 包含以下字段：
/// - `hint` - 分类结果的提示标识
/// - `model` - 选定的模型 ID
/// - `rule_priority` - 匹配规则的优先级
/// - `message_length` - 原始消息长度
///
/// # 工作流程
///
/// 1. 调用分类器对消息进行分类决策
/// 2. 如果分类器返回决策，查找匹配的模型路由规则
/// 3. 找到匹配规则后，记录分类日志并返回路由选择
/// 4. 任何步骤失败（分类禁用、无匹配规则）均返回 None
pub(crate) fn classify_message_route(
    ctx: &ChannelRuntimeContext,
    message: &str,
) -> Option<ChannelRouteSelection> {
    // 使用分类器对消息进行分类，获取决策结果
    // 如果分类功能禁用或消息不匹配任何规则，此处返回 None
    let decision = crate::app::agent::agent::classifier::classify_with_decision(
        &ctx.query_classification,
        message,
    )?;

    // 在模型路由规则列表中查找与分类提示匹配的路由
    // 使用分类决策的 hint 字段作为匹配键
    let route = ctx.model_routes.iter().find(|r| r.hint == decision.hint)?;

    // 记录分类决策日志，便于调试和监控
    tracing::info!(
        target: "query_classification",
        hint = %decision.hint,
        model = %route.model,
        rule_priority = decision.priority,
        message_length = message.len(),
        "Classified message route"
    );

    // 构建并返回路由选择，任务模式禁用
    Some(ChannelRouteSelection {
        provider: route.provider.clone(),
        model: route.model.clone(),
        reasoning_effort: None,
        task_mode_enabled: false,
    })
}

/// 设置指定发送者的路由选择配置
///
/// 为指定发送者设置自定义路由选择。如果新配置与默认配置相同，
/// 则会移除该发送者的自定义配置，恢复使用默认值。
///
/// # 参数
///
/// * `ctx` - 通道运行时上下文，包含路由覆盖映射表
/// * `sender_key` - 发送者的唯一标识键
/// * `next` - 新的路由选择配置
///
/// # 行为说明
///
/// - 如果 `next` 与默认路由相同：从映射表中移除该发送者的条目
/// - 如果 `next` 与默认路由不同：在映射表中插入或更新该发送者的配置
///
/// # 线程安全
///
/// 此函数通过 Mutex 访问共享的路由覆盖映射表。如果 Mutex 被污染，
/// 会自动恢复并继续操作。
///
/// # 设计理念
///
/// 当用户切换回默认配置时移除条目而非存储相同值，可以：
/// - 减少内存占用
/// - 保持映射表清洁
/// - 使默认配置变更能自动生效
pub(crate) fn set_route_selection(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
    next: ChannelRouteSelection,
) {
    // 获取默认路由作为比较基准
    let default_route = default_route_selection(ctx);
    // 获取 Mutex 锁，处理可能的污染情况
    let mut routes = ctx.route_overrides.lock().unwrap_or_else(|e| e.into_inner());
    // 如果新配置等于默认配置，移除自定义条目（恢复默认）
    if next == default_route {
        routes.remove(sender_key);
    } else {
        // 否则，存储新的自定义配置
        routes.insert(sender_key.to_string(), next);
    }
}

/// 从缓存文件加载指定 Provider 的模型列表预览
///
/// 从工作目录的状态子目录中读取模型缓存文件，提取指定 Provider
/// 的模型 ID 列表。返回的列表受预览数量限制，避免响应过长。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间目录路径
/// * `provider_name` - 目标 Provider 的名称
///
/// # 返回值
///
/// 返回指定 Provider 的模型 ID 列表（最多 `MODEL_CACHE_PREVIEW_LIMIT` 个）。
/// 如果缓存文件不存在、格式错误或未找到该 Provider，返回空向量。
///
/// # 文件路径
///
/// 缓存文件位置：`{workspace_dir}/state/{MODEL_CACHE_FILE}`
///
/// # 错误处理
///
/// 此函数采用容错设计，任何读取或解析错误都会返回空向量而非报错，
/// 确保缓存不可用时系统仍能正常运行。
///
/// # 性能考虑
///
/// - 使用 `take()` 限制返回数量，避免大量数据的内存分配
/// - 文件读取失败时快速返回，无额外开销
pub(crate) fn load_cached_model_preview(workspace_dir: &Path, provider_name: &str) -> Vec<String> {
    // 构建缓存文件的完整路径
    let cache_path = workspace_dir.join("state").join(MODEL_CACHE_FILE);
    // 尝试读取缓存文件内容，失败则返回空向量
    let Ok(raw) = std::fs::read_to_string(cache_path) else {
        return Vec::new();
    };
    // 尝试解析 JSON 格式的缓存状态，失败则返回空向量
    let Ok(state) = serde_json::from_str::<ModelCacheState>(&raw) else {
        return Vec::new();
    };

    // 在缓存条目中查找指定的 Provider
    state
        .entries
        .into_iter()
        .find(|entry| entry.provider == provider_name)
        // 提取模型列表，限制预览数量
        .map(|entry| entry.models.into_iter().take(MODEL_CACHE_PREVIEW_LIMIT).collect::<Vec<_>>())
        // 未找到 Provider 时返回空向量
        .unwrap_or_default()
}

/// 构建模型帮助命令的响应文本
///
/// 生成包含当前路由状态、可用命令说明和模型列表的帮助信息。
/// 此响应通常用于 `/models` 命令的回复。
///
/// # 参数
///
/// * `current` - 当前的路由选择配置
/// * `workspace_dir` - 工作空间目录路径，用于加载模型缓存
///
/// # 返回值
///
/// 返回格式化的帮助文本字符串，包含以下内容：
///
/// ## 基础信息
/// - 当前使用的 Provider 和 Model
///
/// ## 命令说明
/// - `/model <model-id>` - 切换模型
/// - `/approve-request <tool-name>` - 请求受监督工具的授权
/// - `/approve-all-once` - 请求一次性全工具授权
/// - `/approve-confirm <request-id>` - 确认授权
/// - `/approve-deny <request-id>` - 拒绝授权
/// - `/approve-pending` - 列出待处理请求
/// - `/approve <tool-name>` - 直接授权受监督工具
/// - `/unapprove <tool-name>` - 撤销授权
/// - `/approvals` - 列出授权状态
///
/// ## 自然语言授权
/// - `direct` 模式：立即授权
/// - `request_confirm` 模式：请求后确认授权
///
/// ## 模型列表
/// - 如果有缓存：显示预览数量的模型 ID
/// - 如果无缓存：提示运行刷新命令
///
/// # 使用场景
///
/// 当用户执行 `/models` 命令或请求查看可用模型时调用。
pub(crate) fn build_models_help_response(
    current: &ChannelRouteSelection,
    workspace_dir: &Path,
) -> String {
    let mut response = String::new();
    // 添加当前路由状态信息
    let _ = writeln!(
        response,
        "Current provider: `{}`\nCurrent model: `{}`",
        current.provider, current.model
    );
    // 添加模型切换命令说明
    response.push_str("\nSwitch model with `/model <model-id>`.\n");
    // 添加授权相关命令说明
    response.push_str("Request supervised tool approval with `/approve-request <tool-name>`.\n");
    response.push_str("Request one-time all-tools approval with `/approve-all-once`.\n");
    response.push_str("Confirm approval with `/approve-confirm <request-id>`.\n");
    response.push_str("Deny approval with `/approve-deny <request-id>`.\n");
    response.push_str("List pending requests with `/approve-pending`.\n");
    response.push_str("Approve supervised tools with `/approve <tool-name>`.\n");
    response.push_str("Revoke approval with `/unapprove <tool-name>`.\n");
    response.push_str("List approval state with `/approvals`.\n");
    // 添加自然语言授权模式的说明
    response.push_str(
        "Natural language also works (policy controlled).\n\
         - `direct` mode (default): `授权工具 shell` grants immediately.\n\
         - `request_confirm` mode: `授权工具 shell` then `确认授权 apr-xxxxxx`.\n",
    );

    // 尝试加载并显示缓存的模型列表
    let cached_models = load_cached_model_preview(workspace_dir, &current.provider);
    if cached_models.is_empty() {
        // 无缓存时，提示用户运行刷新命令
        let _ = writeln!(
            response,
            "\nNo cached model list found for `{}`. Ask the operator to run `vibewindow models refresh --provider {}`.",
            current.provider, current.provider
        );
    } else {
        // 有缓存时，显示模型预览列表
        let _ = writeln!(response, "\nCached model IDs (top {}):", cached_models.len());
        for model in cached_models {
            let _ = writeln!(response, "- `{model}`");
        }
    }

    response
}

#[cfg(test)]
#[path = "routing_tests.rs"]
mod routing_tests;
