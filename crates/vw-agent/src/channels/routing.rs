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
pub(crate) fn available_provider_ids() -> Result<Vec<String>, provider::CatalogError> {
    block_on_future(channel_support::routing::available_provider_ids())
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
pub(crate) fn available_provider_ids() -> Result<Vec<String>, provider::CatalogError> { Ok(Vec::new()) }

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
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve_provider_alias(name: &str) -> Result<Option<String>, provider::CatalogError> {
    block_on_future(channel_support::routing::resolve_provider_alias(name))
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
    channel_support::routing::default_route_selection(runtime_defaults_snapshot(ctx))
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
pub(crate) use channel_support::routing::get_route_selection;

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
pub(crate) fn classify_message_route(ctx: &ChannelRuntimeContext, message: &str) -> Option<ChannelRouteSelection> {
    channel_support::routing::classify_message_route(&ctx.query_classification, &ctx.model_routes, message)
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
pub(crate) fn set_route_selection(ctx: &ChannelRuntimeContext, sender_key: &str, next: ChannelRouteSelection) {
    channel_support::routing::set_route_selection(&ctx.route_overrides, default_route_selection(ctx), sender_key, next)
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
pub(crate) use channel_support::routing::load_cached_model_preview;

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
pub(crate) use channel_support::routing::build_models_help_response;

#[cfg(test)]
#[path = "routing_tests.rs"]
mod routing_tests;

#[cfg(target_arch = "wasm32")]
pub(crate) fn resolve_provider_alias(_name: &str) -> Result<Option<String>, provider::CatalogError> { Ok(None) }
