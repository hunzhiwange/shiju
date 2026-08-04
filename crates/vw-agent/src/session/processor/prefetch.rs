//! 会话预取处理器模块
//!
//! 本模块提供了会话上下文的预取功能，用于在用户查询处理之前主动获取相关信息，
//! 从而提升响应速度和用户体验。
//!
//! ## 主要功能
//!
//! - **权限检查**：验证预取操作是否符合安全策略
//! - **文件预取**：从查询中提取文件路径并预读取文件内容
//! - **最小预取面**：仅对显式文件路径执行预取
//! - **异步桥接**：提供同步上下文中执行异步操作的能力
//!
//! ## 工作流程
//!
//! 1. 解析用户查询，识别可能需要预取的文件路径
//! 2. 检查预取操作是否被权限系统允许
//! 3. 对于文件路径，尝试预读取文件内容
//! 4. 未识别到文件路径时直接结束，不追加搜索工具调用
//!
//! ## 平台兼容性
//!
//! 大部分预取功能在 WASM 目标上被禁用，因为在浏览器环境中无法访问文件系统。

use crate::app::agent::agent;
use crate::app::agent::permission::next as permission_next;
use crate::app::agent::project;
use crate::app::agent::tools;
use crate::app::agent::tools::ToolRuntimeContext;
use std::collections::HashSet;
use std::future::Future;
use std::path::PathBuf;

/// 在同步上下文中执行异步 Future
///
/// 该函数提供了一个桥接机制，允许在同步代码中执行异步操作。
/// 它会尝试复用当前 Tokio 运行时（如果存在），否则创建一个新的运行时。
///
/// # 平台兼容性
///
/// - **非 WASM 目标**：正常工作，支持复用现有运行时或创建新运行时
/// - **WASM 目标**：会触发 panic，因为 WASM 不支持此操作
///
/// # 参数
///
/// - `fut`：要执行的异步 Future
///
/// # 返回值
///
/// 返回 Future 的输出结果
///
/// # 示例
///
/// ```ignore
/// let result = block_on(async {
///     some_async_operation().await
/// });
/// ```
///
/// # 实现细节
///
/// 1. 首先尝试获取当前 Tokio 运行时的句柄
/// 2. 如果存在当前运行时，使用 `block_in_place` 在原地阻塞执行
/// 3. 如果不存在运行时，创建一个新的单线程运行时执行
pub(crate) fn block_on<F: Future>(fut: F) -> F::Output {
    // WASM 平台不支持此操作，直接 panic
    #[cfg(target_arch = "wasm32")]
    panic!("block_on not supported on WASM");

    // 非 WASM 平台的实现
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 尝试获取当前 Tokio 运行时的句柄
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // 存在当前运行时，使用 block_in_place 在多线程环境中安全阻塞
            return tokio::task::block_in_place(|| handle.block_on(fut));
        }
        // 不存在当前运行时，创建新的单线程运行时执行 Future
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("创建 tokio 运行时")
            .block_on(fut)
    }
}

/// 检查预取操作是否被权限系统允许
///
/// 该函数通过权限系统验证指定的预取操作是否可以执行。
/// 它会获取当前代理的权限规则集，并使用权限评估引擎进行判断。
///
/// # 平台兼容性
///
/// - **非 WASM 目标**：执行权限检查并返回结果
/// - **WASM 目标**：始终返回 `false`，禁止所有预取操作
///
/// # 参数
///
/// - `permission`：要检查的权限类型（如 "read"）
/// - `target`：操作目标（如文件路径或搜索查询）
///
/// # 返回值
///
/// - `true`：权限评估结果为允许（Allow）
/// - `false`：权限评估结果为拒绝或平台不支持
///
/// # 实现细节
///
/// 1. 获取默认代理名称
/// 2. 获取代理信息及其权限规则集
/// 3. 如果代理信息不存在，尝试获取 "build" 代理作为备选
/// 4. 使用权限评估引擎评估操作是否被允许
pub(crate) fn prefetch_allowed(permission: &str, target: &str) -> bool {
    // WASM 平台禁止所有预取操作
    #[cfg(target_arch = "wasm32")]
    return false;

    // 非 WASM 平台的权限检查实现
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 获取默认代理名称，失败时使用 "build" 作为备选
        let agent_name = block_on(agent::default_agent()).unwrap_or_else(|_| "main".to_string());
        let ruleset = block_on(agent::permission_rules(&agent_name))
            .or_else(|| block_on(agent::permission_rules("main")))
            .unwrap_or_default();

        // 评估权限并检查是否为允许操作
        permission_next::evaluate(permission, target, &[ruleset]).action
            == permission_next::Action::Allow
    }
}

