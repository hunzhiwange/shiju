//! 会话提示词构建模块
//!
//! 本模块负责构建和管理代理会话中使用的系统提示词（system prompt）。
//! 系统提示词由环境信息、系统指令和约束条件等部分组成，用于引导模型行为。
//!
//! # 主要功能
//!
//! - 构建完整的系统提示词
//! - 收集环境配置信息
//! - 加载系统指令内容
//! - 提供最大步骤约束文本

/// 最大步骤约束文本
///
/// 从外部文件加载的步骤限制说明，用于约束代理执行任务时的最大步骤数。
/// 该文本将被包含在系统提示词中，以限制代理的行为范围。
const MAX_STEPS: &str = include_str!("prompt/max-steps.txt");

/// 获取环境信息
///
/// 从项目引用中提取环境配置信息，包括模型配置和项目根目录等。
/// 该函数会异步获取环境数据，然后阻塞等待结果。
///
/// # 参数
///
/// - `model`: 可选的模型标识符，用于指定使用的 AI 模型
/// - `root`: 可选的项目根目录路径
///
/// # 返回
///
/// 返回环境信息字符串向量，每个元素代表一条环境配置项
fn environment(model: Option<&str>, root: Option<&str>) -> Vec<String> {
    vec![block_on(super::system::environment_from_ref(model, root))]
}

/// 阻塞执行异步操作的辅助函数
///
/// 在同步上下文中执行异步 Future，根据运行环境自动选择合适的执行方式。
/// 对于 WebAssembly 目标，该函数会直接 panic（不支持阻塞操作）。
/// 对于原生目标，会尝试使用当前的 Tokio 运行时，或创建新的运行时。
///
/// # 类型参数
///
/// - `T`: Future 的输出类型
///
/// # 参数
///
/// - `fut`: 需要执行的异步 Future
///
/// # 返回
///
/// 返回 Future 的执行结果
///
/// # Panics
///
/// - 在 WebAssembly 环境中调用时会 panic
/// - 创建 Tokio 运行时失败时会 panic
fn block_on<T>(fut: impl std::future::Future<Output = T>) -> T {
    #[cfg(target_arch = "wasm32")]
    panic!("Cannot block_on in wasm async context");

    #[cfg(not(target_arch = "wasm32"))]
    match tokio::runtime::Handle::try_current() {
        // 如果已在 Tokio 运行时中，直接使用当前运行时阻塞执行
        Ok(h) => h.block_on(fut),
        // 否则创建新的单线程运行时执行
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
            .block_on(fut),
    }
}

/// 获取系统指令
///
/// 异步获取系统级指令内容，这些指令定义了代理的基本行为准则和能力边界。
/// 该函数会阻塞等待异步指令加载完成。
///
/// # 返回
///
/// 返回系统指令字符串向量，每个元素代表一条独立的指令
fn instruction_system() -> Vec<String> {
    block_on(super::instruction::system())
}

/// 获取最大步骤约束文本
///
/// 返回用于限制代理执行步骤数的约束文本。
/// 该文本通常包含步骤计数的规则和限制说明。
///
/// # 返回
///
/// 返回去除首尾空白后的最大步骤约束文本的静态引用
///
/// # 示例
///
/// ```ignore
/// let steps_text = max_steps_text();
/// println!("步骤约束: {}", steps_text);
/// ```
pub fn max_steps_text() -> &'static str {
    MAX_STEPS.trim()
}

/// 过滤空白片段并按系统提示词的段落格式拼接。
fn join_non_empty_parts(parts: Vec<String>) -> String {
    parts.into_iter().filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join("\n\n")
}

/// 构建完整的系统提示词
///
/// 将环境信息、系统指令等多个部分组合成完整的系统提示词字符串。
/// 各部分之间用双换行符分隔，自动过滤掉空白内容。
///
/// # 参数
///
/// - `model`: 可选的模型标识符，用于指定目标 AI 模型
/// - `root`: 可选的项目根目录路径，用于解析相对路径引用
///
/// # 返回
///
/// 返回完整的系统提示词字符串，由环境信息和系统指令组合而成
///
/// # 示例
///
/// ```ignore
/// let prompt = system(Some("gpt-4"), Some("/path/to/project"));
/// println!("系统提示词:\n{}", prompt);
/// ```
pub fn system(model: Option<&str>, root: Option<&str>) -> String {
    let mut parts = Vec::<String>::new();

    // 添加环境配置信息
    parts.extend(environment(model, root));

    // 添加系统指令
    parts.extend(instruction_system());

    // 过滤空白内容并用双换行符连接各部分
    join_non_empty_parts(parts)
}
#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