/// 从项目根目录获取应用会话范围标识符
///
/// 该函数根据提供的项目根目录路径，获取项目信息并返回项目的唯一标识符。
/// 项目标识符用于会话范围隔离和上下文管理。
///
/// # 平台兼容性
///
/// - **非 WASM 目标**：正常工作，从文件系统读取项目信息
/// - **WASM 目标**：始终返回 `None`
///
/// # 参数
///
/// - `root`：项目根目录路径的可选引用
///
/// # 返回值
///
/// - `Some(String)`：成功获取到项目标识符
/// - `None`：路径为空、项目信息获取失败或平台不支持
///
/// # 实现细节
///
/// 1. 验证根目录路径非空
/// 2. 从目录读取项目信息
/// 3. 返回项目的唯一标识符
pub(crate) async fn app_session_scope_from_root_async(root: Option<&str>) -> Option<String> {
    // WASM 平台不支持文件系统访问
    #[cfg(target_arch = "wasm32")]
    return None;

    // 非 WASM 平台的实现
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 验证根目录路径：去除首尾空白并确保非空
        let root = root.map(str::trim).filter(|s| !s.is_empty())?;

        // 转换为路径对象
        let root_path = PathBuf::from(root);
        if !root_path.is_dir() {
            return None;
        }

        let tenant = project::default_tenant_scope();
        project::from_directory(&tenant, &root_path).await.ok().map(|(info, _)| info.id)
    }
}

/// 从项目根目录获取应用会话范围标识符
///
/// 与 `app_session_scope_from_root_async` 的行为一致，但提供同步入口，供
/// 当前仍为同步上下文的调用方复用。调用方应优先在异步上下文提前计算并
/// 透传到会话处理器，避免在 Tokio runtime worker 中创建/阻塞 runtime。
pub(crate) fn app_session_scope_from_root(root: Option<&str>) -> Option<String> {
    // WASM 平台不支持文件系统访问
    #[cfg(target_arch = "wasm32")]
    return None;

    // 非 WASM 平台的实现
    #[cfg(not(target_arch = "wasm32"))]
    {
        block_on(app_session_scope_from_root_async(root))
    }
}

/// 预取种子上下文以优化查询响应
///
/// 该函数是预取处理的核心入口，分析用户查询并主动获取相关上下文信息。
/// 它会识别查询中提到的文件路径，并在权限允许的情况下预读取这些文件。
/// 当前仅对文件路径执行预取，不再主动触发额外搜索工具。
///
/// # 参数
///
/// - `session`：可变引用的会话对象，用于记录预取的上下文
/// - `query`：用户的查询文本
/// - `ctx`：工具执行上下文，包含根目录、工作目录等信息
/// - `allowed_tools`：允许使用的工具集合
/// - `tool_state`：工具会话状态的可变引用，用于跟踪工具执行
///
/// # 行为说明
///
/// 1. **工具检测**：如果查询中包含工具调用语法，则跳过预取
/// 2. **文件路径提取**：使用正则表达式识别查询中的文件路径
/// 3. **文件预取**：对于有效的文件路径，尝试预读取文件内容
/// 4. **停止扩张**：如果没有识别到可预取文件，则直接结束
///
/// # 预取限制
///
/// - 最多预取 3 个文件路径
/// - 仅预取非二进制文件
/// - 所有预取操作都需要通过权限检查
pub(crate) fn prefetch_seed_context(
    session: &mut super::Session,
    query: &str,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    tool_state: &mut super::ToolSessionState,
) {
    // 将查询按行分割，用于工具语法检测
    let lines: Vec<&str> = query.lines().collect();

    // 如果查询中包含工具调用语法，则跳过预取
    // 避免与用户显式的工具调用冲突
    if lines
        .iter()
        .enumerate()
        .any(|(i, _)| super::utils::parse_tool_at(&lines, i, allowed_tools).is_some())
    {
        return;
    }

    // 初始化候选文件路径列表和去重集合
    let mut candidates: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // 特殊处理：style.css 是常见的前端样式文件，优先检测
    if query.to_lowercase().contains("style.css") {
        if seen.insert("style.css".to_string()) {
            candidates.push("style.css".to_string());
        }
    }

    // 构建文件路径匹配的正则表达式
    // 匹配模式：
    // 1. 以 / 开头的路径（Unix 风格绝对路径）
    // 2. 包含扩展名的相对路径（至少 3 个字符，扩展名 1-8 个字符）
    let re = regex::Regex::new(
        r#"(?i)(/[^ \t\r\n"'<>]+?\.[a-z0-9]{1,8}|[A-Za-z0-9_./-]{3,}?\.[A-Za-z0-9]{1,8})"#,
    )
    .unwrap();

    // 遍历所有匹配的文件路径
    for cap in re.captures_iter(query) {
        if let Some(m) = cap.get(1) {
            let s = m.as_str().trim();

            // 跳过 HTTP/HTTPS URL，只处理本地文件路径
            if s.starts_with("http://") || s.starts_with("https://") {
                continue;
            }

            // 去重并添加到候选列表
            if seen.insert(s.to_string()) {
                candidates.push(s.to_string());
            }
        }

        // 限制候选数量为 3 个，避免过度预取
        if candidates.len() >= 3 {
            break;
        }
    }

    // 尝试预取每个候选文件
    for p in candidates {
        let _ = try_prefetch_read(session, &p, ctx, tool_state);
    }
}

/// 尝试预读取指定文件
///
/// 该函数检查文件是否存在、是否为普通文件、权限是否允许，
/// 然后使用 read 工具预读取文件内容到会话上下文中。
///
/// # 参数
///
/// - `session`：可变引用的会话对象
/// - `p`：要预取的文件路径（可能是相对路径或绝对路径）
/// - `ctx`：工具执行上下文
/// - `tool_state`：工具会话状态的可变引用
///
/// # 返回值
///
/// - `true`：成功发起预读取操作
/// - `false`：文件不存在、权限拒绝或文件为二进制格式
///
/// # 实现细节
///
/// 1. 解析文件的完整路径
/// 2. 验证文件存在且为普通文件（非目录）
/// 3. 检查读取权限
/// 4. 检测二进制文件并跳过
/// 5. 计算相对路径（用于工具调用）
/// 6. 构建 read 工具输入并执行
fn try_prefetch_read(
    session: &mut super::Session,
    p: &str,
    ctx: &ToolRuntimeContext,
    tool_state: &mut super::ToolSessionState,
) -> bool {
    // 解析文件的完整路径
    let full = super::utils::resolve_full_path(ctx, p);

    // 验证文件存在且为普通文件
    if !full.exists() || !full.is_file() {
        return false;
    }

    // 检查读取权限
    if !prefetch_allowed("read", &full.to_string_lossy()) {
        return false;
    }

    // 检测二进制文件并跳过（二进制文件不适合作为文本上下文）
    if tools::is_binary(&full) {
        return false;
    }

    // 计算相对于项目根目录的路径
    // 这样工具调用时使用相对路径，更加用户友好
    let rel = ctx
        .root
        .as_deref()
        .and_then(|root| full.strip_prefix(root).ok())
        .map(|pp| pp.to_string_lossy().to_string().replace('\\', "/"))
        .unwrap_or_else(|| p.to_string());

    // 构建 file_read 工具的输入参数
    let input = serde_json::json!({
        "filePath": rel,
        "offset": 0,           // 从文件开头开始读取
        "limit": 200,          // 限制读取行数，避免过大
        "output_mode": "content"  // 输出模式为内容
    })
    .to_string();

    // 创建空的事件处理器
    let mut noop = |_ev: super::StreamEvent| true;

    // 执行 file_read 工具并记录结果到会话上下文
    let _ = super::tools_exec::run_tool_and_record(
        session,
        "file_read",
        &input,
        ctx,
        false,
        &mut noop,
        tool_state,
    );

    true
}
#[cfg(test)]
#[path = "prefetch_tests.rs"]
mod prefetch_tests;
